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

In phase 1 a run gates nothing; its output is evidence on the Issues that
use it.
