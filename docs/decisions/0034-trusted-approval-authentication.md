# ADR-0034: Password-backed approval for model-proposed effects

Status: Accepted

## Context

The existing approval protocol binds a decision to an exact preview, request
identity, peer and expiry. It proves that the same session-bus connection which
started a request returned the decision. It does not prove that a human made
that decision: another process running as the desktop user can call the same
closed D-Bus methods and approve its own request.

Blossom now admits one bounded model-proposed effect, create-only workspace
publication. Adding further effects while same-user self-approval remains
possible would turn the visual prompt into presentation rather than an
authorization boundary.

The desktop user was also provisioned into Linux's `input` group. That grants
every process running as the user direct access to raw input devices, including
password and approval keystrokes. Hyprland does not require this membership:
logind grants the active compositor the devices it needs.

Linux and Wayland provide no general-purpose UI primitive whose pixels are both
conveniently integrated into a user session and cryptographically
unspoofable. The first trusted approval design must therefore state what it
proves and what it does not.

## Decision

### Remove ambient raw-input authority

No Blossom desktop, live, development or legacy provisioning path adds the
desktop user to the `input` group. Input devices remain mediated by logind and
the compositor. A closed source test rejects reintroduction of the membership.

### Authenticate model-proposed effects

Every model-proposed effect requires a dedicated polkit action whose policy is
`auth_self`. A successful ordinary shell approval is not sufficient. Trusted
code derives the action from the already prepared request and submits the exact
approval-binding digest, effect type and expiry to the privileged authorization
boundary. The client cannot replace the prepared request or select another
action.

The policy declares `unix-user:blossom` as the action owner. This narrowly
allows the non-root broker to ask PolicyKit about the action while attaching
the exact request and preview-digest details. Action ownership does not grant
authorization, does not bypass `auth_self`, and does not let the broker choose
another action. Without it, PolicyKit rejects a non-root caller that supplies
details before evaluating either the production policy or qualification rule.

The decision sequence is:

1. the broker resolves and retains the prepared request;
2. the broker renders the exact preview from that retained request;
3. the user chooses to continue;
4. polkit authenticates the active user with a password;
5. the broker verifies that authorization is fresh and bound to the same
   approval digest; and
6. execution consumes the stored prepared request exactly once.

Denial, cancellation, authentication failure, timeout, peer disconnect,
preview mutation or digest mismatch removes the prepared request and produces
zero effects. Authentication is never cached across model-proposed effects in
the first version.

The broker also limits interactive authentication challenges per trusted
graphical session. It permits at most three challenges in a rolling 60-second
window; the next request starts a 120-second cooldown. Requests during cooldown
fail closed before the interactive PolicyKit call and produce the visible
`authentication_rate_limited` activity category. The broker never evicts an
older approval to make room. Changing to a newly verified graphical session
starts a new limiter scope.

The shell remains a presentation client. It may request that the broker expose
a preview surface, but it never supplies preview text, authorization identity,
the prepared request or the final decision binding. The broker is the sole source of preview data.
The password prompt is provided through the system polkit authentication path,
not a Blossom password field.

### Closed first-version scope

The first implementation applies only to `model_proposed` effects. Human CLI
requests retain their existing exact approval semantics until separately
reviewed. Read-only model operations remain governed by their existing policy
and do not acquire a password prompt merely because a model participated.

No new capability is admitted by this ADR. Plan approval and additional file
operations remain out of scope until this boundary has installed evidence.

## Alternatives considered

### Existing same-user approval only

Rejected for model-proposed effects. It binds exact bytes but cannot distinguish
a human from another process using the same session identity.

### Separate-user confirmation UI

A prompt rendered by a separate account can isolate its process and files from
the desktop user. Wayland prevents ordinary clients from injecting input into
another client, but it does not prevent a malicious desktop process from
drawing a convincing lookalike overlay. This remains a useful future
strengthening, especially when paired with a recognizable secure-attention
gesture, but it does not eliminate phishing by itself.

### Compartment isolation

A Qubes-style trusted desktop or security domain provides the strongest visual
and input separation. It is the security ceiling against which this design is
measured, but it is too large a platform change for the current phase.

## Security meaning and remaining risk

Password-backed `auth_self` prevents a silent same-user process from approving
an effect without knowledge of the user's password. Removing the `input` group
prevents that process from reading the password directly from `/dev/input`.

This does not create trusted pixels. A malicious process can draw a lookalike
preview or password dialog and attempt to trick the user. Polkit's system-owned
authentication flow, consistent wording and absence of any Blossom password
field reduce confusion but do not eliminate overlay phishing. Compromise of the
compositor, polkit agent, kernel, root or firmware remains outside this
boundary.

The authentication agent is also a same-user process. A malicious process can
kill `lxqt-policykit-agent`, register a substitute agent and present a convincing
lookalike challenge. It still cannot authorize the broker's request without the
real password, but it can phish for that password. This agent-substitution route
is a second expression of the same overlay-phishing risk, not trusted pixels.

Using the active graphical session as the PolicyKit subject also permits a
background or remote same-user process to cause a password prompt to appear on
that desktop. Repeated prompts can be used to wear down attention and increase
phishing risk. The per-session challenge limit bounds that pressure and exposes
the cooldown in activity, but it cannot prove which same-user process had
benign intent and it resets when the broker restarts.

## Qualification requirements

Before the path is enabled in a physical candidate, installed Linux evidence
must prove:

