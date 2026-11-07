# Bagua Linux scenes

The three exact IDs are `bagua-workspace`, `bagua-lifecycle` and `bagua-flow`.
Each runs the same verified, formally distributed Bagua CLI in a separate
labelled container and private case volume. The configuration follows Bagua's
RC channel and the Images stable runtime. No product source, crate or runtime
Cargo build is used. The Python programs beside the manifests are ordinary
case-owned hosted applications, not product testing interfaces.

`bagua-workspace` checks public wire v2 creation, explicit launch, splits,
windows, focus, controller transfer, observer refusal, fixed/automatic sizing
and failure atomicity. A real controlling PTY drives the released interactive
client's text, keys, bracketed paste, mouse and local copy/paste; two viewport
sizes, detach/reacquisition and successful terminal-mode restoration are checked.

`bagua-lifecycle` checks that connection loss and interactive client SIGKILL
preserve host and application, with explicit control recovery. Exit code,
PTY EOF, pane removal, empty-session retention and host shutdown are separate
observations. A direct child ignoring SIGHUP must actually be reaped before
close finishes; launch failure remains observable and a retired ID refuses.

`bagua-flow` checks exact terminal byte/bell counts through output pressure,
fragmented UTF-8, an observer that stops reading, full snapshots after reconnect
with an obsolete cursor, narrow/tiny interactive viewports, and the digest of
admitted input consumed after its controller disappears. Pane admission refuses
without model mutation at the documented bound and recovers after closure.

The scene uses only its own PTYs, programs, socket, filesystem and processes.
The runner owns labelled lane teardown and its independent cleanup readback.
Assertion failures retain their traceback in the case output; no default case
is silently removed. Local `--scenes scenes/bagua` execution is development
evidence. Qualification requires the published complete Scene generation and
official runner on the exact product artifact through Wharf, with every case
and cleanup passing. A source merge or local scene pass is not that credential.
