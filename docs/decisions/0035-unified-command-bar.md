# ADR-0035: Unified local command bar

Status: Accepted

## Context

Blossom currently exposes applications, files and the local agent through
separate shell surfaces. That makes the agent feel added to the desktop rather
than part of its interaction model, while an unrestricted chat box would imply
capabilities the system does not have and would send simple local actions to a
model unnecessarily.

The existing security boundary must not move. The shell is an untrusted
presentation client. Trusted code resolves requests, policy decides whether an
effect may proceed, and the broker renders exact approval in a separate window.
The command bar must not become an approval surface or an alternate route to
execution.

## Decision

### Surface and focus

The Blossom flower is the first dock item and opens the command bar. The same
flower at the left edge of the top bar is a second visible entry point.
`Super+Space` opens the same centered overlay and places keyboard focus in its
input. `Escape` closes it without submitting or preserving the text.

Every dock icon has an accessible name and a visible hover label. The first
version includes no decorative trash affordance; a trash item may appear only
if the file manager exposes a real trash location and behavior.

The bar yields focus when the separate broker-owned approval or PolicyKit
authentication window appears. It must not reclaim focus until those surfaces
have finished. Closing the bar never denies, approves or mutates a pending
request.

### Closed routing order

Trusted local routing evaluates input in this order:

1. normalized local application matches derived from installed `.desktop`
   files;
2. normalized file matches within the configured workspace;
3. the existing deterministic `create NAME containing CONTENT` parser; and
4. one explicit `Ask Blossom: ...` choice for all other non-empty input.

Application and workspace-file discovery are local and never reach a model.
Launching an exact application is a direct user action and requires no agent
approval. Opening an exact existing workspace file is also a direct user action
and does not expand model authority. Any operation which changes files uses the
existing trusted resolver, policy and separate exact-effect approval path.

The deterministic create parser keeps its fail-closed contract. If its syntax
matches but its arguments are invalid, the bar reports the validation failure;
it never asks a model to repair or reinterpret the request.

Input which does not resolve locally is not sent immediately. The bar displays
an explicit `Ask Blossom: ...` choice. Activating that choice calls the existing
`StartAgentTurn1` shell interface. Model output remains untrusted and cannot
execute without the existing resolver, policy and approval flow.

The `Ask Blossom` row is always the final result and is never initially
selected when a local result exists. For example, `fire` selects Firefox first
and shows `Ask Blossom: fire` beneath it. The model does not run unless the user
explicitly selects and activates that final row.

Prefix input such as `fire` may produce multiple normalized local matches. The
bar lists them in stable order instead of guessing; an exact result remains a
single local row.

The router does not infer model capability from words in the query. Any
non-empty input which is neither a local result nor a deterministic-create
result receives the explicit `Ask Blossom: ...` row. Only after the user
activates that row may the agent return one of two closed results: a validated
`files.write:create` proposal, or `unsupported`.

`unsupported` is part of the fixed provider output constraint, not free text.
Trusted orchestration maps it to the fixed plain response `Blossom can't do
this yet. Today it can create one file in your workspace.` It produces no
proposal, no approval window and no effect. Malformed, mixed or unknown model
output fails closed and may not be reinterpreted as a proposal.

### Ambiguity and truthful suggestions

The bar never guesses between multiple local matches. It shows a keyboard- and
accessibility-navigable result list and requires the user to choose one.

Empty-state suggestions advertise only capabilities implemented and admitted
by current policy. The first version may suggest opening installed apps,
opening workspace files and create-only workspace publication. It must not
suggest general file organization, transcription, web search or other future
capabilities.

The empty state shows `Super+Space to open · Enter to run · Esc to close`.
Choosing `Create a file` fills the input with `create  containing ` and places
the cursor between `create ` and ` containing ` so the completed request follows
the deterministic parser.

The terminal unsupported state does not show `Enter to run`, because it has no
action.

### Broker-owned routing and activation

Routing and result discovery run in the broker. The shell sends bounded raw
query text and receives display-only rows carrying opaque row identifiers. It
never receives an executable, application path or filesystem authority.
Activating a row sends only its opaque identifier.

The broker keeps rows only for the latest query from that peer. A new query
invalidates every earlier identifier; stale, unknown and cross-peer identifiers
are rejected. Query bytes, query rate, result count and discovery work are
bounded. The shell debounces keystrokes, but broker-side admission does not rely
on that client behavior.

