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

#[test]
fn uart_reset_controls_preserve_other_halfword_bits() {
    let m: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    let g: ws53_pac::glb_ctl::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::GlbCtl::PTR as usize, 0x57000000);
    assert_eq!(offset(&m, m.uart_soft_reset()), 0x168);
    assert_eq!(offset(&g, g.uart_soft_reset()), 0xb0);
    assert_eq!(size_of_val(m.uart_soft_reset()), 2);
    assert_eq!(size_of_val(g.uart_soft_reset()), 2);
    assert_eq!(offset(&g, g.uart_l0_irq_route_command()), 0x48);
    assert_eq!(size_of_val(g.uart_l0_irq_route_command()), 4);
    // Local RAM only: these are field/mask tests, not hardware reset experiments.
    unsafe {
        m.uart_soft_reset().write_with_zero(|w| w.bits(0xa5a5));
        g.uart_soft_reset().write_with_zero(|w| w.bits(0x5a5a));
    }
    m.uart_soft_reset()
        .modify(|_, w| w.uart_h0_reset_n().clear_bit().uart_h1_reset_n().set_bit());
    g.uart_soft_reset()
        .modify(|_, w| w.uart_l0_reset_n().set_bit());
    assert_eq!(m.uart_soft_reset().read().bits(), 0xa5a6);
    assert_eq!(g.uart_soft_reset().read().bits(), 0x5ada);
}

