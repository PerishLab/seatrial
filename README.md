# Seatrial

Seatrial puts PerishLab's executables through a sea trial: the whole vessel
run in real water and judged only by how it behaves. Its cases are
black-box. They drive released executables through their public interfaces,
over real services, and never link a product crate.

Every run is one lane. It gets a fresh namespace, and its network and volumes
are labelled with `perish.owner` and `perish.lane`. No host port is
published, and teardown leaves nothing labelled with the lane.

## Running

On Linux with Docker, use the distributed generic `seatrial` executable and a
`seatrial.toml` configuration:

```sh
seatrial run --report result.json
seatrial run --case writer --report writer.json
```

The default scene source is `[scenes]` in that configuration. It resolves the
Depot channel once, verifies its version standing, generation and object bytes,
and executes the resulting temporary material. The pilot follows the stable
scene channel and Plumb's published `rc` channel independently. A missing,
inconsistent or unreadable source refuses; it never falls back to a checkout.
The host needs no Rust toolchain for the portable writer bundle.

For scene development, select an explicit local root:

```sh
cargo run --release -- run --scenes scenes --case sudo --case restart
cargo run --release -- run --scenes scenes/plumb/follow --case writer
```

The CLI lives in `crates/cli`; `crates/lib` supplies scene-independent
execution, artifact verification, file snapshots and lane management. Keel's
black-box driver lives under `scenes/keel/src`, compiled as the separate
`seatrial-keel` auxiliary executable. Product-specific code stays in scenes.

`scenes/plumb/follow/writer.toml` selects the writer scene. `writer` remains
its short alias. A scene command runs on the host from a fresh snapshot of
the manifest's directory; adjacent scripts may drive Docker and other tools
directly. `--scenes <directory>` selects a local scene root. Several scenes can be
selected in one invocation. With `--scenes scenes` and no `--case`, the legacy
suite selects the same 28 Keel cases and 48 storage instances. Short
names resolve only when unique; otherwise use the qualified scene path.

```toml
products = ["plumb"]
image = "plumb"
timeout = 120
command = ["sh", "-eu", "writer.run.sh"]
```

`products` names channel configurations in `seatrial.toml`; `image` optionally
names a product's configured runtime image. That root configuration remains
the sole owner of channels, release authorities and image references. `timeout`
is the positive command budget in seconds, starting after preparation, and
`command` is an argv list. Unknown fields are refused. There is no expression,
step or assertion language.

The command receives `SEATRIAL_LANE`, `SEATRIAL_VOLUME`, `SEATRIAL_SCENE` and
`SEATRIAL_WORK`, plus `SEATRIAL_BINARY_<PRODUCT>` for each verified product
and `SEATRIAL_IMAGE` when selected. The scene snapshot hash, product commit,
archive digest and actual runtime image digest are printed with the result.
Snapshot identity includes adjacent file names, contents and permissions;
symbolic links and special files are refused. Scene code owns its business
setup and assertions. The writer's original business script is unchanged.

A portable scene root can declare its entry points in `bundle.toml`:

```toml
format = 1
cases = ["writer"]
```

Case names are relative paths without the final `.toml`. Only the listed
manifests are parsed; adjacent TOML files can remain ordinary scene assets.
Unknown formats or fields, empty or repeated entries, missing cases and escaping
paths refuse before execution. The bundle's listed cases form its catalog;
without `--case`, their manifest `default` values select the run. An excluded
case remains explicitly selectable. Group preparation still uses the nearest
`suite.toml`. The legacy repository-wide catalog keeps its existing defaults.
For example, `--scenes scenes/plumb/follow` selects the portable writer root,
while explicitly selecting the repository root keeps the 48 Keel instances.

