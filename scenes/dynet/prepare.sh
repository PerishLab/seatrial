set -eu
if [ -z "${SEATRIAL_BINARY_SECRETS:-}" ]; then exit 0; fi
rustc -vV
rustc --edition 2024 "$SEATRIAL_SCENE/wire/main.rs" -o "$SEATRIAL_WORK/dynet-dns"
sha256sum "$SEATRIAL_WORK/dynet-dns"
