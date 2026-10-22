#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Record exact inputs, including uncommitted RDL changes, without timestamps."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

source, root = map(Path, sys.argv[1:])
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

inputs = sorted((source / "rdl").rglob("*.rdl")) + [
    source / path for path in ("scripts/export_svd.py", "scripts/check_schema.py",
                              "schema/CMSIS-SVD-1.3.9.xsd", "pyproject.toml", "uv.lock")
]
recipe = ["scripts/regen.sh", "scripts/postprocess.py", "scripts/provenance.py",
          "ws53-settings.yaml", "rust-toolchain.toml"]
record = {
    "source": "https://github.com/hispark-rs/hisi-registers",
    "commit": subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip(),
    "dirty": bool(subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"], text=True).strip()),
    "profile": "ws53_liteos_app/acore/MCU_MODE_1",
    "svd2rust": "0.37.1",
    "rust_toolchain": "nightly-2026-07-09",
    "rustfmt": subprocess.check_output(["rustup", "run", "nightly-2026-07-09", "rustfmt", "--version"], text=True).strip(),
    "target": "riscv",
    "inputs": {str(p.relative_to(source)): digest(p) for p in inputs},
    "generator_inputs": {p: digest(root / p) for p in recipe},
    "outputs": {p: digest(root / p) for p in ["src/lib.rs", "device.x", "svd/WS53.svd"]},
}
(root / "generation.json").write_text(json.dumps(record, indent=2) + "\n")