#[test]
fn gpio_banks_use_word_slots_and_per_pin_bitmaps() {
    macro_rules! bank {
        ($module:ident, $width:expr) => {{
            let r: ws53_pac::$module::RegisterBlock = unsafe { core::mem::zeroed() };
            assert_eq!(offset(&r, r.gpio_sw_ctl()), 8);
            assert_eq!(offset(&r, r.gpio_int_eoi()), 0x2c);
            assert_eq!(offset(&r, r.gpio_data_set()), 0x30);
            assert_eq!(offset(&r, r.gpio_data_clr()), 0x34);
            assert_eq!(size_of_val(r.gpio_sw_oen()), 4);
            let top = 1 << ($width - 1);
            // Local-memory field encoding only; no hardware access or reset claim.
            r.gpio_sw_oen().modify(|_, w| unsafe { w.pins().bits(top) });
            r.gpio_int_type()
                .modify(|_, w| unsafe { w.pins().bits(top) });
            r.gpio_int_polarity()
                .modify(|_, w| unsafe { w.pins().bits(top) });
            r.gpio_int_dedge()
                .modify(|_, w| unsafe { w.pins().bits(top) });
            assert_eq!(r.gpio_sw_oen().read().pins().bits(), top);
            assert_eq!(r.gpio_int_type().read().pins().bits(), top);
            assert_eq!(r.gpio_int_polarity().read().pins().bits(), top);
            assert_eq!(r.gpio_int_dedge().read().pins().bits(), top);
        }};
    }
    bank!(gpio0, 23);
    bank!(gpio1, 12);
    bank!(gpio2, 3);
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
macro_rules! gpio_command_api {
    ($module:ident) => {
        assert_not_impl!(
            ws53_pac::$module::gpio_int_eoi::GpioIntEoiSpec,
            ws53_pac::generic::Readable
        );
        assert_not_impl!(
            ws53_pac::$module::gpio_data_set::GpioDataSetSpec,
            ws53_pac::generic::Readable
        );
        assert_not_impl!(
            ws53_pac::$module::gpio_data_clr::GpioDataClrSpec,
            ws53_pac::generic::Readable
        );
        assert_not_impl!(
            ws53_pac::$module::gpio_sw_oen::GpioSwOenSpec,
            ws53_pac::generic::Resettable
        );
    };
}
gpio_command_api!(gpio0);
gpio_command_api!(gpio1);
gpio_command_api!(gpio2);
assert_not_impl!(
    ws53_pac::glb_ctl::uart_l0_irq_route_command::UartL0IrqRouteCommandSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::glb_ctl::uart_l0_irq_route_command::UartL0IrqRouteCommandSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::glb_ctl::uart_soft_reset::UartSoftResetSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::m_crg::uart_soft_reset::UartSoftResetSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(ws53_pac::spi0::rx::RxSpec, ws53_pac::generic::Writable);
assert_not_impl!(ws53_pac::spi0::tx::TxSpec, ws53_pac::generic::Readable);
assert_not_impl!(
    ws53_pac::spi0::clear_all::ClearAllSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(ws53_pac::qspi1::rx::RxSpec, ws53_pac::generic::Writable);
assert_not_impl!(ws53_pac::qspi1::tx::TxSpec, ws53_pac::generic::Readable);
assert_not_impl!(
    ws53_pac::trng::fifo_data::FifoDataSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::trng::fifo_data::FifoDataSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::trng::fifo_ready::FifoReadySpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::m_crg::trng_ring_control::TrngRingControlSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(m_crg::m_clken1::MClken1Spec, ws53_pac::generic::Resettable);
assert_not_impl!(ws53_pac::uart0::rx::RxSpec, ws53_pac::generic::Writable);
assert_not_impl!(ws53_pac::uart0::tx::TxSpec, ws53_pac::generic::Readable);
assert_not_impl!(ws53_pac::uart0::tx::TxSpec, ws53_pac::generic::Resettable);
assert_not_impl!(
    ws53_pac::uart0::fifo_ctl::FifoCtlSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::uart0::rx_fifo_write::RxFifoWriteSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::timer::timer_load_count0::TimerLoadCount0Spec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::timer::timer_control0::TimerControl0Spec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::timer::timer_eoi0::TimerEoi0Spec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::timer::timer_eoi0::TimerEoi0Spec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::timer::timer_masked_intr0::TimerMaskedIntr0Spec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::tcxo::tcxo_status::TcxoStatusSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::tcxo::tcxo_count0::TcxoCount0Spec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::efuse::data_read::DataReadSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::efuse::data_read::DataReadSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::efuse::mode_status::ModeStatusSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::efuse::read_mode_command::ReadModeCommandSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::efuse::read_mode_command::ReadModeCommandSpec,
    ws53_pac::generic::Resettable
);
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
assert_not_impl!(
    ws53_pac::adc::scan_command::ScanCommandSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::adc::offset_state_clear::OffsetStateClearSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::adc::cap_finish_clear::CapFinishClearSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::adc::analog_config::AnalogConfigSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::adc::offset_status::OffsetStatusSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::adc::scan_control::ScanControlSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::tsensor::auto_status::AutoStatusSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::tsensor::start::StartSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::tsensor::control::ControlSpec,
    ws53_pac::generic::Resettable
);
assert_not_impl!(
    ws53_pac::sdio::global_interrupt_ack::GlobalInterruptAckSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::sdio::global_interrupt_status::GlobalInterruptStatusSpec,
    ws53_pac::generic::Writable
);
assert_not_impl!(
    ws53_pac::sdio::function0_interrupt_ack::Function0InterruptAckSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::sdio::function1_interrupt_ack::Function1InterruptAckSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::sdio::function0_reset_command::Function0ResetCommandSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::sdio::function1_send_command::Function1SendCommandSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::sdio::adma_table::AdmaTableSpec,
    ws53_pac::generic::Readable
);
assert_not_impl!(
    ws53_pac::sdio::global_interrupt_enable::GlobalInterruptEnableSpec,
    ws53_pac::generic::Resettable
);

#[test]
fn uart_word_views_preserve_data_directions_and_receive_flags() {
    let uart: ws53_pac::uart0::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(size_of_val(&uart), 0x64);
    assert_eq!(size_of_val(uart.rx()), 4);
    assert_eq!(size_of_val(uart.tx()), 4);
    assert_eq!(offset(&uart, uart.rx()), 4);
    assert_eq!(offset(&uart, uart.tx()), 4);
    assert_eq!(offset(&uart, uart.rx_fifo_cnt()), 0x4c);
    for raw in 0..16 {
        unsafe { uart.fifo_status().as_ptr().write_volatile(raw) };
        let status = uart.fifo_status().read();
        assert_eq!(status.tx_fifo_full().bit(), raw & 1 != 0);
        assert_eq!(status.tx_fifo_empty().bit(), raw & 2 != 0);
        assert_eq!(status.rx_not_empty().bit(), raw & 4 != 0);
        assert_eq!(status.rx_full().bit(), raw & 8 != 0);
    }
    unsafe {
        uart.rx_fifo_cnt().as_ptr().write_volatile(64);
        uart.tx().write_with_zero(|w| w.data().bits(0xa5));
    }
    assert_eq!(uart.rx_fifo_cnt().read().rx_fifo_level().bits(), 64);
    // Local memory checks encoding only: physical RX and TX are separate FIFOs.
    assert_eq!(unsafe { uart.tx().as_ptr().read_volatile() }, 0xa5);
}

#[test]
fn timer_channel_status_and_tcxo_words_match_selected_sdk_profile() {
    let timer: ws53_pac::timer::RegisterBlock = unsafe { core::mem::zeroed() };
    let tcxo: ws53_pac::tcxo::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::Timer::ptr() as usize, 0x52002000);
    assert_eq!(size_of_val(&timer), 0x220);
    assert_eq!(offset(&timer, timer.timer_load_count0()), 0x100);
    assert_eq!(offset(&timer, timer.timer_control0()), 0x110);
    assert_eq!(offset(&timer, timer.timer_eoi0()), 0x114);
    assert_eq!(offset(&timer, timer.timer_masked_intr0()), 0x11c);
    assert_eq!(offset(&timer, timer.timer_load_count1()), 0x200);
    assert_eq!(offset(&timer, timer.timer_control1()), 0x210);
    assert_eq!(offset(&timer, timer.timer_eoi1()), 0x214);
    assert_eq!(offset(&timer, timer.timer_masked_intr1()), 0x21c);
    // Local storage verifies field placement and preservation, not hardware
    // initial values, acknowledgement side effects or counting behavior.
    unsafe { timer.timer_control0().as_ptr().write_volatile(0xa5a50000) };
    timer
        .timer_control0()
        .modify(|_, w| w.mode().periodic().int_mask().set_bit());
    assert_eq!(timer.timer_control0().read().bits(), 0xa5a5000a);
    assert_eq!(ws53_pac::Tcxo::ptr() as usize, 0x57000200);
    assert_eq!(size_of_val(&tcxo), 0x14);
    assert_eq!(offset(&tcxo, tcxo.tcxo_count0()), 4);
    assert_eq!(offset(&tcxo, tcxo.tcxo_count1()), 8);
    assert_eq!(offset(&tcxo, tcxo.tcxo_count2()), 12);
    assert_eq!(offset(&tcxo, tcxo.tcxo_count3()), 16);
    assert_eq!(size_of_val(tcxo.tcxo_count0()), 4);
    unsafe {
        tcxo.tcxo_count0().as_ptr().write_volatile(0xffff1234);
        tcxo.tcxo_count1().as_ptr().write_volatile(0xffff5678);
        tcxo.tcxo_count2().as_ptr().write_volatile(0xffff9abc);
        tcxo.tcxo_count3().as_ptr().write_volatile(0xffffdef0);
    }
    let count = (u64::from(tcxo.tcxo_count3().read().count3().bits()) << 48)
        | (u64::from(tcxo.tcxo_count2().read().count2().bits()) << 32)
        | (u64::from(tcxo.tcxo_count1().read().count1().bits()) << 16)
        | u64::from(tcxo.tcxo_count0().read().count0().bits());
    assert_eq!(count, 0xdef09abc56781234);
}

#[test]
fn sdio_device_layout_keeps_command_views_and_cmd53_fields() {
    let sdio: ws53_pac::sdio::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::Sdio::PTR as usize, 0x52060000);
    assert_eq!(ws53_pac::interrupt::ExternalInterrupt::SDIO as u16, 46);
    assert_eq!(size_of::<ws53_pac::sdio::RegisterBlock>(), 0x25c);
    assert_eq!(offset(&sdio, sdio.global_interrupt_ack()), 0x1c);
    assert_eq!(offset(&sdio, sdio.global_interrupt_status()), 0x1c);
    assert_eq!(offset(&sdio, sdio.function0_interrupt_ack()), 0x38);
    assert_eq!(offset(&sdio, sdio.function0_interrupt_status()), 0x38);
    assert_eq!(offset(&sdio, sdio.function0_reset_command()), 0x3c);
    assert_eq!(offset(&sdio, sdio.function0_reset_status()), 0x3c);
    assert_eq!(offset(&sdio, sdio.adma_table()), 0x40);
    assert_eq!(offset(&sdio, sdio.host_clock()), 0x44);
    assert_eq!(offset(&sdio, sdio.function1_send_command()), 0x104);
    assert_eq!(offset(&sdio, sdio.function1_send_status()), 0x104);
    assert_eq!(offset(&sdio, sdio.function1_interrupt_ack()), 0x110);
    assert_eq!(offset(&sdio, sdio.function1_interrupt_status()), 0x110);
    assert_eq!(offset(&sdio, sdio.cmd53_argument()), 0x254);
    assert_eq!(offset(&sdio, sdio.function1_block_size()), 0x258);
    // Stack-backed read injection: verifies protocol bit extraction only.
    for count in [0, 1, 511] {
        for address in [0, 0x1ffff] {
            let raw = (1 << 31) | (7 << 28) | (1 << 27) | (1 << 26) | (address << 9) | count;
            unsafe { sdio.cmd53_argument().as_ptr().write_volatile(raw) };
            let arg = sdio.cmd53_argument().read();
            assert_eq!(arg.count().bits(), count as u16);
            assert_eq!(arg.address().bits(), address);
            assert_eq!(arg.function().bits(), 7);
            assert!(arg.write().bit() && arg.block_mode().bit() && arg.increment().bit());
        }
    }
    unsafe {
        sdio.function1_block_size()
            .as_ptr()
            .write_volatile(0xffff_ffff)
    };
    assert_eq!(sdio.function1_block_size().read().bytes().bits(), 0xfff);
}

#[test]
fn temperature_status_decodes_ten_bit_code_without_a_clear_write_api() {
    let sensor: ws53_pac::tsensor::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::Tsensor::PTR as usize, 0x59006300);
    assert_eq!(offset(&sensor, sensor.start()), 0);
    assert_eq!(offset(&sensor, sensor.control()), 4);
    assert_eq!(offset(&sensor, sensor.auto_status()), 8);
    assert_eq!(offset(&sensor, sensor.sample_time()), 0x10);
    assert_eq!(offset(&sensor, sensor.auto_refresh_period()), 0x30);
    assert_eq!(offset(&sensor, sensor.auto_refresh_config()), 0x34);
    assert_eq!(size_of::<ws53_pac::tsensor::RegisterBlock>(), 0x38);
    // Inject values into a stack-backed cell, not into the hardware RO register.
    for code in [0, 118, 933, 1023] {
        for ready in [false, true] {
            unsafe {
                sensor
                    .auto_status()
                    .as_ptr()
                    .write_volatile(0xffff_f001 | (code << 2) | ((ready as u32) << 1));
            }
            let status = sensor.auto_status().read();
            assert_eq!(status.code().bits(), code as u16);
            assert_eq!(status.ready().bit(), ready);
        }
    }
}

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

#[test]
fn adc_v155_uses_diagnostic_capture_and_explicit_pulses() {
    use ws53_pac::{adc, mcu_diag};
    let adc: adc::RegisterBlock = unsafe { core::mem::zeroed() };
    let diag: mcu_diag::RegisterBlock = unsafe { core::mem::zeroed() };
    let clocks: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(offset(&adc, adc.scan_command()), 0x1c);
    assert_eq!(offset(&adc, adc.analog_control()), 0x24);
    assert_eq!(offset(&adc, adc.manual_control()), 0x28);
    assert_eq!(offset(&adc, adc.offset_status()), 0x44);
    assert_eq!(offset(&adc, adc.cap_calibration()), 0x88);
    assert_eq!(offset(&adc, adc.cap_status()), 0xa0);
    for index in 0..3 {
        assert_eq!(offset(&adc, adc.analog_config(index)), 0xf4 + 4 * index);
    }
    assert_eq!(offset(&diag, diag.sample_mode()), 0x104);
    assert_eq!(offset(&diag, diag.start_low()), 0x110);
    assert_eq!(offset(&diag, diag.end_high()), 0x11c);
    assert_eq!(offset(&diag, diag.done_address_low()), 0x120);
    assert_eq!(offset(&diag, diag.sample_done()), 0x128);
    assert_eq!(offset(&clocks, clocks.soft_reset1()), 0x54);
    assert_eq!(size_of_val(clocks.soft_reset1()), 4);
    assert_eq!(size_of_val(clocks.m_clken1()), 2);
    adc.scan_control().modify(|_, w| unsafe {
        w.channels()
            .bits(0x80)
            .average()
            .eight()
            .sample_count()
            .bits(6)
            .start_count()
            .bits(0x1a)
            .discard_count()
            .bits(0x20)
    });
    assert_eq!(
        adc.scan_control().read().bits(),
        0x80 | (3 << 8) | (6 << 10) | (0x1a << 15) | (0x20 << 23)
    );
    diag.sample_mode()
        .modify(|_, w| w.mode().repeat().enable().set_bit().sync().set_bit());
    assert_eq!(diag.sample_mode().read().bits(), 0x301);
    assert_eq!(ws53_pac::interrupt::ExternalInterrupt::LSADC as u16, 80);
    // SDK requires explicit high/low writes, not a W1C/self-clearing assumption.
    assert_eq!(<adc::offset_state_clear::OffsetStateClearSpec as ws53_pac::generic::Writable>::ONE_TO_MODIFY_FIELDS_BITMAP, 0);
    unsafe {
        adc.offset_state_clear()
            .write_with_zero(|w| w.pulse().set_bit());
        assert_eq!(adc.offset_state_clear().as_ptr().read_volatile(), 1);
        adc.offset_state_clear()
            .write_with_zero(|w| w.pulse().clear_bit());
        assert_eq!(adc.offset_state_clear().as_ptr().read_volatile(), 0);
    }
}

#[test]
fn trng_uses_unified_address_and_shared_halfword_ring_control() {
    use ws53_pac::trng;
    let trng: trng::RegisterBlock = unsafe { core::mem::zeroed() };
    let clocks: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::Trng::ptr() as usize, 0x52024000);
    assert_eq!(offset(&trng, trng.fifo_data()), 0x100);
    assert_eq!(offset(&trng, trng.fifo_ready()), 0x104);
    assert_eq!(offset(&trng, trng.fro_sample_clk_sel()), 0x174);
    assert_eq!(offset(&trng, trng.fro_div_cnt()), 0x178);
    assert_eq!(offset(&clocks, clocks.trng_ring_control()), 0x1600);
    assert_eq!(size_of_val(clocks.trng_ring_control()), 2);
    // Local cells test generated field placement only. They do not emulate
    // FIFO consumption, ring startup, health tests or hardware entropy.
    for ready in 0..4 {
        unsafe { trng.fifo_ready().as_ptr().write_volatile(ready) };
        let value = trng.fifo_ready().read();
        assert_eq!(value.data_ready().bit(), ready & 1 != 0);
        assert_eq!(value.done().bit(), ready & 2 != 0);
    }
    unsafe { clocks.trng_ring_control().as_ptr().write_volatile(0xa5a4) };
    clocks
        .trng_ring_control()
        .modify(|_, w| w.ring_enable().set_bit());
    assert_eq!(clocks.trng_ring_control().read().bits(), 0xa5a5);
    unsafe { trng.fro_div_cnt().write_with_zero(|w| w.value().bits(0x1b)) };
    assert_eq!(trng.fro_div_cnt().read().bits(), 0x1b);
}

