# Bagua Linux scenes

Eleven required cases drive only the verified, formally distributed Bagua CLI,
public wire v8 and documented configuration. Each gets a separate labelled
container and private volume. First-party artifacts follow the configured RC
or stable channel and retain their resolved identities. There is no product
crate dependency, source-checkout lookup, runtime Cargo build, testing switch,
credential requirement or interaction with a production workspace.

| Exact case ID | Assertions |
| --- | --- |
| `bagua-workspace` | Original creation, explicit program launch, split/window/focus, controller transfer, observer refusal, failure atomicity, fixed/automatic geometry; actual controlling PTYs check text/keys/paste/application mouse/local selection, detach/reacquire and terminal restoration. |
| `bagua-lifecycle` | Original disconnect/client SIGKILL persistence; explicit reacquisition; exit status versus EOF; automatic exit retirement and last-session removal; launch failure; retired identity refusal; stubborn direct-child reap before completion. |
| `bagua-flow` | Original exact flood byte/bell counts, fragmented UTF-8, stopped observer/full resynchronization, tiny viewports, digest of 32768 admitted bytes across controller loss, capacity refusal/recovery. |
| `bagua-config` | Explicit/XDG/HOME discovery, no working-directory search, no-config, literal paths and redacted program values; unknown/syntax/type/range/action refusal; failed whole reload and startup-section refusal; successful combined preference reload preserves identities/control. |
| `bagua-entry` | Default private endpoint, missing observer and unknown endpoint refusal, same-content/different-inode executable refusal, concurrent ordinary entry creates one exact named workspace, detach persistence and own host/application reclamation; foreground `cd` propagates to split/window, immediate trailing input targets the new pane, deleted cwd refuses creation. |
| `bagua-history` | 50000 full-width physical rows at both 80 and 160 columns are read back individually, in bounded one-pane pages; line eviction, actual byte budgets and two-pane fair share; alternate-screen exclusion, foreign scope/evicted range/resize epoch refusal, reconnect, ongoing output preserves a frozen page and its local selection, keyboard/wheel navigation returns live, primary clear refuses stale/bootstrap history. |
| `bagua-input` | Ordinary hosted applications observe actual query/DA replies and Enter versus ShiftEnter bytes for flags 1/5/7; stack counts, set/clear/unknown masks, primary/alternate isolation, RIS and malformed requests; prefix literal, releases, legal paste clearing prefix and illegal control-character paste refusal; batched split/window/Alt-index/click/close routing; 32-window capacity failure drops unsent text and acknowledgement never replays it. |
| `bagua-mouse` | Local versus explicit OSC52, drag-release versus manual copy and reload limit refusal; complete base64/ST including 38400-byte Unicode soft-wrapped selection, hard newline and wide-cell boundaries; hosted application OSC is blocked; actual divider drag changes PTY widths, observer cannot resize, click focuses before text and invalid minimum preserves usable panes. |
| `bagua-display` | Actual header/separator content-height reservation, observed foreground cwd and active indexed blue, observer cannot reserve rows, atomic invalid/valid decor reload, prefix/help/error priority, tiny reservation failure preserves ordinary input; a delayed reader sees final counter rows with bounded total traffic and idle status-row updates; Shanghai minute display. |

The ordinary Python programs record their own process identities and received
bytes inside the case volume. All direct children must be gone after successful
host shutdown. The entry trial also observes and reaps its exact self-started
host and verifies its shell disappeared. The runner independently tears down
and reads back every labelled lane resource. Failure tracebacks and bounded
current-screen diagnostics remain case output; no required case is excluded.

The small outer-terminal observer interprets the standard VT sequences emitted
by the client, reconstructing current cells across partial/coalesced writes and
capturing complete OSC strings. It is a fixture, not a terminal emulator
compatibility claim. Counters measure emitted bytes, including discarded old
capture data. They are traffic budgets, not throughput or latency SLOs.

