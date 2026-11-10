# Seatrial placement

The formal outer runner drives the distributed Seatrial candidate through its
public CLI. The candidate resolves the current stable Locus Debian package in an
isolated nested fixture, verifies its exact service bytes and version against an
independent dpkg-deb reference in the resolved execution image, then refuses a
noncanonical placement. Both nested lanes must report and independently prove
cleanup. The local override is test input to the candidate, not qualification of
that inner run. Only the complete formal outer case qualifies the candidate.

Locus is a channel-following static package fixture here. The emitted evidence
records its actual source, version, seal, archive and executable digest. This case
does not qualify Locus service semantics or Bagua observation. Those remain with
the corresponding service cases and installed canaries. No maintainer scripts,
source checkout, runtime Cargo build or shared service is used.

`seatrial-service` is an optional diagnostic for the installed runner's Debian
placement. It starts the prepared formal Locus executable in a labelled lane
container, probes its loopback-only health through the canonical component name,
checks its version, stops it with SIGINT and observes exit zero before removal.
The case uses only its own volume and publishes no host port. Its diagnostic role
adds no normative subject coverage. An explicit local `--scenes` invocation is
installed-consumer evidence and never substitutes for formal Scene qualification
or the Bagua/Locus service-combination contract.
