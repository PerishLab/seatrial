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