Current-page selection does not promise unloaded cross-page copying, reflow or
full grapheme clustering. The synthetic applications establish protocol and
application-byte behavior; they do not establish real Codex composer UX. Actual
installed Codex startup and safe consumption belong to Bagua #32; ET key
interpretation, system clipboard state, colors and subjective daily habits
belong to independent human verification #33. These cases never read or write
the host OS clipboard and never submit a business prompt.

Source Guard and AST/TOML checks establish material readiness only. Formal
qualification requires the published complete Scene generation, actual formal
runner/runtime and exact candidate artifact through Wharf, all eleven cases and
successful cleanup. Source merge, a development `--scenes` override and a
historical pass cannot replace that credential. A changed whole generation
requires fresh qualification under current requirements.

Seatrial #88 corrects an erroneous paste positive: Ctrl-Q inside a paste is refused by the existing Bagua contract. The fixture separately checks a legal paste while a prefix is pending, explicit refusal of the illegal paste, unchanged application bytes and unchanged window count. It does not relax the input contract or drop coverage.

Seatrial #90 preserves the public admission boundary: an immediate matching Reply::Err terminates an unadmitted request. Finished is checked for admitted work, including later native failure; negative history reads accept their immediate refusal. The fixture never waits for a completion the protocol does not promise.

Seatrial #91 keeps the long copy inside the advertised 256-column, 64-row host canvas. The 38400-byte sample uses 12800 supported base-plus-combining cells in 50 rows, with matching in-page coordinates. Each sample waits for its actual complete visible content before selection; no fixed delay or unloaded-row assumption supplies readiness. The separate CJK wide-cell and hard-newline checks remain.

The valid combining-text sample exposed [Bagua #47](https://github.com/PerishLab/bagua/issues/47): the first RC loses base characters across UTF-8 input chunks. The required mouse case retains this counterexample and remains selected for the complete nine-case qualification after the product fix.

Seatrial #97 waits for the actual HISTORY-DONE screen before sending the first history gesture. Entering the outer alternate screen alone does not establish that the client has installed its initial typed screen. The complete 50000-row checks at 80 and 160 columns, frozen-page copying, wheel/live navigation, invalid cursors and shared budgets remain required.


## Interaction recovery and takeover

`bagua-interaction` extends the original nine Bagua cases for wire8 consumers.
It asserts invalid-index/no-op navigation followed by real input, failed layout
and native exec without fallback delivery, historical release/keyboard copy
followed immediately by shell commands, and local Copy/prefix Escape behavior.
Explicit takeover binds the observed owner; success restores the old TTY while
preserving shell PID/state. Rejection and competing choices preserve the actual
controller, observers stay read-only, and fixed takeover preserves the canvas.

A case-owned Unix proxy forwards the public framed protocol to the real host.
It can retain an actual refusal until a partial paste has left the owned TTY
input queue, or omit a focus reply and retain the subsequent synchronization
response. The case verifies no Escape-unlocked input or replay, actual fresh
catalog completion, persistent recovery after a notice expires, bounded client
exit with restored terminal, and surviving host/application processes. It uses
no product source, runtime build, test endpoint or product testing mode.

The input case replaces persistent-pause acknowledgement with explicit refusal
and fresh input without Escape; cold-entry races retain unique host/workspace
assertions and settle the loser through its chooser. History copying returns
live without an extra End. Full history capacity, byte budgets, real keyboard,
mouse, display, cleanup and the original nine required case identities remain.
Source Guard and local scene overrides do not establish published-artifact
qualification; Bagua's exact new candidate and complete scene generation must
run through the formal release path before stable promotion.


## Automatic process departure

`bagua-exit` preserves the original ten case identities and adds normal,
nonzero and signal exits, tail draining before retirement, EOF without exit,
background retirement with no controller, surviving-pane geometry/focus,
last-window fallback and final-session client/observer terminal restoration.
Reentry with the same name creates fresh identities; unrelated applications
and the host survive. Startup failures remain covered by lifecycle.
Process outcomes are observed through the reliable public subscription stream,
including events interleaved with requests, rather than inspecting retired IDs.
The old v0.3 retention assertion is intentionally replaced by Bagua #63's
new default contract; no required case is disabled.
