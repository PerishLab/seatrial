root=$(cd "$SEATRIAL_SOURCE/../.." && pwd)
cargo build --offline --locked --manifest-path "$root/Cargo.toml" --target-dir "$root/target" --bin seatrial-dynet
cp "$root/target/debug/seatrial-dynet" "$SEATRIAL_WORK/dynet-dns"
sha256sum "$SEATRIAL_WORK/dynet-dns"
