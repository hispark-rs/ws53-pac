#![cfg(not(target_arch = "riscv32"))]

use core::mem::{size_of, size_of_val};
use ws53_pac::{m_crg, pinmux_aon, pinmux_mcu, pinmux_tzpc};

fn offset<B, R>(base: &B, register: &R) -> usize {
    register as *const R as usize - base as *const B as usize
}

#[test]
fn generated_pin_domains_and_halfword_clocks_match_sdk_offsets() {
    // Generated blocks contain only integer register cells and byte padding.
    // Zero is a valid Rust representation; these local objects do not claim
    // silicon reset values and never access the peripheral physical addresses.
    let clocks: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    let mcu: pinmux_mcu::RegisterBlock = unsafe { core::mem::zeroed() };
    let aon: pinmux_aon::RegisterBlock = unsafe { core::mem::zeroed() };
    let tzpc: pinmux_tzpc::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(offset(&clocks, clocks.m_clken1()), 0x44);
    assert_eq!(size_of_val(clocks.m_clken1()), 2);
    assert_eq!(offset(&mcu, mcu.mgpio14_sel()), 0x38);
    assert_eq!(offset(&tzpc, tzpc.mgpio14_pad()), 0x170);
    assert_eq!(offset(&aon, aon.agpio6_sel()), 0xb8);
    assert_eq!(offset(&aon, aon.agpio6_pad()), 0x44);
    assert_eq!(offset(&aon, aon.agpio12_pad()), 0x20);
    assert_eq!(size_of_val(aon.agpio6_pad()), 4);
    assert_eq!(size_of::<pinmux_mcu::RegisterBlock>(), 0x5c);
    clocks.m_clken1().modify(|_, w| w.spi3_ms_enable().set_bit().lsadc_apb_enable().set_bit());
    assert_eq!(clocks.m_clken1().read().bits(), (1 << 11) | (1 << 2));
    aon.agpio6_pad().modify(|_, w| unsafe { w.drive_high().bits(3).drive_low().bits(2).pull().bits(1) });
    assert_eq!(aon.agpio6_pad().read().bits(), 0xca);
}

#[test]
fn peripheral_singleton_is_not_reissued() {
    let _peripherals = ws53_pac::Peripherals::take().expect("first claim");
    assert!(ws53_pac::Peripherals::take().is_none());
}
