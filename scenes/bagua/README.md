# Bagua Linux scenes

Nine required cases drive only the verified, formally distributed Bagua CLI,
public wire v7 and documented configuration. Each gets a separate labelled
container and private volume. First-party artifacts follow the configured RC
or stable channel and retain their resolved identities. There is no product
crate dependency, source-checkout lookup, runtime Cargo build, testing switch,
credential requirement or interaction with a production workspace.

| Exact case ID | Assertions |
| --- | --- |
| `bagua-workspace` | Original creation, explicit program launch, split/window/focus, controller transfer, observer refusal, failure atomicity, fixed/automatic geometry; actual controlling PTYs check text/keys/paste/application mouse/local selection, detach/reacquire and terminal restoration. |
| `bagua-lifecycle` | Original disconnect/client SIGKILL persistence; explicit reacquisition; exit status versus EOF; empty-session retention; launch failure; retired identity refusal; stubborn direct-child reap before completion. |
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
runner/runtime and exact candidate artifact through Wharf, all nine cases and
successful cleanup. Source merge, a development `--scenes` override and a
historical pass cannot replace that credential. A changed whole generation
requires fresh qualification under current requirements.

Seatrial #88 corrects an erroneous paste positive: Ctrl-Q inside a paste is refused by the existing Bagua contract. The fixture separately checks a legal paste while a prefix is pending, explicit refusal of the illegal paste, unchanged application bytes and unchanged window count. It does not relax the input contract or drop coverage.
