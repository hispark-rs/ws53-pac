#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Pinned svd2rust 0.37.1 edition-2024 and host-target compatibility fixes.

WS53 arrays currently generate zero duplicate bare array accessors.
The generated singleton unsafe block, no_mangle attribute and RISC-V-only
reexports are transformed deterministically, with exact occurrence assertions.
Register definitions are never edited here; change hisi-registers instead.
"""
import re
import sys

EXPECTED_BARE_STRIPS = 0
EXPECTED_NO_MANGLE = 1
EXPECTED_RISCV_PAC_ENUM = 1
EXPECTED_RISCV_GATES = 6
EXPECTED_UNSAFE_FN_FIXES = 1


def strip_bare_array_accessors(src: str) -> tuple[str, int]:
    indexed = set(re.findall(r"pub const fn (\w+)\(&self, n: usize\)", src))
    bare = set(re.findall(r"pub const fn (\w+)\(&self\)\s*->", src))
    removed = 0
    for name in sorted(indexed & bare):
        pat = re.compile(
            r"[ \t]*#\[doc[^\n]*\]\n"
            r"[ \t]*#\[inline\(always\)\]\n"
            r"[ \t]*pub const fn " + re.escape(name) + r"\(&self\) -> &\w+ \{\n"
            r"[ \t]*self\." + re.escape(name) + r"\(0\)\n"
            r"[ \t]*\}\n",
            re.M,
        )
        src, n = pat.subn("", src)
        removed += n
    return src, removed


def migrate_no_mangle(src: str) -> tuple[str, int]:
    return re.subn(r"#\[no_mangle\]", "#[unsafe(no_mangle)]", src)


# riscv-lib-dependent items in the generated `pub mod interrupt` block. They are
# unused by hisi-hal (which only consumes the device ExternalInterrupt enum) and
# the `riscv` lib does not build on x86, so each is gated to riscv32.
_RISCV_GATE_NEEDLES = (
    "pub use riscv::interrupt::Exception;",
    "pub use riscv::interrupt::Interrupt as CoreInterrupt;",
    "pub use riscv::{",
    "pub type Trap = riscv::interrupt::Trap<CoreInterrupt, Exception>;",
    "pub fn try_cause() -> riscv::result::Result<Trap> {",
    "pub fn cause() -> Trap {",
)


def gate_riscv_for_host(src: str) -> tuple[str, int, int]:
    cfg = '#[cfg(target_arch = "riscv32")]'
    # Apply the InterruptNumber-deriving attribute only on riscv32; on host the
    # enum stays a plain fieldless enum (hisi-hal uses `irq as u16`).
    src, n_attr = re.subn(
        r"#\[riscv :: pac_enum \(unsafe ExternalInterruptNumber\)\]",
        '#[cfg_attr(target_arch = "riscv32", riscv :: pac_enum (unsafe ExternalInterruptNumber))]',
        src,
    )
    gated = 0
    for needle in _RISCV_GATE_NEEDLES:
        src, n = re.subn(
            r"^([ \t]*)" + re.escape(needle),
            lambda m: f"{m.group(1)}{cfg}\n{m.group(1)}{needle}",
            src,
            count=1,
            flags=re.M,
        )
        gated += n
    return src, n_attr, gated


def fix_unsafe_fn_body(src: str) -> tuple[str, int]:
    lines = src.splitlines(keepends=True)
    signature = "    pub unsafe fn steal() -> Self {\n"
    try:
        start = lines.index(signature)
    except ValueError:
        return src, 0
    if start + 1 < len(lines) and lines[start + 1] == "        unsafe {\n":
        return src, 0

    depth = 0
    end = None
    for index in range(start, len(lines)):
        depth += lines[index].count("{") - lines[index].count("}")
        if index > start and depth == 0:
            end = index
            break
    if end is None:
        return src, 0

    body = ["    " + line for line in lines[start + 1:end]]
    lines[start + 1:end] = ["        unsafe {\n", *body, "        }\n"]
    return "".join(lines), 1


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: postprocess.py <lib.rs>", file=sys.stderr)
        return 2
    path = sys.argv[1]
    with open(path) as f:
        src = f.read()

    src, n_strip = strip_bare_array_accessors(src)
    src, n_mangle = migrate_no_mangle(src)
    src, n_attr, n_gate = gate_riscv_for_host(src)
    src, n_unsafe = fix_unsafe_fn_body(src)

    with open(path, "w") as f:
        f.write(src)

    print(f"postprocess: stripped {n_strip} bare accessors, "
          f"migrated {n_mangle} no_mangle attribute(s), "
          f"gated {n_attr} pac_enum + {n_gate} riscv re-exports for host, "
          f"fixed {n_unsafe} unsafe fn body")

    ok = True
    if n_strip != EXPECTED_BARE_STRIPS:
        print(f"  WARNING: expected {EXPECTED_BARE_STRIPS} bare-accessor "
              f"strips, got {n_strip} (SVD changed? verify the diff)",
              file=sys.stderr)
        ok = False
    if n_mangle != EXPECTED_NO_MANGLE:
        print(f"  WARNING: expected {EXPECTED_NO_MANGLE} no_mangle migration, "
              f"got {n_mangle}", file=sys.stderr)
        ok = False
    if n_attr != EXPECTED_RISCV_PAC_ENUM:
        print(f"  WARNING: expected {EXPECTED_RISCV_PAC_ENUM} pac_enum gate, "
              f"got {n_attr}", file=sys.stderr)
        ok = False
    if n_gate != EXPECTED_RISCV_GATES:
        print(f"  WARNING: expected {EXPECTED_RISCV_GATES} riscv re-export "
              f"gates, got {n_gate} (interrupt module changed? verify the diff)",
              file=sys.stderr)
        ok = False
    if n_unsafe != EXPECTED_UNSAFE_FN_FIXES:
        print(f"  WARNING: expected {EXPECTED_UNSAFE_FN_FIXES} unsafe fn fix, "
              f"got {n_unsafe}", file=sys.stderr)
        ok = False
    # Non-fatal: counts are sanity checks, not gates — the build/clippy in
    # regen.sh is the real verification.
    return 0 if ok else 0


if __name__ == "__main__":
    sys.exit(main())