#[test]
fn efuse_read_window_preserves_two_bytes_per_word_and_command_alias() {
    use ws53_pac::efuse;
    // Stack storage only: no OTP reads, mode changes or programming on silicon.
    let efuse: efuse::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::Efuse::ptr() as usize, 0x57024000);
    assert_eq!(offset(&efuse, efuse.boot_status()), 0x2c);
    assert_eq!(offset(&efuse, efuse.mode_status()), 0x30);
    assert_eq!(offset(&efuse, efuse.read_mode_command()), 0x30);
    assert_eq!(offset(&efuse, efuse.clock_period()), 0x34);
    assert_eq!(efuse.data_read_iter().count(), 128);
    assert_eq!(size_of::<efuse::RegisterBlock>(), 0xa00);
    for (index, slot) in efuse.data_read_iter().enumerate() {
        assert_eq!(offset(&efuse, slot), 0x800 + index * 4);
        assert_eq!(size_of_val(slot), 4);
        let even = 2 * index as u32;
        unsafe {
            slot.as_ptr()
                .write_volatile(0xffff0000 | ((even + 1) << 8) | even)
        };
        assert_eq!(slot.read().even_byte().bits(), even as u8);
        assert_eq!(slot.read().odd_byte().bits(), (even + 1) as u8);
    }
    // The physical mode setter preserves unmodeled upper bits. The separate
    // command view has no modify(); a driver must coordinate the whole sequence.
    unsafe { efuse.mode_status().as_ptr().write_volatile(0xabcd1234) };
    let upper = efuse.mode_status().read().bits() & 0xffff0000;
    unsafe {
        efuse
            .read_mode_command()
            .write_with_zero(|w| w.bits(upper).read_mode().bits(0x5a5a))
    };
    assert_eq!(efuse.mode_status().read().bits(), 0xabcd5a5a);
}