The release declaration ships only the generic Linux `seatrial` executable.
The auxiliary product drivers remain development inputs during migration.
Runner distribution and Depot consumption are tracked in
[the independent writer pilot](https://github.com/PerishLab/seatrial/issues/32).

A nearest `suite.toml` groups cases and optionally declares one `prepare`
command, its command timeout, shared `products` and `default = true`. The runner
prepares every selected group before starting its cases concurrently, using
one lane and one verified product resolution per invocation. There is no
nested inheritance or business-step language. Keel's suite shares one Postgres
service while each case owns its database and SQLite volume. Each case TOML
explicitly passes `lite`, `pg` or both to its scene driver, preserving the 48
independent instance verdicts. An excluded broken case uses `default = false`
and its `bug` reference; it remains explicitly selectable.

`SEATRIAL_SOURCE` names the original group directory; `SEATRIAL_SCENE` is its
snapshot and `SEATRIAL_WORK` its writable run directory. Configured images are
available as `SEATRIAL_IMAGE_<NAME>`, verified products additionally provide
`SEATRIAL_BINARY_<PRODUCT>` and `SEATRIAL_VERSION_<PRODUCT>`. Source changes
during preparation are refused. Keel's prepare script builds its auxiliary
binary once through the repository's cached, locked offline Cargo target,
copies it into the run directory and prints its SHA-256. This source-only
runner therefore needs the Rust/Cargo toolchain as well as Docker; build time
is part of the invocation's overall cost. Existing Rust assertions, recovery
loops and race barriers remain in the scene program and use public product
interfaces. Guard compiles this auxiliary target and tests the runner without
running the Docker suite.

`--report <NEW_FILE>` reserves a new file with owner-only permissions and writes
one JSON `seatrial.run/v1` report at completion, including preparation failures.
It records the running binary's SHA-256, platform, resolved scene pointer,
executed group snapshots, product channel/version/commit/seal/archive digests,
selected runtime image digests, case verdicts and elapsed times, and the lane's
cleanup result. Local scenes have no Depot pointer. An incomplete run, failed
cleanup or cancellation cannot report success. Reports are observations, not
signed attestations; consume the command's exit status as well. Existing files
are never overwritten. A report write failure is an operational failure.

Reports additionally carry a flat `stages` list: `configuration`, `scene`,
`selection`, `lane`, `artifacts`, `preparation`, `execution` and `cleanup`.
Each entry records its start offset from the run, elapsed seconds and `ok`.
Only stages that started are listed; failures keep their elapsed time, and
cleanup is measured independently. An execution stage passes only when every
selected case passes. Case times can overlap during parallel execution;
stage times are sequential. Total time includes work outside these spans.
Timing is observational and preserves the command's original result semantics.

Timeout, Ctrl-C and SIGTERM terminate the scene process group and wait for its
command before lane teardown. Cancellation exits 130. Failed commands retain
stdout, stderr and status; cleanup errors are reported alongside the original
failure. Every cleanup attempts containers, volumes and network, then reads
back remaining resources. Cleanup failure retains the working directory for
inspection. An uncatchable SIGKILL cannot run teardown. Scene commands are
trusted repository programs using the host's ordinary tools, not a sandbox.

`seatrial.toml` names what a run trials. Keel is followed by channel: each
run resolves Keel's current stable from its release authority's stable
pointer, the one `manage.sh` reads, and prints the version, commit, CLI
archive digest and image digest it actually ran. The Postgres, probe and
SQLite tool images are third-party and stay pinned by exact tag.

Every Keel case serves the one model in `scenes/keel/model.json`, a canonical
manifest written by Keel.

## Cases

Each Keel case runs on SQLite and on Postgres, with a fresh store and its own
`keel-api` container:

- `again`: bootstrap seals, and a second bootstrap is refused and writes no
  custody file.
- `health`: `keel-api` serves `/health`.
- `sudo`: a sudo write reads back, an anonymous write is refused and a forged
  sudo is 401.
- `restart`: a written row reads back after `keel-api` restarts over the same
  estate.
- `unique`: a second row with the same unique value is a 409.

The parity cases prove that the two adapters behave the same
(PerishLab/seatrial#5):

- `portable`: create, read, list, edit, end, tie, tune and cut; a unique
  conflict; a linked query and a count; a tied row refuses to end.
- `scoped`: a scoped serial numbers entries per ledger.
- `spare`: an optional field reads as absent or held.
- `lease`: a lease in the past is refused, a leased row's expiry reads back
  through `/-/keel/row`, and the row disappears when its lease ends.
- `outlive`: an entry put under a leased ledger is refused, so no live entry
  remains once the lease ends.
- `linger`: a tie to a leased course is refused, so no live tie remains once
  the lease ends.
- `ghost`: a tie whose course has ended is gone from every query path, as
  from `link`. The ended course is planted with the store's own tool, since
  no API path leaves such a tie since Keel v0.20.2. Fixed in Keel v0.20.3,
  PerishLab/keel#73.
- `cascade`: ending a root ledger ends its entries.
- `batch`: a batch with a failing deed commits nothing; a good one answers its
  ids and keeps every deed.
- `pulse`: put, set and end each leave a pulse row, read through
  `/-/keel/pulse`.
- `foreign`: a write through a second `keel-api` on the same estate
  invalidates the first one's cached reads.
- `revived`: after Postgres terminates the backend, reads recover within one
  failed request and writes succeed. On SQLite the case proves only that
  reads and writes continue.
- `vetoed`: when the store rejects the pulse append, the write fails and
  commits nothing. The rejection is planted with the store's own tool: a
  `CHECK (false) NOT VALID` constraint through `psql`, and an aborting
  trigger through `sqlite3`.
- `blank`: a model whose unique scope is optional is refused at bootstrap,
  so a key has no null part.
- `closed`: a row carries no `created_at`, `updated_at` or `expires_at` on
  create, read, list or query; `@seal` and `@pulse` answer 404 through list,
  read, query and batch; `/-/keel` refuses anonymous callers.

The store cases run on Postgres only:

- `outage`: while the store refuses connections, reads and `/health` answer
  503 `store unavailable`; once it accepts them again, `/health` recovers and
  the written row reads back.

The race cases run on Postgres only, with two `keel-api` processes on one
estate (PerishLab/seatrial#7). A third Postgres session holds the `@clock`
`pulse` row, which every Keel write updates after its checks and its own
write. The first request is sent, the second follows once the first is
seen waiting, and the holder is terminated once both are waiting:

- `rival`: two processes put the same unique value; one answers 409.
- `lapse`: a leased row holds no unique value: a put of a leased row's value
  is a 201, a put of a held value is a 409, and a put racing an edit of a
  leased row to the same value both succeed (PerishLab/keel#68).
- `dangle`: one process ends a course while the other ties a student to it;
  no live tie to the ended course remains. Fixed in Keel v0.20.2,
  PerishLab/keel#69.
- `cling`: the same pair in the other order, the tie first and the end
  second.
- `orphan`: one process ends a ledger while the other puts an entry under it;
  no live entry under the ended ledger remains. Fixed in Keel v0.20.2,
  PerishLab/keel#69.
- `adopt`: the same pair in the other order, the entry first and the end
  second.
- `kin`: one process puts a badge while the other edits another badge of
  the same kind to its label; exactly one live badge holds the label, and
  the loser answers 409 `field label taken`.

In phase 1 a run gates nothing; its output is evidence on the Issues that
use it.

The opt-in `writer` case can run alone or beside selected Keel cases. It follows
Plumb's configured channel, verifies the downloaded archive, and drives the
released `plumb follow` CLI in the Images environment. Git repositories and
the data home belong to a fresh lane volume. A controlled `gh` executable
answers the three allowed provider reads and refuses every other operation.
The case checks that a foreign operator is refused and that the source,
local and remote refs, and registered worktrees stay unchanged. It uses no
product crate or private state format. Teardown runs after a failed trial too.

This proves the command boundary against a controlled provider, not the real
GitHub service. It covers the first-discovery refusal only; the existing
seated and publication-stage tests remain in Plumb until their guarantees
are carried over. It is not a candidate-artifact gate and does not replace
the current per-commit regression tests (PerishLab/seatrial#21,
PerishLab/.github#136).

## Encrypted Dynet control

`cargo run -- run --scenes scenes --case dynet/control` explicitly selects the coupled Dynet and
Secrets trial. It follows their configured channels and the existing Plumb
runtime image, records artifact/image identities and builds its own small Rust
DNS fixture. The host needs `jq`, `rg`, Docker and `/dev/net/tun`; the client
container gets NET_ADMIN only in its isolated shared namespace. Its disposable
filesystem installs Debian trixie's exact `iproute2=6.15.0-1` fixture tool and
prints that package version. See the [Debian package record](https://packages.debian.org/trixie/amd64/iproute2).

The scene provisions an owned persistent TUN interface and routing-table alias
through ordinary OS interfaces. It checks configuration activation and route
projection, without claiming system resolver takeover or commercial VPN egress.
API, publisher and device use separate filesystems; only their network namespace
is shared, making the SDK's literal-loopback HTTP origin truthful. The API is
reachable as `api.<lane>.internal:8080` from the probe. No host port is published.
The API sees a pinned public publisher root, opaque bundles and digest-only
credentials; it never mounts the signer or device identity.

The flow checks encrypted URL application, subscription and heartbeat, restart
from confirmed cache with the API stopped, generation/conflict refusal, durable
revocation across an offline restart, higher-generation recovery and API-token
denial without a false stop claim. It also drives encrypted file input, invalid
payload masking and the client's received-generation floor through the public
CLI. SQLite observation uses the configured pinned SQLite image against only
this case's store. Lane teardown removes all stores, containers and namespace.

A product configuration may list `binaries = ["secrets-api"]` to request extra
executables from the same verified archive. The product-named executable remains
`SEATRIAL_BINARY_<PRODUCT>`; `SEATRIAL_BINARIES_<PRODUCT>` is a JSON map of all
selected executable names to local paths. Selection rejects unsafe/repeated
names, duplicate archive members, non-regular members and missing executables.
The archive is fetched and checked once for the selected channel identity.

An image pinned by both exact tag and SHA-256 may reuse the matching local
repository digest, which is printed explicitly. Mutable channel images still
require a fresh pull. This does not substitute a different digest on failure.

The portable Keel pilot at `scenes/keel/native` separates the SQLite instances of
`health` and `again`. It uses the resolved published CLI and that release's OCI
image, with no Cargo preparation. The image API version must equal the resolved
CLI version. A product image without a tag follows that resolved release marker;
an explicit tag or digest keeps its existing meaning. Reports record both the
resolved reference and immutable runtime digest.

The two pilot cases keep the legacy assertions: health bootstraps custody,
starts the API, reads custody, and independently requires anonymous `/health`
HTTP 200 after readiness; again requires the first custody file, refuses a
second bootstrap without a second custody file, and requires status `sealed`.
Each gets a fresh volume. The legacy group and its PostgreSQL instances remain;
the portable group is excluded from the legacy root defaults and selected through
its own Depot bundle. The stable writer bundle is independent of this pilot.
