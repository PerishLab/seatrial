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
archive digest and image digest it actually ran. The Postgres and probe
images are third-party and stay pinned by exact tag.

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

In phase 1 a run gates nothing; its output is evidence on the Issues that
use it.
