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
- A run trials released artifacts pinned in `seatrial.toml`: a product's
  version, release authority and image, and third-party images by exact tag.
  A downloaded artifact is checked against its seal before it runs.
- Every run is one lane, `st<hex>`. Each container, network and volume carries
  `perish.owner=seatrial` and `perish.lane=<ns>`. No host port is published:
  a case reaches an executable from the in-lane probe, and an executable a
  case drives answers as `<component>.<ns>.internal` on port 8080. Teardown
  removes everything labelled with the lane and reports anything left.
- Each case gets a store of its own: a fresh volume, SQLite file or Postgres
  database. A case never resets shared state. Cases run in parallel.
- Guard proves the runner's source only and never runs the suite. In phase 1
  a run gates nothing; its output is manual evidence on the Issues that use
  it.
- Source code admits no comments.

## Repository map

- `crates/seatrial` — the runner: lane provisioning, the probe, artifact
  fetching, and the cases.
- `seatrial.toml` — the pins a run trials.
- `.runseal` — committed inert Runseal resources.
