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

`M_CRG::msub_normal_word()` and `msub_sleep_word()` expose the SDK's
32-bit read-modify-write views at offsets 0x3e0 and 0x3e4. The existing
halfword accessors remain available for API compatibility. The source proves
19 named fields, not reset values or the validity of halfword accesses on
silicon; unmodeled bits must be preserved. These shared gates do not establish
SDMA availability, idle status, transaction drain, or exclusive ownership.

The WS53-only SFC/FAMA status views and `SystemMemory::ccore_reset_status()`
support read-only Flash handoff diagnostics. Their presence does not provide a
Flash erase/program driver, permit mapping changes, or prove a cold boot.

RTC0/1 expose the SDK v150 channel layout with local causes 49/50. Snapshot-ready
and interrupt status are read-only, acknowledgement is a write-one command, and
unknown reset values are omitted. The SDK header's read-clear comment is not
enough to expose an EOI read API. The reference application uses 32-bit counters;
the high-word slots do not establish permission to use 64-bit mode or the shared
common-bank unlock path. No board crystal, clock setup, IRQ behavior or ROM
implementation is validated by this PAC. The SDK source's start/stop delays and
ambiguous timeout/zero result must be handled in the integration driver.

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

UART0/1/2 use the SDK's 32-bit access profile. RX and TX are separate views of
the data address, so neither exposes modify(). Reading LINE_STATUS clears some
errors; polling FIFO_STATUS.rx_not_empty avoids that side effect. The WS63
receive-count read alias at +0x40 is not established on WS53: that address only
exposes the SDK's WO test-injection view. Unknown resets and PRO extensions are
not imported. Full UART configuration, DMA and hardware validation remain open.

UART port reset controls are halfword RMW fields: H0/H1 at M_CRG +0x168 and
L0 at GLB_CTL +0xb0. Their reset values and pulse timing are unknown; the SDK
skips its debug UART. GLB_CTL +0x48 is a restricted WO command view of the SDK's
full-word acore-only IRQ route value 1, with no speculative read API. These are
shared controls, not application-exclusive resources. Conflicting H1/L0 clock
gate enums are omitted, so the PAC does not claim a complete UART clock tree.

GPIO has three basic banks with 23/12/3 bit spans, not three equal groups of
bonded pins. Direction/type/polarity/dual-edge use per-pin `pins()` bitmaps;
the value 1 is never presented as a whole-bank input mode. SW_CTL is opaque:
its source-selection encoding is not established. EOI/SET/CLR have WO command
interfaces, and no GPIO reset values are assumed. Pin aliases, GPIO0 holes and
the SGPIO/SWD reservation must be enforced by the future HAL. LOCK, low-power
state and MULTISYSTEM register semantics are not claimed by this PAC.

WDT uses an SDK-audited application view of the shared v151 register layout.
LOCK command/status and counter-latch request/status are separate WO/RO views;
restart specifies the actual SDK key `0x5a5a5a5a`. STATUS bit0 is 0 free /
1 busy, and EOI has an explicit read side effect. No WDT reset value is
assumed. These corrections do not prove watchdog expiry, lock protection range,
clock rate, NMI delivery or recovery. The SDK has conflicting NMI_CWDT bit
assignments; the application WDT is not assigned the control-core B_WDT IRQ.
This PAC does not itself provide a watchdog HAL or hardware reset qualification.

For the pinned `ws53_liteos_app` source configuration, the SDK target resolver
selects `non_os`, excluding the old ROM-header components. Its application
watchdog source is NMI bit1. M_CRG exposes halfword NMI enable/raw/masked status
at offsets 0x700/0x704/0x708; status is RO and reset values are unknown. This
resolves the source-profile header choice, not CPU NMI semantics or the board's
installed firmware. The PAC deliberately adds no guessed local WDT interrupt.

PWM exposes the application profile's four groups/eight channels at 0x52082000,
with local causes 71/72. All 115 register slots have source-layout evidence;
unknown resets and command self-clear claims are withheld. START and the three
event-clear words have no read/modify interface. INT_MASK is an event enable:
named Disabled/AbnormalOnly/PeriodEndOnly/AllEvents values encode 0/1/2/3. The
halfword PWM_M_DIV3 at M_CRG+0x7c describes enable/load/divider fields from the
source port, not a measured 32MHz clock or exclusive ownership of shared clocks.
The source's 255-tick period and 32767-cycle API limits are not hardware-width
claims; high count halves remain source-defined 16-bit slots. Register presence
does not establish usable pins, shadow-latch timing, pulse waveforms or a PWM HAL.

