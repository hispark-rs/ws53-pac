#![cfg(not(target_arch = "riscv32"))]

use ws53_pac::spacc::{self, hash_error, symc_channel, symc_iv, symc_output};

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
    spacc::symc_clear_status::SymcClearStatusSpec,
    spacc::symc_clear_ack::SymcClearAckSpec,
    spacc::symc_clear_read::SymcClearReadSpec,
    spacc::symc_clear_request::SymcClearRequestSpec,
    spacc::symc_lock::SymcLockSpec,
    spacc::symc_input_interrupt_status::SymcInputInterruptStatusSpec,
    spacc::symc_input_interrupt_ack::SymcInputInterruptAckSpec,
    spacc::symc_input_interrupt_enable::SymcInputInterruptEnableSpec,
    spacc::symc_output_interrupt_status::SymcOutputInterruptStatusSpec,
    spacc::symc_output_interrupt_ack::SymcOutputInterruptAckSpec,
    spacc::symc_output_interrupt_enable::SymcOutputInterruptEnableSpec,
    symc_channel::control::ControlSpec, symc_channel::output_control::OutputControlSpec,
    symc_channel::key_control::KeyControlSpec, symc_channel::node_base_high::NodeBaseHighSpec,
    symc_channel::node_base_low::NodeBaseLowSpec, symc_channel::node_depth::NodeDepthSpec,
    symc_channel::write_pointer::WritePointerSpec, symc_channel::submit::SubmitSpec,
    symc_channel::read_pointer::ReadPointerSpec,
    symc_output::node_base_high::NodeBaseHighSpec, symc_output::node_base_low::NodeBaseLowSpec,
    symc_output::node_depth::NodeDepthSpec, symc_output::write_pointer::WritePointerSpec,
    symc_output::submit::SubmitSpec, symc_output::read_pointer::ReadPointerSpec,
    symc_iv::word0::Word0Spec, symc_iv::word1::Word1Spec, symc_iv::word2::Word2Spec,
    symc_iv::word3::Word3Spec, hash_error::symc_cause::SymcCauseSpec,
    hash_error::symc_status::SymcStatusSpec,
);
capability!(ws53_pac::generic::Readable;
    spacc::symc_clear_ack::SymcClearAckSpec,
    spacc::symc_clear_request::SymcClearRequestSpec,
    spacc::symc_input_interrupt_ack::SymcInputInterruptAckSpec,
    spacc::symc_output_interrupt_ack::SymcOutputInterruptAckSpec,
    symc_channel::submit::SubmitSpec, symc_output::submit::SubmitSpec,
);
capability!(ws53_pac::generic::Writable;
    spacc::symc_clear_status::SymcClearStatusSpec,
    spacc::symc_clear_read::SymcClearReadSpec,
    spacc::symc_input_interrupt_status::SymcInputInterruptStatusSpec,
    spacc::symc_output_interrupt_status::SymcOutputInterruptStatusSpec,
    symc_channel::write_pointer::WritePointerSpec, symc_channel::read_pointer::ReadPointerSpec,
    symc_output::write_pointer::WritePointerSpec, symc_output::read_pointer::ReadPointerSpec,
    symc_iv::word0::Word0Spec, symc_iv::word1::Word1Spec,
    symc_iv::word2::Word2Spec, symc_iv::word3::Word3Spec,
    hash_error::symc_cause::SymcCauseSpec, hash_error::symc_status::SymcStatusSpec,
);

// These are local ordinary-memory layout/API tests, not device/reset emulation.
fn storage() -> spacc::RegisterBlock {
    unsafe { core::mem::zeroed() }
}

