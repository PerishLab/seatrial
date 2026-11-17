# Plumb cache lifecycle

`cache-lifecycle` is the first bounded black-box case for Plumb's Guard build
cache under Seatrial. It runs the released `plumb` product in the normal
`plumb` image, creates a synthetic committed Rust repository in a lane-private
volume, and uses `PLUMB_HOME=/trial/state` for all cache state.

The case asserts that:

- the first successful Guard creates exactly one Cargo bucket whose identity is
  the fixture's git common directory;
- `plumb cache status --json` reports that bucket as live with non-zero `bytes`
  and `working`, before Seatrial tears the lane down;
- Guard's diagnostic finishes with exit code zero and the fixture remains clean;
- no transport parcel is requested by this case; `plumb cache transport` is never invoked.

The formal observation is printed to stdout. Seatrial's runner owns teardown;
the case never deletes its own container or volume. Failure, timeout and
cooperative-cancellation cleanup are exercised by the Issue acceptance harness
through the public runner, not by assertions inside this success case. A
SIGKILL is documented as an orchestration residual rather than a Plumb cleanup
guarantee.

## Disposable source compilation

`cache-run` drives the released `plumb cache run` CLI with lane-private,
dependency-free Cargo source. It checks successful compilation, nonzero and
signal status, timeout, background process-group cleanup, cooperative cancellation
alongside an independent successful run, literal arguments, existing binding and
output refusal, and explicit retention when ownership records change. It also
checks source files, the ordinary source target and unrelated state remain intact.

The bounded stdout observation retains each admitted invocation, result and raw
stdout/stderr, plus the actual Plumb version and binary digest. The generic runner
owns lane teardown, including intentionally preserved ownership-conflict material.
This case qualifies a formal Plumb prerelease through the public CLI; source-built
or locally substituted binaries remain development observations only. Supervisor
SIGKILL and deliberately escaped process groups remain outside cleanup guarantees.
