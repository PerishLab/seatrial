# Two-hop UDP scene

Run `dynet-udp` from the published scene generation against a Dynet channel
containing Dynet #148. The release operator records the resolved channel
seal, commit, artifact digest and scene generation from the report. For
candidate qualification, use the Dynet RC channel and its declared runtime
image. The witness program is built in that image.

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

The four files in `fixture/` are public, inert test enrollment material.
The private-key format in `identity.age` decrypts these two test envelopes
only. The fixture has no external endpoint or account. The Secrets CLI seals
both generations against the fixture recipient. The second generation omits
the exit's UoT declaration. The fixture DNS maps the front name to the
current lane address, and the front resolves the exit from its local hosts file.
