#![cfg(not(target_arch = "riscv32"))]

use ws53_pac::spacc::{self, hash_channel, hash_error, hash_state};

// These checks fail to compile if generation invents access/reset capabilities.
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
macro_rules! no_reset {
    ($($ty:ty),+ $(,)?) => { $(absent!($ty, ws53_pac::generic::Resettable);)+ };
}
no_reset!(
    spacc::interrupt_enable::InterruptEnableSpec,
    spacc::hash_clear_status::HashClearStatusSpec,
    spacc::hash_clear_ack::HashClearAckSpec,
    spacc::hash_lock::HashLockSpec,
    spacc::hash_clear_request::HashClearRequestSpec,
    spacc::hash_interrupt_status::HashInterruptStatusSpec,
    spacc::hash_interrupt_ack::HashInterruptAckSpec,
    spacc::hash_interrupt_enable::HashInterruptEnableSpec,
    hash_channel::control::ControlSpec,
    hash_channel::key_control::KeyControlSpec,
    hash_channel::node_base_high::NodeBaseHighSpec,
    hash_channel::node_base_low::NodeBaseLowSpec,
    hash_channel::node_depth::NodeDepthSpec,
    hash_channel::write_pointer::WritePointerSpec,
    hash_channel::submit::SubmitSpec,
    hash_channel::read_pointer::ReadPointerSpec,
    hash_channel::data_length::DataLengthSpec,
    hash_error::cause::CauseSpec,
    hash_error::status::StatusSpec,
    hash_state::data::DataSpec,
    hash_state::index::IndexSpec,
);
macro_rules! write_only {
    ($($ty:ty),+ $(,)?) => { $(absent!($ty, ws53_pac::generic::Readable);)+ };
}
write_only!(
    spacc::hash_clear_ack::HashClearAckSpec,
    spacc::hash_clear_request::HashClearRequestSpec,
    spacc::hash_interrupt_ack::HashInterruptAckSpec,
    hash_channel::submit::SubmitSpec,
    hash_state::index::IndexSpec,
);
macro_rules! read_only {
    ($($ty:ty),+ $(,)?) => { $(absent!($ty, ws53_pac::generic::Writable);)+ };
}
read_only!(
    spacc::hash_clear_status::HashClearStatusSpec,
    spacc::hash_interrupt_status::HashInterruptStatusSpec,
    hash_channel::write_pointer::WritePointerSpec,
    hash_channel::read_pointer::ReadPointerSpec,
    hash_channel::data_length::DataLengthSpec,
    hash_error::cause::CauseSpec,
    hash_error::status::StatusSpec,
);

// Zero is valid local Rust storage, not a claim about any silicon reset value.
fn storage() -> spacc::RegisterBlock {
    unsafe { core::mem::zeroed() }
}

