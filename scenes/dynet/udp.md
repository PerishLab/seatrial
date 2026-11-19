# Two-hop UDP scene

Run `dynet-udp` from the published scene generation against a Dynet channel
containing Dynet #148. The release operator records the resolved channel
seal, commit, artifact digest and scene generation from the report. For
candidate qualification, use the normal pins file with the Dynet channel
set to `rc`.

```sh
seatrial run --pins /path/to/candidate-pins.toml --case dynet-udp --report /path/to/new-report.json
```

The direct control proves that the client can reach the WAN witness through
router source NAT. This keeps return traffic compatible with Docker bridge
filtering. The live checks require the witness to see the exit address and
return exact datagrams from two destinations. The failure checks compare
witness request counts while the exit is down and after the router worker
is killed. A fresh exchange must work after each recovery. The case also
checks fragment suppression with a smaller client MTU, saturation at the
configured 32-session budget and the client socket's ICMP error after
the encrypted policy removes UoT.

An opaque datagram on UDP port 443 exercises routing on that port. QUIC
interoperability and the OpenWrt/Pixel operating checks require their own
observations. The Seatrial report must show the lane's owned resources
removed before the run is accepted.
