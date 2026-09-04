#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

const MAX_REQUEST_BYTES: usize = 128 * 1024;
const MODEL: &str = "qwen2.5-0.5b-instruct:q4_k_m";

fn read_request(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "timeout setup failed")?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4 * 1024];
    let header_end = loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|_| "request read failed")?;
        if count == 0 {
            return Err("request ended before headers".into());
        }
        request.extend_from_slice(&buffer[..count]);
        if request.len() > MAX_REQUEST_BYTES {
            return Err("request exceeded fixture bound".into());
        }
        if let Some(position) = request.windows(4).position(|part| part == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let headers = std::str::from_utf8(&request[..header_end])
        .map_err(|_| "request headers were not utf-8")?;
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.strip_prefix("Content-Length: ")
                .and_then(|value| value.parse::<usize>().ok())
        })
        .ok_or("request omitted content length")?;
    if header_end.saturating_add(content_length) > MAX_REQUEST_BYTES {
        return Err("request exceeded fixture bound".into());
    }
    while request.len() - header_end < content_length {
        let count = stream
            .read(&mut buffer)
            .map_err(|_| "request read failed")?;
        if count == 0 {
            return Err("request ended before body".into());
        }
        request.extend_from_slice(&buffer[..count]);
        if request.len() > MAX_REQUEST_BYTES {
            return Err("request exceeded fixture bound".into());
        }
    }
    Ok(())
}

fn serve(mode: &str) -> Result<(), String> {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 8080)).map_err(|_| "fixture bind failed")?;
    let (mut stream, _) = listener.accept().map_err(|_| "fixture accept failed")?;
    read_request(&mut stream)?;
    match mode {
        "headers" => {
            std::thread::sleep(Duration::from_secs(5));
        }
        "completion" => {
            let delta = format!(
                "data: {{\"id\":\"installed-race\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"{MODEL}\",\"choices\":[{{\"index\":0,\"delta\":{{\"role\":\"assistant\",\"content\":\"x\"}},\"finish_reason\":null}}]}}\n\n"
            );
            let terminal = format!(
                "data: {{\"id\":\"installed-race\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"{MODEL}\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n"
            );
            let length = delta.len() + terminal.len();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{delta}"
            )
            .map_err(|_| "fixture response write failed")?;
            stream.flush().map_err(|_| "fixture flush failed")?;
            std::thread::sleep(Duration::from_secs(5));
            let _ = stream.write_all(terminal.as_bytes());
        }
        _ => return Err("expected headers or completion".into()),
    }
    Ok(())
}

fn main() {
    let result = std::env::args()
        .nth(1)
        .ok_or_else(|| "expected headers or completion".into())
        .and_then(|mode: String| serve(&mode));
    if let Err(error) = result {
        eprintln!("installed race fixture failed: {error}");
        std::process::exit(1);
    }
}