Application results come from parsed `.desktop` entries. System entries sort
before user-installed entries, and the latter are visibly labelled because
`~/.local/share/applications` is writable by same-user processes. The broker
parses the selected entry's `Exec` value, removes desktop field codes according
to the desktop-entry contract, never invokes a shell, and launches the retained
selection in its own systemd user scope.

The first version deliberately supports only regular application entries. It
skips `Hidden` and `NoDisplay` entries, desktop-environment exclusions and a
missing `TryExec`; it refuses terminal and D-Bus-activated launch modes. It uses
the current localized name for display while retaining the plain name for
matching. File size, total entries scanned and returned results are bounded.
The broker caches this snapshot and watches the application directories for a
change instead of rescanning on each keystroke. At activation it rereads the
selected entry and compares its digest with the retained row before launching;
any substitution fails closed.

File discovery is restricted to the configured workspace. Rows display the
workspace-relative path. Activation uses only the retained broker-side
selection; the shell cannot substitute a path.

Deterministic create activation reuses the existing resolver, prepared request,
policy and exact-effect approval path. The command bar adds no constructor or
execution route.

### Privacy

The first version stores no prompt or query history. Closing the overlay clears
its input and results. Audit records retain only the data already required by
the selected action's existing contract; the command bar adds no new prompt
retention.

The usable desktop contains no persistent promotional slogan or duplicate
bottom-corner product label. Those elements may appear in presentation mockups,
but not in the everyday shell.

### Approval separation

Approval never appears inline in the command bar. The broker-owned approval
window continues to display the complete original request, proposal source,
full destination, full bounded content, byte length and every fixed security
field without elision. PolicyKit authentication remains a separate system
surface. Styling may not weaken those requirements.

The command bar and ordinary shell use Petal styling. The approval window does
not inherit theme-controlled presentation. It retains a fixed, broker-owned
security treatment and the non-configurable header `Blossom approval · drawn by
the system broker` with a lock icon. This distinction is a recognizable cue,
not trusted pixels: another same-user application can still draw a lookalike,
as recorded in ADR-0034.

Activating a deterministic create result with Enter only starts the existing
prepared-request flow and opens that separate approval window. The bar contains
no approve or deny control and cannot submit a decision.

All text, including placeholders, provenance labels, keyboard hints and trust
signals, meets at least WCAG 2.1 AA 4.5:1 contrast against its rendered
background. Tests calculate the actual Petal token pairs rather than assuming
semantic token names imply sufficient contrast.

## Alternatives considered

### Put the prompt in the wallpaper

Rejected. Normal windows obscure it, background-layer keyboard focus is
fragile, accessibility is poorer, and model interaction would be mixed into a
surface which is not designed for input.

### Send every query to the model

Rejected. It adds latency and nondeterminism to exact local actions, discloses
more input to the model than necessary and makes local availability depend on
the model runtime.

### Approve inline

Rejected. Combining proposal and authorization would blur the trust boundary
and make it harder for a user to distinguish the agent's output from the
broker's exact-effect preview.

## Qualification requirements

Before acceptance and merge, tests must prove:

1. the routing table sends exact app, file and deterministic-create matches to
   trusted local handlers and records zero model-gateway requests;
2. invalid input which matches deterministic-create syntax fails closed and is
   not offered to the model;
3. ambiguous app or file queries show choices and perform no action until one
   is selected;
4. open-ended input reaches the model only after the explicit `Ask Blossom`
   choice is activated;
5. `Super+Space`, `Escape`, result navigation and activation work from the
   keyboard and are exposed through AT-SPI with meaningful names and roles;
6. the bar yields focus to approval and PolicyKit surfaces and does not steal
   it back while either is active;
7. closing and reopening the bar preserves no prompt history;
8. the existing full-length approval accessibility gate passes unchanged after
   Petal restyling and command-bar integration;
9. unsupported requests produce the fixed `Blossom can't do this yet` response,
   zero proposals, zero approval windows, zero effects and zero executor starts;
10. the `Ask Blossom` result is last and unselected whenever a local result is
    available;
11. Enter on deterministic create opens only the separate approval window and
    the command bar exposes no approval action; and
12. automated contrast checks prove every command-bar text/background token
    pair is at least 4.5:1.

## Migration and rollback

The Petal-token visual direction covering the empty, exact-app,
deterministic-create and bounded-agent states was accepted before implementation. The
existing launcher and agent buttons remain available until the new routing and
accessibility gates pass.

Rollback restores those existing buttons. It must not route unresolved input
directly to the model, embed approval in the bar or weaken any approval field.
