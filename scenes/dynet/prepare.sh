set -eu
rustc -vV
rustc --edition 2024 "$SEATRIAL_SCENE/wire/main.rs" -o "$SEATRIAL_WORK/dynet-dns"
sha256sum "$SEATRIAL_WORK/dynet-dns"