#[test]
fn shared_block_irq_arrays_and_command_aliases_match_source() {
    assert_eq!(ws53_pac::Spacc::PTR as usize, 0x5201_0000);
    assert_eq!(ws53_pac::interrupt::ExternalInterrupt::SEC_INT1 as u16, 59);
    assert!(include_str!("../device.x").contains("PROVIDE(SEC_INT1 = DefaultHandler)"));
    let r = storage();
    let offset = |p: *mut u32| p as usize - &r as *const _ as usize;
    for (p, expected) in [
        (r.interrupt_enable().as_ptr(), 4),
        (r.hash_clear_status().as_ptr(), 0x14),
        (r.hash_clear_ack().as_ptr(), 0x14),
        (r.hash_lock().as_ptr(), 0x40),
        (r.hash_clear_request().as_ptr(), 0x68),
        (r.hash_interrupt_status().as_ptr(), 0x8600),
        (r.hash_interrupt_ack().as_ptr(), 0x8600),
        (r.hash_interrupt_enable().as_ptr(), 0x8604),
    ] {
        assert_eq!(offset(p), expected);
    }
    assert_eq!(r.hash_channel_iter().count(), 2);
    assert_eq!(r.hash_error_iter().count(), 2);
    assert_eq!(r.hash_state_iter().count(), 2);
    assert_eq!(core::mem::size_of_val(r.hash_channel(0)), 0x100);
    assert_eq!(core::mem::size_of_val(r.hash_error(0)), 0x10);
    assert_eq!(core::mem::size_of_val(r.hash_state(0)), 0x80);
    for i in 0..2 {
        let ch = r.hash_channel(i);
        for (p, relative) in [
            (ch.control().as_ptr(), 0),
            (ch.key_control().as_ptr(), 0x10),
            (ch.node_base_high().as_ptr(), 0x20),
            (ch.node_base_low().as_ptr(), 0x24),
            (ch.node_depth().as_ptr(), 0x2c),
            (ch.write_pointer().as_ptr(), 0x30),
            (ch.submit().as_ptr(), 0x30),
            (ch.read_pointer().as_ptr(), 0x34),
            (ch.data_length().as_ptr(), 0xc0),
        ] {
            assert_eq!(offset(p), 0x5100 + i * 0x100 + relative);
        }
        assert_eq!(offset(r.hash_error(i).cause().as_ptr()), 0x8068 + i * 0x10);
        assert_eq!(offset(r.hash_error(i).status().as_ptr()), 0x806c + i * 0x10);
        assert_eq!(offset(r.hash_state(i).data().as_ptr()), 0xa080 + i * 0x80);
        assert_eq!(offset(r.hash_state(i).index().as_ptr()), 0xa084 + i * 0x80);
    }
}

#[test]
fn owners_and_interrupt_gates_preserve_unmodeled_channels() {
    let r = storage();
    for seed in [0, u32::MAX, 0xa55a_5aa5, 0x5aa5_a55a] {
        for owner1 in 0..16u8 {
            for owner2 in 0..16u8 {
                unsafe { r.hash_lock().write_with_zero(|w| w.bits(seed)) };
                r.hash_lock().modify(|_, w| unsafe {
                    w.channel1_owner()
                        .bits(owner1)
                        .channel2_owner()
                        .bits(owner2)
                });
                let read = r.hash_lock().read();
                assert_eq!(
                    read.bits(),
                    (seed & !0xff0) | ((owner1 as u32) << 4) | ((owner2 as u32) << 8)
                );
                assert_eq!(read.channel1_owner().bits(), owner1);
                assert_eq!(read.channel2_owner().bits(), owner2);
            }
        }
        for mask in 0..8 {
            unsafe { r.interrupt_enable().write_with_zero(|w| w.bits(seed)) };
            r.interrupt_enable().modify(|_, w| {
                w.ree()
                    .bit(mask & 1 != 0)
                    .tee()
                    .bit(mask & 2 != 0)
                    .hpp()
                    .bit(mask & 4 != 0)
            });
            assert_eq!(
                r.interrupt_enable().read().bits(),
                (seed & !0x111) | (mask & 1) | ((mask & 2) << 3) | ((mask & 4) << 6)
            );
        }
        for mask in 0..4 {
            unsafe { r.hash_interrupt_enable().write_with_zero(|w| w.bits(seed)) };
            r.hash_interrupt_enable().modify(|_, w| {
                w.channel1()
                    .bit(mask & 1 != 0)
                    .channel2()
                    .bit(mask & 2 != 0)
            });
            assert_eq!(
                r.hash_interrupt_enable().read().bits(),
                (seed & !6) | (mask << 1)
            );
        }
    }
}

