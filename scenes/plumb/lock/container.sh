export PATH="/opt/tools:$PATH"
unset CARGO_TARGET_DIR RUSTC_WRAPPER SCCACHE_DIR
mkdir -p "$HOME"
cd /trial
exec python3 "/scene/$1.py" "$2" "$3"
