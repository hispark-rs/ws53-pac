# ws53-pac

Experimental WS53 application-core PAC. Register facts are owned by
[hisi-registers](https://github.com/hispark-rs/hisi-registers); generated Rust is
not edited by hand. The supported profile is `ws53_liteos_app`, MCU mode 1.
Only modeled peripherals are exposed; this is not a complete-chip claim.

Regenerate with `HISI_REGISTERS=/path/to/hisi-registers bash scripts/regen.sh`.
Requires svd2rust **0.37.1**, uv and the **nightly-2026-07-09** rustfmt. Ordinary Cargo builds need none of
the register-generation tools. `generation.json` records input hashes and the
source commit (including whether the source tree was dirty), formatter version,
and hashes of the generator recipe/settings as well as its outputs.
The recipe validates SVD against a pinned CMSIS schema and enables
`--max-cluster-size`: MDMA channels preserve the 0x20-byte stride, including
reserved tail padding. RX/TX FIFO views and interrupt-clear command views
intentionally restrict generated access APIs. These checks establish register
layout and API shape, not peripheral behavior or safe DMA buffer ownership.

`rt` supplies `device.x`. The custom interrupt controller is not a PLIC; IRQ
numbers are absolute HiSilicon local interrupt cause numbers. This PAC does not
initialize clocks, release the control core, or establish peripheral ownership.

## Standalone verification

The committed Cargo.lock belongs to this repository. From a standalone clone:

```sh
cargo test --locked --all-features --target x86_64-unknown-linux-gnu
cargo check -Zbuild-std=core --locked --all-features --target riscv32imfc-unknown-none-elf
cargo check -Zbuild-std=core --locked --no-default-features --target riscv32imfc-unknown-none-elf
cargo package --locked --target x86_64-unknown-linux-gnu
```

CI runs these checks and separately fetches the exact `generation.json` source
commit, regenerates with the fixed tools, and compares Rust/SVD/device.x and
provenance. Normal Cargo builds use committed Rust and do not fetch register
sources or require Python, the SDK, a ROM capture, or a connected board.

SDIO is a ROM-audited register subset, not an implemented SDIO driver. Its
command views deliberately omit read-modify-write and unverified clear/reset
semantics. PAC availability does not establish DMA quiescence, safe buffer
ownership, clock configuration, or completed silicon peripheral validation.

TRNG describes the SDK unified-security application path at `0x52024000`.
`FIFO_DATA` reads consume words; it is not a passive diagnostic register.
The SDK application also services control-core random-buffer requests over IPC.
Neither this PAC nor its `Peripherals` singleton grants exclusive ownership of
that service or its shared 16-bit ring control in `M_CRG`. Entropy quality,
handover, HAL behavior and hardware validation remain outstanding.

eFuse exposes a read-service subset: 256 logical bytes in 128 word-sized slots,
status, volatile timing and a separate read-mode command view. It has no OTP
data-write or programming-power interface. Read setup must preserve unmodeled
control bits and coordinate shared users. The SVD command constraint documents
`0x5a5a`; it is not a runtime guard on unsafe raw writes. SDK die-ID/MAC schemas
disagree, so this PAC does not select an identity interpretation or claim a
hardware-validated eFuse driver.

TIMER exposes two 32-bit channels and local raw/masked status. The SDK's common
structure conflicts with its offset comments; that window is intentionally
absent. EOI only exposes the SDK's write acknowledgement, not speculative read
behavior. TIMER/TCXO do not implement Resettable: inherited IP reset annotations
were not established for WS53. TCXO's four count slots are 32-bit accesses with
16-bit payloads. None of these layout checks proves shared-resource handover.