#[test]
fn source_offsets_and_input_output_ring_strides_are_distinct() {
    let r = storage();
    let offset = |p: *mut u32| p as usize - &r as *const _ as usize;
    for (p, address) in [
        (r.symc_clear_status().as_ptr(), 8),
        (r.symc_clear_ack().as_ptr(), 8),
        (r.symc_lock().as_ptr(), 0x20),
        (r.symc_clear_read().as_ptr(), 0x60),
        (r.symc_clear_request().as_ptr(), 0x60),
        (r.symc_input_interrupt_status().as_ptr(), 0x8610),
        (r.symc_input_interrupt_ack().as_ptr(), 0x8610),
        (r.symc_input_interrupt_enable().as_ptr(), 0x8614),
        (r.symc_output_interrupt_status().as_ptr(), 0xc000),
        (r.symc_output_interrupt_ack().as_ptr(), 0xc000),
        (r.symc_output_interrupt_enable().as_ptr(), 0xc004),
    ] {
        assert_eq!(offset(p), address);
    }
    assert_eq!(r.symc_channel_iter().count(), 2);
    assert_eq!(r.symc_output_iter().count(), 2);
    assert_eq!(r.symc_iv_iter().count(), 2);
    assert_eq!(core::mem::size_of_val(r.symc_channel(0)), 0x100);
    assert_eq!(core::mem::size_of_val(r.symc_output(0)), 0x100);
    assert_eq!(core::mem::size_of_val(r.symc_iv(0)), 0x80);
    for i in 0..2 {
        let input = r.symc_channel(i);
        let output = r.symc_output(i);
        for (p, relative) in [
            (input.control().as_ptr(), 0),
            (input.output_control().as_ptr(), 4),
            (input.key_control().as_ptr(), 0x10),
            (input.node_base_high().as_ptr(), 0x20),
            (input.node_base_low().as_ptr(), 0x24),
            (input.node_depth().as_ptr(), 0x2c),
            (input.write_pointer().as_ptr(), 0x30),
            (input.submit().as_ptr(), 0x30),
            (input.read_pointer().as_ptr(), 0x34),
        ] {
            assert_eq!(offset(p), 0x4100 + i * 0x100 + relative);
        }
        for (p, relative) in [
            (output.node_base_high().as_ptr(), 0x20),
            (output.node_base_low().as_ptr(), 0x24),
            (output.node_depth().as_ptr(), 0x28),
            (output.write_pointer().as_ptr(), 0x30),
            (output.submit().as_ptr(), 0x30),
            (output.read_pointer().as_ptr(), 0x34),
        ] {
            assert_eq!(offset(p), 0xd000 + i * 0x100 + relative);
        }
        let iv = r.symc_iv(i);
        for (j, p) in [
            iv.word0().as_ptr(),
            iv.word1().as_ptr(),
            iv.word2().as_ptr(),
            iv.word3().as_ptr(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(offset(p), 0x9080 + i * 0x80 + j * 4);
        }
        let error = r.hash_error(i);
        assert_eq!(offset(error.symc_cause().as_ptr()), 0x8060 + i * 16);
        assert_eq!(offset(error.symc_status().as_ptr()), 0x8064 + i * 16);
        assert_eq!(offset(error.cause().as_ptr()), 0x8068 + i * 16);
        assert_eq!(offset(error.status().as_ptr()), 0x806c + i * 16);
    }
}

#[test]
fn configuration_and_keyslot_fields_preserve_reserved_bits() {
    let r = storage();
    for channel in r.symc_channel_iter() {
        for seed in [0, u32::MAX, 0x1234_5678, 0xa5a5_5a5a] {
            for slot in [0, 1, 15, 255, 511] {
                for (alg, mode, len, width, decrypt) in [
                    (2, 1, 1, 0, false),
                    (2, 3, 2, 0, true),
                    (2, 6, 3, 0, false),
                    (5, 3, 1, 1, true),
                ] {
                    unsafe {
                        channel.key_control().write_with_zero(|w| w.bits(seed));
                    }
                    channel.key_control().modify(|_, w| unsafe {
                        w.keyslot()
                            .bits(slot)
                            .algorithm()
                            .bits(alg)
                            .mode()
                            .bits(mode)
                            .key_length()
                            .bits(len)
                            .data_width()
                            .bits(width)
                            .decrypt()
                            .bit(decrypt)
                    });
                    let k = channel.key_control().read();
                    assert_eq!(
                        k.bits(),
                        (seed & !0x1fff_01ff)
                            | u32::from(slot)
                            | (u32::from(alg) << 16)
                            | (u32::from(mode) << 20)
                            | (u32::from(len) << 24)
                            | (u32::from(width) << 26)
                            | (u32::from(decrypt) << 28)
                    );
                    assert_eq!(k.keyslot().bits(), slot);
                    assert_eq!(
                        (
                            k.algorithm().bits(),
                            k.mode().bits(),
                            k.key_length().bits(),
                            k.data_width().bits(),
                            k.decrypt().bit()
                        ),
                        (alg, mode, len, width, decrypt)
                    );
                }
            }
            for security in [5u8, 10] {
                unsafe {
                    channel.control().write_with_zero(|w| w.bits(seed));
                }
                channel.control().modify(|_, w| unsafe {
                    w.source_security()
                        .bits(security)
                        .destination_security()
                        .bits(security)
                        .enable()
                        .set_bit()
                });
                assert_eq!(
                    channel.control().read().bits(),
                    (seed & !0x8000_ff00) | 0x8000_0000 | (u32::from(security) * 0x1100)
                );
            }
            unsafe {
                channel.node_depth().write_with_zero(|w| w.bits(seed));
            }
            channel
                .node_depth()
                .modify(|_, w| unsafe { w.length().bits(2) });
            assert_eq!(channel.node_depth().read().bits(), (seed & !255) | 2);
        }
    }
}

#[test]
fn command_words_do_not_gain_rmw_or_automatic_w1c_semantics() {
    use ws53_pac::generic::Writable;
    assert_eq!(
        <spacc::symc_clear_ack::SymcClearAckSpec as Writable>::ONE_TO_MODIFY_FIELDS_BITMAP,
        0
    );
    assert_eq!(<spacc::symc_output_interrupt_ack::SymcOutputInterruptAckSpec as Writable>::ONE_TO_MODIFY_FIELDS_BITMAP,0);
    let r = storage();
    for word in [0, 2, 4, 0xa5a5_0006, u32::MAX] {
        unsafe {
            r.symc_clear_request()
                .write_with_zero(|w| w.value().bits(word));
            assert_eq!(r.symc_clear_read().read().value().bits(), word);
            r.symc_clear_ack().write_with_zero(|w| w.value().bits(word));
            assert_eq!(r.symc_clear_status().read().bits(), word);
            r.symc_input_interrupt_ack()
                .write_with_zero(|w| w.value().bits(word));
            assert_eq!(r.symc_input_interrupt_status().read().bits(), word);
            r.symc_output_interrupt_ack()
                .write_with_zero(|w| w.value().bits(word));
            assert_eq!(r.symc_output_interrupt_status().read().bits(), word);
            for i in 0..2 {
                r.symc_channel(i)
                    .submit()
                    .write_with_zero(|w| w.value().bits(word));
                r.symc_output(i)
                    .submit()
                    .write_with_zero(|w| w.value().bits(!word));
                assert_eq!(
                    r.symc_channel(i).write_pointer().read().pointer().bits(),
                    word as u8
                );
                assert_eq!(
                    r.symc_output(i).write_pointer().read().pointer().bits(),
                    (!word) as u8
                );
            }
        }
    }
}

#[test]
fn owner_and_interrupt_updates_preserve_other_users() {
    let r = storage();
    for seed in [0, u32::MAX, 0x1234_5678, 0xa55a_5aa5] {
        for a in 0..16u8 {
            for b in 0..16u8 {
                unsafe {
                    r.symc_lock().write_with_zero(|w| w.bits(seed));
                }
                r.symc_lock()
                    .modify(|_, w| unsafe { w.channel1_owner().bits(a).channel2_owner().bits(b) });
                assert_eq!(
                    r.symc_lock().read().bits(),
                    (seed & !0xff0) | (u32::from(a) << 4) | (u32::from(b) << 8)
                );
            }
        }
        for mask in 0..4 {
            unsafe {
                r.symc_input_interrupt_enable()
                    .write_with_zero(|w| w.bits(seed));
                r.symc_output_interrupt_enable()
                    .write_with_zero(|w| w.bits(seed));
            }
            r.symc_input_interrupt_enable().modify(|_, w| {
                w.channel1()
                    .bit(mask & 1 != 0)
                    .channel2()
                    .bit(mask & 2 != 0)
            });
            r.symc_output_interrupt_enable().modify(|_, w| {
                w.channel1()
                    .bit(mask & 1 != 0)
                    .channel2()
                    .bit(mask & 2 != 0)
            });
            assert_eq!(
                r.symc_input_interrupt_enable().read().bits(),
                (seed & !6) | (mask << 1)
            );
            assert_eq!(
                r.symc_output_interrupt_enable().read().bits(),
                (seed & !6) | (mask << 1)
            );
        }
    }
}

#[test]
fn read_only_diagnostics_do_not_alias_hash_or_each_other() {
    let r = storage();
    for i in 0..2 {
        let e = r.hash_error(i);
        for bit in [0, 1, 2, 3, 4, 6, 7, 16, 17, 18] {
            unsafe {
                e.symc_cause().as_ptr().write_volatile(1 << bit);
            }
            let v = e.symc_cause().read();
            let fields = [
                v.destination_security().bit(),
                v.source_security().bit(),
                v.algorithm().bit(),
                v.invalid_algorithm().bit(),
                v.decrypt().bit(),
                v.sc().bit(),
                v.key_size().bit(),
                v.key_information().bit(),
                v.tee_only().bit(),
                v.aidsp().bit(),
            ];
            assert_eq!(fields.into_iter().filter(|x| *x).count(), 1);
            assert_eq!(e.cause().read().bits(), 0);
        }
        for channel in 0..16 {
            unsafe {
                e.symc_status().as_ptr().write_volatile(channel << 16);
            }
            assert_eq!(u32::from(e.symc_status().read().channel().bits()), channel);
            assert_eq!(e.status().read().bits(), 0);
        }
        let iv = r.symc_iv(i);
        for (j, p) in [
            iv.word0().as_ptr(),
            iv.word1().as_ptr(),
            iv.word2().as_ptr(),
            iv.word3().as_ptr(),
        ]
        .into_iter()
        .enumerate()
        {
            unsafe {
                p.write_volatile(0x1234_5678 + j as u32);
            }
        }
        assert_eq!(
            [
                iv.word0().read().value().bits(),
                iv.word1().read().value().bits(),
                iv.word2().read().value().bits(),
                iv.word3().read().value().bits()
            ],
            [0x1234_5678, 0x1234_5679, 0x1234_567a, 0x1234_567b]
        );
    }
}