1. the desktop user is not a member of `input` and cannot open representative
   `/dev/input/event*` devices directly;
2. a same-user client can request an effect but cannot authorize or execute it
   without successful polkit authentication;
3. correct authentication executes the exact prepared request once;
4. wrong password, cancellation, expiry, replay, mutation and peer disconnect
   produce zero effects and zero executor starts;
5. the audit records the action, preview digest and authentication outcome but
   never a password, password-derived value or file content; and
6. the approval surface contains no password input widget owned by Blossom.

VM-qualification images may install one package-owned polkit rule which returns
`yes` for the `blossom` qualification user and only the exact model-effect
action. This exists solely because the automated driver cannot type a password.
The rule is absent from physical and production packages, and image source
checks enforce that separation. Qualification evidence must state that its
approvals bypassed password authentication and therefore prove pipeline wiring,
not independent human intent.

Production-policy qualification is split deliberately. Automation, running in
an installed image without the bypass rule or qualification driver, proves that
a same-user client cannot self-approve, that a missing authentication agent
fails closed, and that both cases produce zero effects and zero executor starts.
It also proves from installed package and filesystem state that the bypass is
absent. Automation does not claim that a person entered a password.

The automated production-policy image creates a greetd initial session for the
qualification user. This autologin is confined to the ARM64 qualification
builder and is forbidden from physical and production images by source checks.
The gate must observe an active logind session before it starts a request. The
broker first asks polkit non-interactively about the trusted session subject:
only `is_challenge=true` is an eligible environment. A flat unauthorized result
is recorded as `invalid_environment` and can never satisfy the gate. The later
interactive result retains whether that proven challenge was rejected, expired
or could not be served because the agent was absent.

The first complete production-policy run proved that the preceding subject was
still wrong. The graphical logind session was active, but the system-managed
broker process is not a member of that session. Consequently a
`system-bus-name` subject for the broker was classified as inactive and polkit
returned a flat denial.

The first interactive ARM64 run then exposed a second interoperability limit:
PolicyKit's JavaScript backend aborts when asked to evaluate a `unix-session`
subject, even though that subject exists in the public API. The broker therefore
keeps the session as the trust anchor but submits a verified graphical-process
subject. It first discovers exactly one active, local, non-remote Wayland
session for its own UID. Within that session's scope it resolves exactly one
`/usr/bin/Hyprland` process, reads its UID, PID, executable, cgroup and kernel
start time, then re-reads the start time before constructing the subject. The
PolicyKit `unix-process` tuple contains PID, UID and start time; PID alone is
never accepted. Neither the model nor the session-bus client supplies any of
those values. Zero or multiple eligible sessions or graphical processes fail
as `invalid_environment`.

The graphical authentication agent is `lxqt-policykit-agent`, managed by a
systemd user unit tied to `graphical-session.target` with restart-on-failure.
The previously selected Hyprland agent crashed in Hyprtoolkit while allocating
the ARM64 QEMU password window. `blossom-core` also ships a systemd preset for
`polkit-agent-helper.socket`; modern non-setuid PolicyKit helpers cannot perform
password authentication when that socket is absent. These lifecycle choices
change only presentation and helper activation. They do not weaken `auth_self`
or move password handling into Blossom.

The closed selector is tested explicitly with zero sessions, two eligible
graphical sessions and a remote-only session. All three environments fail
closed without displaying an authentication challenge.

The first attempted automated image exposed a separate startup error: a
`Type=oneshot` qualification service was ordered after and wanted by
`graphical.target`, so it held that target open while waiting for the session
broker. The corrected service is non-blocking, waits for greetd, never pulls up
`user@1000.service` or the broker itself, and fails with `invalid_environment`
unless the real session has activated both.

Live inspection of the corrected image then established why the broker itself
remained inactive. Greetd had created a real active Wayland session, the user
manager and polkit agent were running, and the broker was attached to
`user@1000.service`. The broker's `ExecCondition` nevertheless exited at the
namespace step with `226/NAMESPACE`: its writable `BindPaths` source did not
exist because `/home/blossom/Workspace` had never been provisioned. The account
provisioners now create the workspace before a login can start the broker.

A follow-up image established that `%h` is not a valid way to make this system
unit account-neutral: the system manager expanded it to `/root`, before the
numeric `User=` identity was applied. Live replacement with the explicit
qualification-account path made the broker active immediately, with the
intended bounded bind. The ARM64 gate therefore retains the explicit
`/home/blossom/Workspace` path. Mapping an arbitrary graphical-installer owner
name into the system-managed broker is still an open physical-candidate design
item; this gate does not claim that mapping is solved.

The remaining claim requires a single-tester manual check through the Blossom
shell UI: correct authentication produces exactly one preview-matching effect;
wrong authentication and cancellation produce none; two consecutive effects
each open a fresh challenge; and authentication completed after the 30-second
approval lifetime loses to expiry. The tester preserves the exact preview and
system password-prompt screenshots. This evidence is a manual security check,
not a usability study. The maintained checklist is
`docs/TRUSTED_APPROVAL_MANUAL_EVIDENCE.md`.

## Consequences

Model-proposed writes gain deliberate per-effect friction. Systems without a
usable password or polkit authentication agent fail closed. Live sessions may
demonstrate proposals but cannot enable authenticated model effects unless a
separate, explicitly qualified live-session policy is accepted.

Rollback disables model-proposed effects. It must never fall back to the old
same-user approval as a compatibility mode.
