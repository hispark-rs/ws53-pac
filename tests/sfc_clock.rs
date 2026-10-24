#![cfg(not(target_arch = "riscv32"))]

use core::mem::size_of_val;
use ws53_pac::m_crg;

fn offset<T, U>(base: &T, register: &U) -> usize {
    register as *const U as usize - base as *const T as usize
}

// Compilation fails if regeneration invents a reset value for either view.
macro_rules! no_reset {
    ($ty:ty) => {
        const _: fn() = || {
            trait Ambiguous<A> {
                fn check() {}
            }
            impl<T: ?Sized> Ambiguous<()> for T {}
            struct HasReset;
            impl<T: ?Sized + ws53_pac::generic::Resettable> Ambiguous<HasReset> for T {}
            let _ = <$ty as Ambiguous<_>>::check;
        };
    };
}
no_reset!(m_crg::xip_sfc_clock::XipSfcClockSpec);
no_reset!(m_crg::xip_sfc_clock_word::XipSfcClockWordSpec);

#[test]
fn sfc_clock_aliases_have_sdk_widths_and_distinct_qspi_neighbor() {
    fn halfword<T: ws53_pac::generic::RegisterSpec<Ux = u16>>() {}
    fn word<T: ws53_pac::generic::RegisterSpec<Ux = u32>>() {}
    halfword::<m_crg::xip_sfc_clock::XipSfcClockSpec>();
    word::<m_crg::xip_sfc_clock_word::XipSfcClockWordSpec>();

    // Zeroed local RAM is just storage for layout tests, not a silicon reset.
    let r: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    let half: &m_crg::XipSfcClock = r.xip_sfc_clock();
    let word: &m_crg::XipSfcClockWord = r.xip_sfc_clock_word();
    let qspi: &m_crg::XipQspiClock = r.xip_qspi_clock();
    assert_eq!(ws53_pac::MCrg::PTR as usize, 0x5200_0000);
    assert_eq!(offset(&r, half), 0x570);
    assert_eq!(offset(&r, word), 0x570);
    assert_eq!(size_of_val(half), 2);
    assert_eq!(size_of_val(word), 4);
    assert_eq!(offset(&r, qspi), 0x574);
    assert_eq!(size_of_val(qspi), 2);
}

#[test]
#[cfg(target_endian = "little")]
fn sfc_clock_fields_preserve_unmodeled_bits_in_both_views() {
    let r: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    // All raw three-bit codes exercise encoding only. The SDK establishes
    // writes of 0 and 6, not frequencies or legal operating modes for all codes.
    for seed in [
        0,
        u32::MAX,
        0xa55a_1357,
        0x5aa5_eca8,
        0x8000_0000,
        0x0000_fff0,
    ] {
        for source in 0..8u8 {
            for enabled in [false, true] {
                unsafe {
                    r.xip_qspi_clock().write_with_zero(|w| w.bits(0x5aa5));
                    r.xip_sfc_clock_word().write_with_zero(|w| w.bits(seed));
                    r.xip_sfc_clock_word()
                        .modify(|_, w| w.source_config().bits(source).enable().bit(enabled));
                }
                let expected = (seed & !0xf) | (u32::from(source) << 1) | u32::from(enabled);
                assert_eq!(r.xip_sfc_clock_word().read().bits(), expected);
                assert_eq!(r.xip_sfc_clock().read().bits(), expected as u16);
                assert_eq!(r.xip_sfc_clock().read().source_config().bits(), source);
                assert_eq!(r.xip_sfc_clock().read().enable().bit(), enabled);

                unsafe {
                    r.xip_sfc_clock_word().write_with_zero(|w| w.bits(seed));
                    r.xip_sfc_clock()
                        .modify(|_, w| w.source_config().bits(source).enable().bit(enabled));
                }
                assert_eq!(r.xip_sfc_clock_word().read().bits(), expected);
                assert_eq!(r.xip_sfc_clock_word().read().source_config().bits(), source);
                assert_eq!(r.xip_sfc_clock_word().read().enable().bit(), enabled);
                assert_eq!(r.xip_qspi_clock().read().bits(), 0x5aa5);
            }
        }
    }
}

#[test]
#[cfg(target_endian = "little")]
fn sdk_mixed_width_sequence_preserves_upper_bits_and_neighbor() {
    // SDK 163d9150 sfc_switch_clock()/boot_flash_clk_init(): word bit0 RMW,
    // halfword bits3:1 RMW, then word bit0 RMW. Delays and real hardware are
    // intentionally not simulated; this grants no live-switch/ownership API.
    let r: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    for seed in [0, u32::MAX, 0xa55a_1357, 0x5aa5_eca8] {
        for source in [0, 6] {
            unsafe {
                r.xip_qspi_clock().write_with_zero(|w| w.bits(0xa55a));
                r.xip_sfc_clock_word().write_with_zero(|w| w.bits(seed));
            }
            r.xip_sfc_clock_word().modify(|_, w| w.enable().clear_bit());
            assert_eq!(r.xip_sfc_clock_word().read().bits(), seed & !1);
            unsafe {
                r.xip_sfc_clock()
                    .modify(|_, w| w.source_config().bits(source))
            };
            assert_eq!(
                r.xip_sfc_clock_word().read().bits(),
                (seed & !0xf) | (u32::from(source) << 1)
            );
            r.xip_sfc_clock_word().modify(|_, w| w.enable().set_bit());
            assert_eq!(
                r.xip_sfc_clock_word().read().bits(),
                (seed & !0xf) | (u32::from(source) << 1) | 1
            );
            assert_eq!(r.xip_qspi_clock().read().bits(), 0xa55a);
        }
    }
}
