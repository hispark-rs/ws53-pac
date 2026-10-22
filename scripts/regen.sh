#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="${HISI_REGISTERS:-$ROOT/hisi-registers}"
SVD2RUST="${SVD2RUST:-svd2rust}"
RUST_TOOLCHAIN="nightly-2026-07-09"
[ "$($SVD2RUST --version | awk '{print $2}')" = 0.37.1 ] || { echo 'svd2rust 0.37.1 required' >&2; exit 1; }
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
uv run --locked --project "$SOURCE" "$SOURCE/scripts/export_svd.py" "$SOURCE/rdl/chips/ws53.rdl" "$WORK/WS53.svd"
(cd "$WORK" && "$SVD2RUST" -i WS53.svd --target riscv --settings "$ROOT/ws53-settings.yaml")
rustup run "$RUST_TOOLCHAIN" rustfmt --edition 2024 "$WORK/lib.rs"
uv run --script "$ROOT/scripts/postprocess.py" "$WORK/lib.rs"
rustup run "$RUST_TOOLCHAIN" rustfmt --edition 2024 "$WORK/lib.rs"
mkdir -p "$ROOT/src" "$ROOT/svd"
cp "$WORK/lib.rs" "$ROOT/src/lib.rs"
cp "$WORK/WS53.svd" "$ROOT/svd/WS53.svd"
cp "$WORK/device.x" "$ROOT/device.x"
uv run --script "$ROOT/scripts/provenance.py" "$SOURCE" "$ROOT"
