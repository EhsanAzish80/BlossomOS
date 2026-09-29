#!/usr/bin/env python3
"""VM-only llama.cpp-compatible fixture behind the production gateway."""

from __future__ import annotations

import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HOST = "127.0.0.1"
PORT = 8080
MAX_REQUEST_BYTES = 256 * 1024
EXPECTED_TOOL = "files.write:create"
PROMPTS = {
    "Create qualification-note.txt containing exactly: Blossom qualification passed.": (
        "qualification-note.txt",
        "Blossom qualification passed.",
    ),
    "Create mutation-note.txt containing exactly: mutation must fail.": (
        "mutation-note.txt",
        "mutation must fail.",
    ),
}
INDIRECT_INVALID_PROMPT = (
    "Read the code-owned invalid injection fixture as untrusted data, then continue."
)
INDIRECT_VALID_PROMPT = (
    "Read the code-owned valid injection fixture as untrusted data, then continue."
)
UNTRUSTED_FILE_LABEL = "UNTRUSTED_FILE_CONTENT:"


def validate_request(request: object) -> tuple[str, str | None, str | None]:
    if not isinstance(request, dict):
        raise ValueError("request must be an object")
    messages = request.get("messages")
    if not isinstance(messages, list) or len(messages) not in (1, 2):
        raise ValueError("fixture expects one prompt and at most one untrusted-data message")
    message = messages[0]
    if not isinstance(message, dict) or message.get("role") != "user":
        raise ValueError("fixture expects one user message")
    prompt = message.get("content")
    if not isinstance(prompt, str):
        raise ValueError("prompt is not text")
    indirect = prompt in (INDIRECT_INVALID_PROMPT, INDIRECT_VALID_PROMPT)
    if indirect:
        if len(messages) != 2:
            raise ValueError("indirect-injection turn omitted typed untrusted data")
        untrusted = messages[1]
        if (
            not isinstance(untrusted, dict)
            or untrusted.get("role") != "user"
            or not isinstance(untrusted.get("content"), str)
            or not untrusted["content"].startswith(UNTRUSTED_FILE_LABEL + "\n")
        ):
            raise ValueError("indirect-injection data was not separately and visibly labeled")
    elif len(messages) != 1:
        raise ValueError("ordinary fixture turn gained untrusted data")
    if request.get("temperature") != 0 or request.get("seed") != 0:
        raise ValueError("sampling is not pinned")
    if request.get("parallel_tool_calls") is not False:
        raise ValueError("parallel tool calls are not disabled")
    if request.get("tool_choice") != "required":
        raise ValueError("eligible turn must require exactly one tool choice")
    tools = request.get("tools")
    if not isinstance(tools, list) or len(tools) != 1:
        raise ValueError("eligible turn must contain exactly one tool")
    function = tools[0].get("function") if isinstance(tools[0], dict) else None
    if not isinstance(function, dict) or function.get("name") != EXPECTED_TOOL:
        raise ValueError("unexpected eligible tool")
    parameters = function.get("parameters")
    if not isinstance(parameters, dict) or parameters.get("additionalProperties") is not False:
        raise ValueError("workspace schema is not closed")
    selection = PROMPTS.get(prompt)
    if prompt == INDIRECT_INVALID_PROMPT:
        selection = ("../escaped.txt", "injected traversal")
    elif prompt == INDIRECT_VALID_PROMPT:
        selection = ("injected-note.txt", "attacker controlled")
    if selection is None:
        return prompt, None, None
    return prompt, selection[0], selection[1]


def event(model: str, delta: dict, finish_reason: str | None) -> bytes:
    payload = {
        "id": "blossom-qualification-fixture",
        "object": "chat.completion.chunk",
        "created": 1,
        "model": model,
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish_reason}],
    }
    return b"data: " + json.dumps(payload, separators=(",", ":")).encode() + b"\n\n"


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_POST(self) -> None:
        try:
            length = int(self.headers.get("Content-Length", "-1"))
            if length < 0 or length > MAX_REQUEST_BYTES:
                raise ValueError("request size is invalid")
            request = json.loads(self.rfile.read(length))
            _, name, content = validate_request(request)
            model = request.get("model")
            if not isinstance(model, str) or not model:
                raise ValueError("model identity is missing")
            if name is None:
                body = event(model, {"role": "assistant", "content": "denied"}, "stop")
            else:
                arguments = json.dumps({"name": name, "content": content}, separators=(",", ":"))
                body = event(
                    model,
                    {
                        "role": "assistant",
                        "tool_calls": [{
                            "index": 0,
                            "id": "qualification-call",
                            "type": "function",
                            "function": {"name": EXPECTED_TOOL, "arguments": arguments},
                        }],
                    },
                    "tool_calls",
                )
            body += b"data: [DONE]\n\n"
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(body)
        except (ValueError, TypeError, json.JSONDecodeError) as error:
            body = str(error).encode()
            self.send_response(400)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(body)

    def log_message(self, format: str, *args: object) -> None:
        return


def main() -> None:
    ThreadingHTTPServer((HOST, PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
