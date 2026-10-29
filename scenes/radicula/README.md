# Radicula Redis conformance

This scene uses the canonical governance's limited public-library conformance
exception (PerishLab/.github#146). Its separate Rust program links only the
public `radicula-lock` API. The generic Seatrial runner has no product dependency;
Radicula gains no executable, private export or testing mode.

Run the formal Seatrial executable with `--scenes scenes/radicula --case redis`.
The host needs Docker, Python 3.12+, Cargo/Rust and curl. The preparation step
materializes `Cargo.toml.in` as a separate build project and builds the scene program in the lane work directory, following the public
Perish registry with requirement `0`, recording the complete Cargo resolution,
lock checksum and resulting executable checksum. It verifies the latest stable
Cargo index entry and checksum, complete distribution and packaged Git identity
before using the package. Published VCS identity binds Wharf's Cargo snapshot,
whose source identity must match the distribution record; it is not assumed to
equal the original source commit. Every packaged file is compared with the
actual Cargo cache used for compilation. A changed index during preparation refuses. The helper's
build does not install anything on the host or alter the scene source.

`SEATRIAL_RADICULA_CANDIDATE` explicitly opts into a clean committed source tree
for development qualification only. Preparation records commit, tree and archive
SHA, then builds from that immutable archive. Reports label this path
`candidate-source`; it never stands in for formal package consumption or a
promotion credential. Cargo path overrides exist only in this lane's disposable
build invocation, never in the published formal-consumer manifest or runner.

The real Redis service and its wire-fault fixture answer only inside the lane
on port 8080. Drivers use their public stdin/stdout command protocol. Every
container carries both lane labels; no host port is published. Each run owns
its protocol namespace. The scene uses only its own Redis keys and intentionally
restarts that service or pauses its own driver. It never touches shared Redis,
host routes, host credentials or another lane's state.

Assertions cover independent process capacity, policy agreement, shared readers
and exclusive writer, business error/panic and macro cleanup, renewal observed
through the real store, expiry with old work still alive, pause beyond lease,
owner-comparison release and fencing sequence, and explicit restart epoch
separation. The wire fixture waits for a real Redis reply before dropping the
acquire/release/renew response, recording which fault happened. Polls observe
state with deadlines; sleeps only pace those polls. No unknown mutation is
blindly replayed. An old execution is never declared terminated by lease loss.

The scene records package/candidate, resolution, program and image identities.
Its assertion report is printed into the run log and retained in lane work;
the generic runner separately records scene identity, pass/failure, unpublished
ports and full labelled cleanup. A passed scene without confirmed cleanup is
not sufficient evidence. These runs are manual observations until the relevant
Windtun control-plane enforcement is actually verified.
