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
    clocks
        .m_clken1()
        .modify(|_, w| w.spi3_ms_enable().set_bit().lsadc_apb_enable().set_bit());
    assert_eq!(clocks.m_clken1().read().bits(), (1 << 11) | (1 << 2));
    aon.agpio6_pad()
        .modify(|_, w| unsafe { w.drive_high().bits(3).drive_low().bits(2).pull().bits(1) });
    assert_eq!(aon.agpio6_pad().read().bits(), 0xca);
}

#[test]
fn peripheral_singleton_is_not_reissued() {
    let _peripherals = ws53_pac::Peripherals::take().expect("first claim");
    assert!(ws53_pac::Peripherals::take().is_none());
}

// An inferred marker type is unambiguous only if the forbidden trait is absent.
// These are compile-time API checks: FIFO windows must never acquire modify().
macro_rules! assert_not_impl {
    ($ty:ty, $bound:path) => {
        const _: fn() = || {
            trait Ambiguous<A> {
                fn check() {}
            }
            impl<T: ?Sized> Ambiguous<()> for T {}
            struct HasTrait;
            impl<T: ?Sized + $bound> Ambiguous<HasTrait> for T {}
            let _ = <$ty as Ambiguous<_>>::check;
        };
    };
}
assert_not_impl!(ws53_pac::spi0::rx::RxSpec, ws53_pac::generic::Writable);
assert_not_impl!(ws53_pac::spi0::tx::TxSpec, ws53_pac::generic::Readable);
assert_not_impl!(
    ws53_pac::spi0::clear_all::ClearAllSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(ws53_pac::qspi1::rx::RxSpec, ws53_pac::generic::Writable);
assert_not_impl!(ws53_pac::qspi1::tx::TxSpec, ws53_pac::generic::Readable);
assert_not_impl!(m_crg::m_clken1::MClken1Spec, ws53_pac::generic::Resettable);
assert_not_impl!(
    pinmux_aon::agpio6_pad::Agpio6PadSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::mdma::interrupt_clear::InterruptClearSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::mdma::configuration::ConfigurationSpec,
    ws53_pac::generic::Resettable
);

#[test]
fn spi_fifo_views_keep_their_instance_layout_and_access_direction() {
    let spi: ws53_pac::spi0::RegisterBlock = unsafe { core::mem::zeroed() };
    let qspi: ws53_pac::qspi1::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(offset(&spi, spi.enable()), 0);
    assert_eq!(offset(&qspi, qspi.enable()), 8);
    for index in 0..36 {
        assert_eq!(offset(&spi, spi.rx(index)), 0x2c + index * 4);
        assert_eq!(offset(&spi, spi.tx(index)), 0x2c + index * 4);
        assert_eq!(offset(&qspi, qspi.rx(index)), 0x60 + index * 4);
        assert_eq!(offset(&qspi, qspi.tx(index)), 0x60 + index * 4);
    }
    assert_eq!(offset(&spi, spi.clear_all()), 0xf8);
    assert_eq!(offset(&qspi, qspi.clear_all()), 0x48);
    fn readable<T: ws53_pac::generic::Readable>() {}
    fn writable<T: ws53_pac::generic::Writable>() {}
    readable::<ws53_pac::spi0::rx::RxSpec>();
    writable::<ws53_pac::spi0::tx::TxSpec>();
    readable::<ws53_pac::qspi1::rx::RxSpec>();
    writable::<ws53_pac::qspi1::tx::TxSpec>();
}

#[test]
fn mdma_eight_channels_keep_padding_request_encoding_and_w1c_commands() {
    use ws53_pac::mdma;
    let dma: mdma::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(size_of::<mdma::channel::Channel>(), 0x20);
    assert_eq!(size_of::<mdma::RegisterBlock>(), 0x200);
    assert_eq!(offset(&dma, dma.enabled_channels()), 0x10);
    assert_eq!(offset(&dma, dma.configuration()), 0x1c);
    assert_eq!(dma.channel_iter().count(), 8);
    for index in 0..8 {
        let channel = dma.channel(index);
        assert_eq!(offset(&dma, channel), 0x100 + 0x20 * index);
        assert_eq!(offset(channel, channel.linked_list()), 0);
        assert_eq!(offset(channel, channel.destination()), 4);
        assert_eq!(offset(channel, channel.configuration()), 8);
        assert_eq!(offset(channel, channel.source()), 16);
        assert_eq!(offset(channel, channel.control()), 20);
        channel.configuration().modify(|_, w| {
            w.source_request()
                .uart0_rx()
                .destination_request()
                .spi0_tx()
        });
        assert_eq!(channel.configuration().read().bits(), (11 << 1) | (6 << 5));
    }
    assert_eq!(ws53_pac::interrupt::ExternalInterrupt::M_DMA as u16, 58);
    assert_eq!(
        <mdma::interrupt_clear::InterruptClearSpec as ws53_pac::generic::Writable>::ONE_TO_MODIFY_FIELDS_BITMAP,
        0xffff
    );
    // API/layout only; stack-backed cells do not emulate W1C hardware behavior.
    unsafe {
        dma.interrupt_clear()
            .write_with_zero(|w| w.transfer_complete().bits(1 << 7).error().bits(1 << 3));
        assert_eq!(dma.interrupt_clear().as_ptr().read_volatile(), 0x880);
    }
}
