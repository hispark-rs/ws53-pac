#![cfg(not(target_arch = "riscv32"))]

use ws53_pac::km;

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
macro_rules! capability {
    ($bound:path; $($ty:ty),+ $(,)?) => { $(absent!($ty, $bound);)+ };
}
capability!(ws53_pac::generic::Resettable;
    km::data_in::DataInSpec, km::key_address::KeyAddressSpec,
    km::key_config::KeyConfigSpec, km::key_security::KeySecuritySpec,
    km::loader_error::LoaderErrorSpec, km::controller_error::ControllerErrorSpec,
    km::interrupt_status::InterruptStatusSpec, km::interrupt_ack::InterruptAckSpec,
    km::common_lock::CommonLockSpec, km::common_unlock::CommonUnlockSpec,
    km::common_lock_info::CommonLockInfoSpec, km::common_owner::CommonOwnerSpec,
    km::clear_control::ClearControlSpec, km::clear_submit::ClearSubmitSpec,
    km::tee_slot_read::TeeSlotReadSpec, km::tee_slot_command::TeeSlotCommandSpec,
    km::ree_slot_read::ReeSlotReadSpec, km::ree_slot_command::ReeSlotCommandSpec,
    km::tee_flush_status::TeeFlushStatusSpec, km::ree_flush_status::ReeFlushStatusSpec,
    km::slot_select::SlotSelectSpec, km::slot_owner::SlotOwnerSpec,
);
capability!(ws53_pac::generic::Readable;
    km::data_in::DataInSpec, km::interrupt_ack::InterruptAckSpec,
    km::common_lock::CommonLockSpec, km::common_unlock::CommonUnlockSpec,
    km::clear_submit::ClearSubmitSpec, km::tee_slot_command::TeeSlotCommandSpec,
    km::ree_slot_command::ReeSlotCommandSpec,
);
capability!(ws53_pac::generic::Writable;
    km::loader_error::LoaderErrorSpec, km::controller_error::ControllerErrorSpec,
    km::interrupt_status::InterruptStatusSpec, km::common_lock_info::CommonLockInfoSpec,
    km::common_owner::CommonOwnerSpec, km::clear_control::ClearControlSpec,
    km::tee_slot_read::TeeSlotReadSpec, km::ree_slot_read::ReeSlotReadSpec,
    km::tee_flush_status::TeeFlushStatusSpec, km::ree_flush_status::ReeFlushStatusSpec,
    km::slot_owner::SlotOwnerSpec,
);

// Ordinary zeroed host storage only: none of these tests emulate KM silicon.
fn storage() -> km::RegisterBlock {
    unsafe { core::mem::zeroed() }
}

#[test]
fn one_shared_resource_and_application_irq() {
    assert_eq!(ws53_pac::Km::PTR as usize, 0x5202_2000);
    assert_eq!(ws53_pac::interrupt::ExternalInterrupt::SEC_INT3 as u16, 61);
    assert!(include_str!("../device.x").contains("PROVIDE(SEC_INT3 = DefaultHandler);"));
    let peripherals = ws53_pac::Peripherals::take().expect("first claim");
    let _: ws53_pac::Km = peripherals.km;
    assert!(ws53_pac::Peripherals::take().is_none());
}

#[test]
fn offsets_array_stride_and_command_aliases_match_original_headers() {
    let r = storage();
    let offset = |p: *mut u32| p as usize - &r as *const _ as usize;
    assert_eq!(core::mem::size_of_val(&r), 0x1b38);
    assert_eq!(r.data_in_iter().count(), 4);
    for (i, word) in r.data_in_iter().enumerate() {
        assert_eq!(offset(word.as_ptr()), 0x1000 + i * 4);
    }
    for (p, address) in [
        (r.key_address().as_ptr(), 0x1010),
        (r.key_config().as_ptr(), 0x1014),
        (r.key_security().as_ptr(), 0x1018),
        (r.loader_error().as_ptr(), 0x1038),
        (r.controller_error().as_ptr(), 0x103c),
        (r.interrupt_status().as_ptr(), 0x1044),
        (r.interrupt_ack().as_ptr(), 0x1044),
        (r.common_lock().as_ptr(), 0x1074),
        (r.common_unlock().as_ptr(), 0x1078),
        (r.common_lock_info().as_ptr(), 0x107c),
        (r.common_owner().as_ptr(), 0x1080),
        (r.clear_control().as_ptr(), 0x1438),
        (r.clear_submit().as_ptr(), 0x1438),
        (r.tee_slot_read().as_ptr(), 0x1b00),
        (r.tee_slot_command().as_ptr(), 0x1b00),
        (r.ree_slot_read().as_ptr(), 0x1b04),
        (r.ree_slot_command().as_ptr(), 0x1b04),
        (r.tee_flush_status().as_ptr(), 0x1b10),
        (r.ree_flush_status().as_ptr(), 0x1b14),
        (r.slot_select().as_ptr(), 0x1b30),
        (r.slot_owner().as_ptr(), 0x1b34),
    ] {
        assert_eq!(offset(p), address);
    }
    assert_eq!(core::mem::size_of::<km::InterruptStatus>(), 4);
    assert_eq!(core::mem::size_of::<km::InterruptAck>(), 4);
}

