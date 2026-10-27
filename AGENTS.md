# Agent guide

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure. That document owns
organization-wide Issue, pull-request and acceptance policy, and its
"Integration testing" section owns what belongs here; this file keeps
repository-specific constraints without copying that policy.

Seatrial is the domain's black-box test repository (PerishLab/.github#127).

## Constraints

- Cases are black-box. They drive executables only through HTTP, CLI, wire
  protocols and files an executable is documented to read. Seatrial never
  depends on a product crate.
- `seatrial.toml` names what a run trials. A first-party product is followed
  by channel, never pinned: the run resolves the channel pointer at
  `<authority>/v1/channels/<channel>.json` and records the version, commit
  and digests it actually ran. Third-party images are pinned by exact tag.
  The pointer's seal and every downloaded artifact are checked against their
  digests before anything runs.
- Every run is one lane, `st<hex>`. Each container, network and volume carries
  `perish.owner=seatrial` and `perish.lane=<ns>`. No host port is published:
  a case reaches an executable from the in-lane probe, and an executable a
  case drives answers as `<component>.<ns>.internal` on port 8080. Teardown
  removes everything labelled with the lane and reports anything left.
- Each case gets a store of its own: a fresh volume, SQLite file or Postgres
  database. A case never resets shared state. Cases run in parallel.
- A case may reach into its own store only with the store's own tool, `psql`
  in the Postgres container or `sqlite3` from the pinned SQLite image, and
  only to sabotage or observe the store the way an operator could.
- A case asserts the behavior a product promises, never the behavior it
  has. A case that a product breaks names its Bug in the README and is listed
  in `OPEN`, which keeps it out of the default suite until the Bug is fixed.
- Guard proves the runner's source only and never runs the suite. In phase 1
  a run gates nothing; its output is manual evidence on the Issues that use
  it.
- Source code admits no comments.

## Repository map

- `crates/cli` — CLI assembly, scene selection, and the transitional Keel
  runner. Keel-specific behavior stays here until its scene migration.
- `crates/lib` — scene-independent process, artifact, file and Docker
  capabilities. Product-specific setup and assertions never belong here.
- `scenes` — TOML entry points and adjacent black-box scene programs.
- `seatrial.toml` — what a run trials.
- `models` — the canonical Keel manifests the cases serve.
- `.runseal` — committed inert Runseal resources.