#[test]
fn acknowledgements_are_direct_writes_without_implicit_modify_semantics() {
    use ws53_pac::generic::Writable;
    assert_eq!(
        <spacc::hash_interrupt_ack::HashInterruptAckSpec as Writable>::ONE_TO_MODIFY_FIELDS_BITMAP,
        0
    );
    assert_eq!(
        <spacc::hash_clear_ack::HashClearAckSpec as Writable>::ONE_TO_MODIFY_FIELDS_BITMAP,
        0
    );
    let r = storage();
    for mask in 0..4 {
        unsafe {
            r.hash_interrupt_status().as_ptr().write_volatile(u32::MAX);
            r.hash_clear_status().as_ptr().write_volatile(u32::MAX);
            r.hash_interrupt_ack().write_with_zero(|w| {
                w.channel1()
                    .bit(mask & 1 != 0)
                    .channel2()
                    .bit(mask & 2 != 0)
            });
            r.hash_clear_ack().write_with_zero(|w| {
                w.channel1()
                    .bit(mask & 1 != 0)
                    .channel2()
                    .bit(mask & 2 != 0)
            });
            r.hash_clear_request().write_with_zero(|w| {
                w.channel1()
                    .bit(mask & 1 != 0)
                    .channel2()
                    .bit(mask & 2 != 0)
            });
            assert_eq!(r.hash_clear_request().as_ptr().read_volatile(), mask << 1);
        }
        // Aliased host RAM records the written command; this is NOT a W1C model.
        let raw = r.hash_interrupt_status().read();
        assert_eq!(raw.bits(), mask << 1);
        assert_eq!(raw.channel1().bit(), mask & 1 != 0);
        assert_eq!(raw.channel2().bit(), mask & 2 != 0);
        assert_eq!(r.hash_clear_status().read().bits(), mask << 1);
    }
}

#[test]
fn channel_fields_and_submission_preserve_distinct_views() {
    let r = storage();
    for i in 0..2 {
        let ch = r.hash_channel(i);
        for seed in [0, u32::MAX, 0xa55a_5aa5] {
            unsafe { ch.control().write_with_zero(|w| w.bits(seed)) };
            ch.control()
                .modify(|_, w| unsafe { w.security().bits(10).enable().set_bit() });
            assert_eq!(
                ch.control().read().bits(),
                (seed & !0x8000_0f00) | 0x8000_0a00
            );
            unsafe { ch.key_control().write_with_zero(|w| w.bits(seed)) };
            ch.key_control().modify(|_, w| unsafe {
                w.keyslot()
                    .bits(7)
                    .hmac()
                    .set_bit()
                    .algorithm()
                    .bits(11)
                    .mode()
                    .bits(1)
            });
            assert_eq!(
                ch.key_control().read().bits(),
                (seed & !0x00ff_020f) | 0x001b_0207
            );
            for pointer in 0..=255u32 {
                let submitted = (seed & !255) | pointer;
                unsafe { ch.submit().write_with_zero(|w| w.value().bits(submitted)) };
                assert_eq!(ch.write_pointer().read().pointer().bits(), pointer as u8);
                assert_eq!(ch.write_pointer().read().bits(), submitted);
            }
        }
    }
}

#[test]
fn source_error_bits_decode_independently_without_a_clear_writer() {
    let r = storage();
    for i in 0..2 {
        for bit in 0..32 {
            unsafe { r.hash_error(i).cause().as_ptr().write_volatile(1 << bit) };
            let e = r.hash_error(i).cause().read();
            for (position, set) in [
                (1, e.security().bit()),
                (2, e.algorithm().bit()),
                (3, e.invalid_algorithm().bit()),
                (6, e.sc().bit()),
                (13, e.sm3_disabled().bit()),
                (14, e.sha1_disabled().bit()),
                (17, e.tee_only().bit()),
                (18, e.aidsp().bit()),
            ] {
                assert_eq!(set, bit == position);
            }
        }
        for channel in 0..16u32 {
            unsafe {
                r.hash_error(i)
                    .status()
                    .as_ptr()
                    .write_volatile((channel << 16) | 0xfff0_ffff)
            };
            assert_eq!(
                r.hash_error(i).status().read().channel().bits(),
                channel as u8
            );
        }
    }
}