#[test]
fn working_key_and_slot_fields_preserve_unmodeled_bits() {
    let r = storage();
    for seed in [0, u32::MAX, 0xa5a5_5a5a, 0x1234_5678] {
        for engine in [0x20, 0x50, 0xa0, 0xa1, 0xa2] {
            for flags in 0..4u32 {
                unsafe {
                    r.key_config().write_with_zero(|w| w.bits(seed));
                }
                r.key_config().modify(|_, w| unsafe {
                    w.port()
                        .bits(1)
                        .engine()
                        .bits(engine)
                        .encrypt()
                        .bit(flags & 1 != 0)
                        .decrypt()
                        .bit(flags & 2 != 0)
                        .flash_selection()
                        .bits(0)
                });
                let value = r.key_config().read();
                assert_eq!(
                    value.bits(),
                    (seed & !0x000f_0ff7) | 1 | (u32::from(engine) << 4) | (flags << 16)
                );
                assert_eq!(value.engine().bits(), engine);
            }
        }
        for slot in [0, 1, 7, 511, 1023] {
            for hmac in [false, true] {
                unsafe {
                    r.slot_select().write_with_zero(|w| w.bits(seed));
                }
                r.slot_select()
                    .modify(|_, w| unsafe { w.slot().bits(slot).hmac().bit(hmac) });
                assert_eq!(
                    r.slot_select().read().bits(),
                    (seed & !0x83ff) | u32::from(slot) | (u32::from(hmac) << 15)
                );
                unsafe {
                    r.key_address().write_with_zero(|w| w.bits(seed));
                }
                r.key_address()
                    .modify(|_, w| unsafe { w.address().bits(slot) });
                assert_eq!(
                    r.key_address().read().bits(),
                    (seed & !1023) | u32::from(slot)
                );
            }
        }
        for mask in 0..64u32 {
            unsafe {
                r.key_security().write_with_zero(|w| w.bits(seed));
            }
            r.key_security().modify(|_, w| {
                w.secure_key()
                    .bit(mask & 1 != 0)
                    .source_nonsecure()
                    .bit(mask & 2 != 0)
                    .source_secure()
                    .bit(mask & 4 != 0)
                    .destination_nonsecure()
                    .bit(mask & 8 != 0)
                    .destination_secure()
                    .bit(mask & 16 != 0)
                    .same_master_only()
                    .bit(mask & 32 != 0)
            });
            assert_eq!(r.key_security().read().bits(), (seed & !63) | mask);
        }
    }
}

#[test]
fn command_words_have_no_automatic_w1c_or_64_bit_access() {
    let r = storage();
    assert_eq!(<km::interrupt_ack::InterruptAckSpec as ws53_pac::generic::Writable>::ONE_TO_MODIFY_FIELDS_BITMAP, 0);
    assert_eq!(<km::clear_submit::ClearSubmitSpec as ws53_pac::generic::Writable>::ONE_TO_MODIFY_FIELDS_BITMAP, 0);
    for seed in [0, u32::MAX, 0x1122_3344] {
        unsafe {
            r.interrupt_ack().write_with_zero(|w| w.bits(seed));
            r.common_lock().write_with_zero(|w| w.bits(1));
            r.common_unlock().write_with_zero(|w| w.bits(1));
            r.clear_submit().write_with_zero(|w| w.bits(seed));
            r.tee_slot_command().write_with_zero(|w| w.bits(seed));
            r.ree_slot_command().write_with_zero(|w| w.bits(!seed));
        }
        assert_eq!(r.interrupt_status().read().bits(), seed);
        assert_eq!(r.clear_control().read().bits(), seed);
        assert_eq!(r.tee_slot_read().read().bits(), seed);
        assert_eq!(r.ree_slot_read().read().bits(), !seed);
        assert_eq!(
            r.interrupt_status().read().clear_route().bit(),
            seed & (1 << 14) != 0
        );
        for (i, data) in r.data_in_iter().enumerate() {
            unsafe {
                data.write_with_zero(|w| w.value().bits(seed ^ i as u32));
            }
            assert_eq!(unsafe { data.as_ptr().read_volatile() }, seed ^ i as u32);
        }
    }
}

#[test]
fn diagnostic_reads_keep_timeout_and_owner_information() {
    let r = storage();
    for raw in 0..8u32 {
        unsafe {
            r.ree_flush_status().as_ptr().write_volatile(raw);
            r.tee_flush_status().as_ptr().write_volatile(raw);
            r.slot_owner().as_ptr().write_volatile(raw);
        }
        let status = r.ree_flush_status().read();
        assert_eq!(status.busy().bit(), raw & 1 != 0);
        assert_eq!(status.unlock_failure().bit(), raw & 2 != 0);
        assert_eq!(status.timeout_error().bit(), raw & 4 != 0);
        assert_eq!(
            r.tee_flush_status().read().timeout_error().bit(),
            raw & 4 != 0
        );
        assert_eq!(r.slot_owner().read().owner().bits(), raw as u8);
    }
    for raw in [0, 0x8000_0000, u32::MAX] {
        unsafe {
            r.loader_error().as_ptr().write_volatile(raw);
            r.controller_error().as_ptr().write_volatile(!raw);
        }
        assert_eq!(r.loader_error().read().value().bits(), raw);
        assert_eq!(r.controller_error().read().value().bits(), !raw);
    }
}
