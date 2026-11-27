set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: sh crates/cli/consign-scene.sh <stable-marker>" >&2
    exit 2
fi

marker=$1
if ! printf '%s\n' "$marker" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "expected an exact stable marker such as v0.6.3" >&2
    exit 2
fi

cd "$(git rev-parse --show-toplevel)"
commit=$(git rev-parse --verify "refs/tags/${marker}^{commit}")
if [ "$(git rev-parse HEAD)" != "$commit" ] || ! git diff --quiet "$commit" --; then
    echo "check out the exact stable marker with clean tracked files before consigning" >&2
    exit 2
fi

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
git archive --format=tar --output="$stage/scenes.tar" "$commit" scenes
tar -xf "$stage/scenes.tar" -C "$stage"
diff -qr scenes "$stage/scenes"
cargo test --locked -p seatrial --bin seatrial
mv "$stage/scenes/release.toml" "$stage/scenes/bundle.toml"
plumb depot consign --version "$marker" --kind scene --dir "$stage/scenes" --no-watch