I2C0/1 use the audited non-PRO v151 source view: 36 register views each,
IRQs62/63, separate RX-pop and TX-command accesses at0x1c, RO status and abort
causes, and WO clear commands with unknown reset values. `IC_CLR_INT` accepts
the source write-one clear-all operation. Selective `IC_CLR_INTR` positions
remain opaque: the original C union exceeds32 bits and its helpers disagree
with status positions. Header read-clear comments conflict with the executed
write helpers; the PAC deliberately offers no speculative clear-register read
API. Neither these layout tests nor source count formulas validate physical
I2C timings, Fast-plus routing, pin ownership, cancellation or a bus transaction.

SPI0/QSPI1 retain distinct register and field encodings. The optional application
SPI port's missing QSPI clock chain is now modeled as halfword XIP_CLOCK_ENABLE
(+0x1240), nine-bit XIP_QSPI_CLOCK.CONTROL (+0x574), and GLOBAL_CLKEN bit7.
Source helpers set/clear divider-enable, divider-clock and peripheral-clock
bits1/3/5 while preserving other XIP users' fields. This is not ownership of the
XIP path or proof of the upstream frequency, reset state or board wiring.
Baseline SDK SPI is disabled; the compiled source oracle enables it only on
host RAM. Clock-divider documentation records even2..65534 (zero disables),
while the SDK's frequency helper can write odd/truncated values. No physical
clock measurement or stable SPI/QSPI driver is claimed here.

I2S now uses the WS53 SDK's audited layout instead of the imported family SIO
map. All 25 word slots/70 fields have C compiler checks, including MODE bits,
TX/RX enables at12/13, RX width at5:3, six interrupt causes and split/merged
FIFOs. TX/RX sample-width variants encode1..5 for16/18/20/24/32 bits, checked
against the original C enum; zero is reserved and excluded by write constraints.
RX data reads consume FIFO entries; TX, interrupt-clear and CT_SET/CLR
are write-only views. The latter have no claimed readback/zero-write semantics.
Unknown reset values are omitted. These checks do not establish clock handoff,
drain completion or wire timing. `M_CLKEN0_WORD` now gives a 32-bit access view
of the same low-half fields as the unchanged 16-bit `M_CLKEN0`; it is the same
resource and preserves unknown upper bits on RMW. `AUDIO_CMU.NEW_CFG0` describes
only the shared audio divider reset-release bit0. Neither view has a reset value.
Source macros and local PAC RAM tests preserve neighboring/upper bits, but
actual clock/reset ownership and frequency still require a handoff. This PAC
correction alone is not a usable I2S HAL or a claim of silicon support.

UART FIFO status follows three-port A-board evidence, not the contradicted SDK
comments: bit2 is RX full, bit3 enabled-FIFO empty, and bit4 observed busy while
RX is queued. `DIV_EN` set attempts with queued RX do not latch. See source
`evidence/ws53-uart-access-hil.json`; TX-only busy, disabled FIFO semantics,
nonzero high divisor and busy divisor writes remain unqualified.

## SFC command source contract update

The generated SFC view now follows the original WS53 SDK command helper
contract recorded by `hisi-registers` commit `a5f81ca`. Seven DDR/sample fields
are available as source-confirmed layout; their operation is not qualified.
CMD_CONFIG.START, BUS_DMA_CTRL.DMA_START and the 16 command-data words are
documented as hardware-updated, with no invented reset defaults.

This intentionally removes the previously imported `INT_RAW_STATUS`,
`INT_STATUS`, `INT_MASK`, `INT_CLEAR` and `SOFT_RST_MASK` registers from WS53:
the pinned SDK supplies neither their layout nor an application-core SFC IRQ
route. Downstream experimental code using those accessors must be reviewed;
there is no replacement interrupt or reset API. Other chips are unaffected.

START clearing describes the controller's transfer state. It does not certify
NOR WIP, DMA buffer ownership or bus drain. DATA words may contain a previous
response; a fresh status observation requires a new command and completion.
The new host tests check generated layout and encodings in local memory only.
Ordinary Cargo builds still consume checked-in code and need no SDK or generator.
