# Seatrial

Seatrial puts PerishLab's executables through a sea trial: the whole vessel
run in real water and judged only by how it behaves. Its cases are
black-box. They drive released executables through their public interfaces,
over real services, and never link a product crate.

Every run is one lane. It gets a fresh namespace, and its network and volumes
are labelled with `perish.owner` and `perish.lane`. No host port is
published, and teardown leaves nothing labelled with the lane.

## Running

On Linux with Docker:

```sh
cargo run --release -- run
cargo run --release -- run --case sudo --case restart
```

`seatrial.toml` names what a run trials. Keel is followed by channel: each
run resolves Keel's current stable from its release authority's stable
pointer, the one `manage.sh` reads, and prints the version, commit, CLI
archive digest and image digest it actually ran. The Postgres, probe and
SQLite tool images are third-party and stay pinned by exact tag.

Every Keel case serves the one model in `models/keel.json`, a canonical
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
- `lease`: a lease in the past is refused, and a leased row disappears when
  its lease ends.
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
- `pulse`: put, set and end each leave a pulse row, read through `/query`.
- `foreign`: a write through a second `keel-api` on the same estate
  invalidates the first one's cached reads.
- `revived`: after Postgres terminates the backend, reads recover within one
  failed request and writes succeed. On SQLite the case proves only that
  reads and writes continue.
- `vetoed`: when the store rejects the pulse append, the write fails and
  commits nothing. The rejection is planted with the store's own tool: a
  `CHECK (false) NOT VALID` constraint through `psql`, and an aborting
  trigger through `sqlite3`.
- `blank`: a badge's `label` is unique per `kind`, and badges without a kind
  share one scope: a second `B1` without a kind is a 409, and `B1` of another
  kind is a 201.

The race cases run on Postgres only, with two `keel-api` processes on one
estate (PerishLab/seatrial#7). A third Postgres session holds the `@clock`
`pulse` row, which every Keel write updates after its checks and its own
write. The first request is sent, the second follows once the first is
seen waiting, and the holder is terminated once both are waiting:

- `rival`: two processes put the same unique value; one answers 409.
- `lapse`: one process puts a unique value while the other edits a leased
  row to it. Broken, PerishLab/keel#68.
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
- `void`: one process puts a badge without a kind while the other edits
  another badge without a kind to the same label. Broken,
  PerishLab/seatrial#17: a unique index treats null scopes as distinct.
- `kin`: the same pair with both badges of one kind; one live badge holds
  the label.

A broken case asserts the invariant, so it fails until its Bug is fixed. It
is left out of the default suite and runs by name:

```sh
cargo run --release -- run --case lapse --case void
```

In phase 1 a run gates nothing; its output is evidence on the Issues that
use it.
