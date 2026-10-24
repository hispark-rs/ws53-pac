#![cfg(not(target_arch = "riscv32"))]

use ws53_pac::sdio;

// Fail compilation if regeneration invents read/write/reset capabilities.
macro_rules! absent {
    ($ty:ty, $bound:path) => {
        const _: fn() = || {
            trait Ambiguous<A> {
                fn check() {}
            }
            impl<T: ?Sized> Ambiguous<()> for T {}
            struct Present;
            impl<T: ?Sized + $bound> Ambiguous<Present> for T {}
            let _ = <$ty as Ambiguous<_>>::check;
        };
    };
}
absent!(
    sdio::function1_interrupt_ack::Function1InterruptAckSpec,
    ws53_pac::generic::Readable
);
absent!(
    sdio::function1_interrupt_status::Function1InterruptStatusSpec,
    ws53_pac::generic::Writable
);
absent!(
    sdio::function1_interrupt_ack::Function1InterruptAckSpec,
    ws53_pac::generic::Resettable
);
absent!(
    sdio::function1_interrupt_status::Function1InterruptStatusSpec,
    ws53_pac::generic::Resettable
);
absent!(
    sdio::function1_interrupt_enable::Function1InterruptEnableSpec,
    ws53_pac::generic::Resettable
);

macro_rules! event_bits {
    ($r:expr) => {{
        let r = $r;
        [
            r.host_write_complete().bit(),
            r.host_read_complete().bit(),
            r.host_read_error().bit(),
            r.host_read_start().bit(),
            r.host_write_start().bit(),
            r.adma_end().bit(),
            r.adma_interrupt().bit(),
            r.adma_error().bit(),
            r.function_reset().bit(),
            r.host_message().bit(),
            r.suspend().bit(),
            r.resume().bit(),
            r.device_message_ack().bit(),
            r.function_enable().bit(),
        ]
    }};
}

#[test]
fn status_events_decode_individually_and_keep_unclassified_bits() {
    // Local RAM is storage, not a silicon reset or a behavioral SDIO model.
    let r: sdio::RegisterBlock = unsafe { core::mem::zeroed() };
    for raw in (0..32)
        .map(|bit| 1u32 << bit)
        .chain([0, u32::MAX, 0xa55a_5aa5])
    {
        unsafe { r.function1_interrupt_status().as_ptr().write_volatile(raw) };
        let value = r.function1_interrupt_status().read();
        assert_eq!(value.bits(), raw);
        assert_eq!(value.unclassified().bits(), raw >> 14);
        for (bit, enabled) in event_bits!(value).into_iter().enumerate() {
            assert_eq!(enabled, raw & (1 << bit) != 0, "status bit {bit}");
        }
    }
}

#[test]
fn enable_fields_round_trip_all_event_masks_without_touching_upper_bits() {
    let r: sdio::RegisterBlock = unsafe { core::mem::zeroed() };
    for seed in [0, u32::MAX, 0xa55a_5aa5] {
        for mask in 0..=0x3fffu32 {
            unsafe {
                r.function1_interrupt_enable()
                    .write_with_zero(|w| w.bits(seed))
            };
            r.function1_interrupt_enable().modify(|_, w| {
                w.host_write_complete()
                    .bit(mask & 1 != 0)
                    .host_read_complete()
                    .bit(mask & 2 != 0)
                    .host_read_error()
                    .bit(mask & 4 != 0)
                    .host_read_start()
                    .bit(mask & 8 != 0)
                    .host_write_start()
                    .bit(mask & 16 != 0)
                    .adma_end()
                    .bit(mask & 32 != 0)
                    .adma_interrupt()
                    .bit(mask & 64 != 0)
                    .adma_error()
                    .bit(mask & 128 != 0)
                    .function_reset()
                    .bit(mask & 256 != 0)
                    .host_message()
                    .bit(mask & 512 != 0)
                    .suspend()
                    .bit(mask & 1024 != 0)
                    .resume()
                    .bit(mask & 2048 != 0)
                    .device_message_ack()
                    .bit(mask & 4096 != 0)
                    .function_enable()
                    .bit(mask & 8192 != 0)
            });
            let value = r.function1_interrupt_enable().read();
            assert_eq!(value.bits(), (seed & !0x3fff) | mask);
            for (bit, enabled) in event_bits!(value).into_iter().enumerate() {
                assert_eq!(enabled, mask & (1 << bit) != 0, "enable bit {bit}");
            }
        }
    }
}

#[test]
fn ack_stays_full_width_write_only_alias_without_assumed_clear_behavior() {
    let r: sdio::RegisterBlock = unsafe { core::mem::zeroed() };
    let offset = |p: *mut u32| p as usize - &r as *const _ as usize;
    assert_eq!(offset(r.function1_interrupt_status().as_ptr()), 0x110);
    assert_eq!(offset(r.function1_interrupt_ack().as_ptr()), 0x110);
    assert_eq!(offset(r.function1_interrupt_enable().as_ptr()), 0x114);
    for raw in [0, 1, 0x3fff, 0xffff_c000, u32::MAX] {
        unsafe {
            r.function1_interrupt_ack()
                .write_with_zero(|w| w.value().bits(raw))
        };
        // Read the aliased RAM view only. This intentionally does not simulate W1C.
        assert_eq!(r.function1_interrupt_status().read().bits(), raw);
    }
}

absent!(
    ws53_pac::glb_ctl::sdio_soft_reset::SdioSoftResetSpec,
    ws53_pac::generic::Resettable
);

#[test]
fn sdk_reset_patch_is_word_rmw_and_preserves_other_control_bits() {
    fn word<T: ws53_pac::generic::RegisterSpec<Ux = u32>>() {}
    word::<ws53_pac::glb_ctl::sdio_soft_reset::SdioSoftResetSpec>();
    let r: ws53_pac::glb_ctl::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::GlbCtl::PTR as usize, 0x5700_0000);
    assert_eq!(
        r.sdio_soft_reset().as_ptr() as usize - &r as *const _ as usize,
        0xc8
    );
    for seed in [0, u32::MAX, 0xa55a_5aa5, 0x5aa5_a55a] {
        unsafe {
            r.uart_soft_reset().write_with_zero(|w| w.bits(0x5aa5));
            r.sdio_soft_reset().write_with_zero(|w| w.bits(seed));
        }
        r.sdio_soft_reset()
            .modify(|_, w| w.sdio_reset_n().clear_bit());
        assert_eq!(r.sdio_soft_reset().read().bits(), seed & !2);
        assert!(!r.sdio_soft_reset().read().sdio_reset_n().bit());
        // The SDK delays here; local storage verifies encoding, not elapsed
        // time, hardware reset completion or in-flight ADMA transaction drain.
        r.sdio_soft_reset()
            .modify(|_, w| w.sdio_reset_n().set_bit());
        assert_eq!(r.sdio_soft_reset().read().bits(), seed | 2);
        assert!(r.sdio_soft_reset().read().sdio_reset_n().bit());
        assert_eq!(r.uart_soft_reset().read().bits(), 0x5aa5);
    }
}
