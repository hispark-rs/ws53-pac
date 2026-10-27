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

`Spacc` describes the SDK's HASH subset of the shared controller at0x52010000,
with `SEC_INT1` local cause59. `hash_channel(0/1)` and `hash_state(0/1)` select
SDK channels1/2; `hash_error(0/1)` selects TEE/REE diagnostics. Clear, interrupt
acknowledgement, state index and DMA submission have WO interfaces; status and
pointer views are RO. No reset values or W1C behavior are assumed. Submission
is a full-word command because the SDK preserves unmodeled high pointer bits.

This PAC does not implement HASH algorithms or grant cross-core exclusivity,
keyslot ownership or DMA buffer reclamation. The audited SDK ignores channel
clear timeout before publishing descriptors and deinitializes channels0/1
instead of1/2; a driver must not inherit those paths. SYMC behavior and PKE/KM remain pending.
The PKE IRQ58 declaration conflicts with MDMA and is disabled in the selected
SDK application profile, so it is not exposed as a PKE interrupt here.

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
Function1 status and enable expose fourteen event bits audited against the
original ROM handler. Status bits 31:14 remain `unclassified`; the write-only
acknowledgement retains its full-word payload. No reset value or W1C behavior
is inferred from the ROM's write. Host tests exercise all 16,384 enable masks,
preserved upper bits and the split read/write API using local RAM only.
`GlbCtl::sdio_soft_reset()` describes the SDK application's reset patch at
`0x570000c8`: a word read-modify-write clears bit1, delays five microseconds,
then a fresh read-modify-write sets it. This differs from the empty ROM entry.
It proves neither activation of that patch in a board image nor ADMA drain,
reset coverage, pulse timing or safe buffer reclamation. Unknown reset values
are omitted and neighboring control bits must be preserved.

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

## SFC clock source views

`M_CRG::xip_sfc_clock()` and `xip_sfc_clock_word()` are the SDK's 16-bit and
32-bit access views of the same register at `0x52000570`. They expose bit0
`ENABLE` and the raw three-bit `SOURCE_CONFIG` field. The SDK writes source
codes 0 and 6; this establishes neither a complete frequency formula nor a
clock-ready acknowledgement. The existing QSPI view at `0x52000574` is distinct.

Unknown upper bits and reset values remain unmodeled, and neither SFC view
implements `Resettable`. Local RAM tests check alias layout and preservation
of unrelated bits under the SDK's mixed-width sequence. These are source/API
checks only: the PAC does not grant shared-clock ownership, safe live clock
switching, Flash execution safety, DMA drain or a usable SFC HAL.

## SDIO initialization source correction

The pinned RDL source now executes the original ROM base initializer against
ordinary RAM with36 initial states. Function0's final enable operand is bit11
(16 rotated right25), not bit16: the experimental `event16()` accessor is
replaced by `event11()`. The initializer profile uses named partial fields in
MODE, VOLTAGE_CONTROL, CONFIG_30 and FUNCTION1_CONFIG; unknown bits are preserved.

`M_CRG::m_clken1_word()` exposes the SDK patch's 32-bit SDIO AHB gate access,
while the original halfword accessor remains available. Host tests check both
clock word aliases, unrelated bits, and the corrected initialization masks.
These are generated API and source checks, not SDIO bus, IRQ, reset or DMA
completion tests. No reset defaults or buffer-lifetime guarantees are inferred.

## SYMC register extension

The shared `Spacc` now also provides two SYMC input/output rings, channel
configuration, IV readback and completion/acknowledgement views from the pinned
WS53 SDK. These are generated register interfaces, not an AES/SM4 HAL or key
loader. A keyslot selector does not grant ownership of the separate KM resource.
The original SDK's direct `set_key` operation always fails.

`symc_channel(0/1)` and `symc_output(0/1)` correspond to SDK channels1/2. Input
descriptors are32 bytes, output descriptors16 bytes; their depth registers are
at different offsets. Submission/clear/ACK are WO commands with RO read views;
no unknown reset values, read side effects or W1C behavior are fabricated.

SYMC/HASH diagnostics interleave by security domain. The existing `hash_error`
container starts8 bytes earlier to include `symc_cause`/`symc_status`; the original
`cause`/`status` register accessors retain their exact absolute addresses and
the array stride remains16 bytes. Code deriving addresses from the container
base itself must be reviewed. All existing HASH API tests remain applicable.

The SDK releases ownership even on a clear timeout and publishes output before
descriptor-cache maintenance. Neither source sequence proves bus quiescence or
safe resource reclamation. This PAC does not implement those lifecycle choices.

## KM keyslot and clear-key loader

The same isolated integration now supplies one `Km` token at`0x52022000`,
covering25 source-audited register views,46 fields, a four-word `data_in` array
and application IRQ`SEC_INT3=61`. Generation is pinned to the clean source in
`generation.json`; ordinary Cargo users require neither Python nor a local SDK.

Shared selectors, working-key configuration, loader and slots belong to this
one resource. Command/ACK views are write-only, errors/status are read-only,
and unknown reset defaults are absent. The original interrupt C union is8 bytes
because of an overlong reserved field; its actual access and this PAC are32-bit.
There is no root derivation, OTP, permanent-policy or PCPU/AIDSP command API.

The source driver allocates8 MCIPHER slots and2 HMAC slots; field width is not
capacity. Source unlock omits the timeout-error bit and an owner reread, while
clear-loader timeout still clears input words. PAC access does not prove slot
erasure, completion, cross-core exclusion or safe resource handback. Future HAL
code must validate ownership, algorithm/length and error recovery explicitly.
This remains a register interface, not an encryption/HMAC implementation.

## Shared control-core logic/clock release

`SystemMemory::ccore_logic_clock_release_command()` is a full-word write-only
alias of the existing read-only `ccore_reset_status()` at `0x5703096c`.
The `logic_clock_release` field describes only value3: logic and clocks released
while the control CPU stays held. No reset defaults, readable command, RMW or
generic CPU-release/reset helper are generated.

The source SDK waits5us after this command; a future HAL must also establish
exclusive handoff, require exact state0, wait and check state3. This is shared
domain control, not a TSENSOR-only clock. The diagnostic attempt to reset the
domain again stalled debug access, so dropping a software token must not imply
a safe reset/rollback. PAC access alone establishes none of those conditions.

SVD's exact3 range is metadata: unsafe raw writes can bypass it. Host tests
check the32-bit alias, full-word encoding and absence of `Readable`/`Resettable`
on the command and `Writable`/`Resettable` on status. They are ordinary RAM tests,
not hardware startup, delay or rollback qualification.
The [validation report](evidence/ccore-domain-validation.json) records two
byte-identical generations,60 host tests and offline RV32IMFC default/`rt`
builds, tied to the clean RDL commit and SDK contract.

The IPC increment follows the pinned RDL/ROM/patch contract: two application
receive IRQs26/27, word status/ACK, distinct discard-only ACK readback, and a
halfword send command replacing the ROM word store. Host tests verify layout,
field encoding, preserved neighboring bits and intentionally absent read/RMW/
reset traits. They do not send an IPC message or prove coherence, doorbell
completion, firmware peer ownership or safe shared-memory access. No live-peer
reset API is added. The header's shared-memory defaults must not be substituted
for the control-image-selected layout documented by hisi-registers.
