root=$(cd "$SEATRIAL_SOURCE/../.." && pwd)
cargo build --offline --locked --manifest-path "$root/Cargo.toml" --target-dir "$root/target" --bin seatrial-keel
cp "$root/target/debug/seatrial-keel" "$SEATRIAL_WORK/keel-scene"
sha256sum "$SEATRIAL_WORK/keel-scene"
exec "$SEATRIAL_WORK/keel-scene" prepare "$SEATRIAL_LANE" "$SEATRIAL_WORK" "$SEATRIAL_BINARY_KEEL" "$SEATRIAL_SCENE/model.json" "$SEATRIAL_IMAGE_KEEL" "$SEATRIAL_VERSION_KEEL" "$SEATRIAL_IMAGE_POSTGRES" "$SEATRIAL_IMAGE_SQLITE"
