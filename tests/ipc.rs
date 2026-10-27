#![cfg(not(target_arch = "riscv32"))]

use ws53_pac::m_crg::{
    self, ipc_receive_ack_command::IpcReceiveAckCommandSpec as Ack,
    ipc_receive_ack_sync::IpcReceiveAckSyncSpec as Sync,
    ipc_receive_enable::IpcReceiveEnableSpec as Enable,
    ipc_receive_status::IpcReceiveStatusSpec as Receive,
    ipc_send_set_command::IpcSendSetCommandSpec as Send,
    ipc_send_status::IpcSendStatusSpec as Pending,
};

macro_rules! lacks_trait {
    ($ty:ty, $bound:path) => {
        const _: fn() = || {
            trait Ambiguous<A> {
                fn check() {}
            }
            impl<T: ?Sized> Ambiguous<()> for T {}
            struct Forbidden;
            impl<T: ?Sized + $bound> Ambiguous<Forbidden> for T {}
            let _ = <$ty as Ambiguous<_>>::check;
        };
    };
}
lacks_trait!(Ack, ws53_pac::generic::Readable);
lacks_trait!(Send, ws53_pac::generic::Readable);
lacks_trait!(Receive, ws53_pac::generic::Writable);
lacks_trait!(Pending, ws53_pac::generic::Writable);
lacks_trait!(Sync, ws53_pac::generic::Writable);
lacks_trait!(Ack, ws53_pac::generic::Resettable);
lacks_trait!(Send, ws53_pac::generic::Resettable);
lacks_trait!(Receive, ws53_pac::generic::Resettable);
lacks_trait!(Pending, ws53_pac::generic::Resettable);
lacks_trait!(Sync, ws53_pac::generic::Resettable);
lacks_trait!(Enable, ws53_pac::generic::Readable);
lacks_trait!(Enable, ws53_pac::generic::Resettable);

#[test]
fn ipc_access_widths_and_alias_match_patched_contract() {
    fn halfword<T: ws53_pac::generic::RegisterSpec<Ux = u16>>() {}
    fn word<T: ws53_pac::generic::RegisterSpec<Ux = u32>>() {}
    halfword::<Send>();
    word::<Ack>();
    word::<Sync>();
    word::<Receive>();
    word::<Pending>();
    let r: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    let base = &r as *const _ as usize;
    assert_eq!(ws53_pac::MCrg::PTR as usize, 0x5200_0000);
    assert_eq!(r.ipc_receive_enable() as *const _ as usize - base, 0x20);
    assert_eq!(r.ipc_receive_status() as *const _ as usize - base, 0x28);
    assert_eq!(
        r.ipc_receive_ack_command() as *const _ as usize - base,
        0x2c
    );
    assert_eq!(r.ipc_receive_ack_sync() as *const _ as usize - base, 0x2c);
    assert_eq!(r.ipc_send_status() as *const _ as usize - base, 0x30);
    assert_eq!(r.ipc_send_set_command() as *const _ as usize - base, 0x34);
    assert_eq!(r.ipc_release_command() as *const _ as usize - base, 0x38);
}

#[test]
fn send_is_halfword_and_does_not_overwrite_upper_host_ram_guard() {
    // Host RAM encoding only; no hardware write or IRQ delivery is implied.
    let r: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    let upper = unsafe { (&r as *const _ as *mut u8).add(0x36).cast::<u16>() };
    for sentinel in [0xa55a, 0x5aa5, u16::MAX] {
        unsafe { upper.write_volatile(sentinel) };
        unsafe {
            r.ipc_send_set_command()
                .write_with_zero(|w| w.channel0().set_bit())
        };
        assert_eq!(unsafe { upper.read_volatile() }, sentinel);
        assert_eq!(unsafe { upper.sub(1).read_volatile() }, 1);
        unsafe {
            r.ipc_send_set_command()
                .write_with_zero(|w| w.channel1().set_bit())
        };
        assert_eq!(unsafe { upper.read_volatile() }, sentinel);
        assert_eq!(unsafe { upper.sub(1).read_volatile() }, 2);
    }
}

#[test]
fn receive_enable_and_ack_are_complete_word_commands() {
    let r: m_crg::RegisterBlock = unsafe { core::mem::zeroed() };
    for seed in [0, u32::MAX, 0x8000_0100, 0xa55a_1357] {
        unsafe { r.ipc_receive_enable().write_with_zero(|w| w.bits(seed)) };
        unsafe {
            r.ipc_receive_enable()
                .write_with_zero(|w| w.channel0().set_bit())
        };
        let host_word = (&r as *const _ as *const u8)
            .wrapping_add(0x20)
            .cast::<u32>();
        assert_eq!(unsafe { host_word.read_volatile() }, 1);
        unsafe {
            r.ipc_receive_ack_command()
                .write_with_zero(|w| w.channel1().set_bit())
        };
        assert_eq!(r.ipc_receive_ack_sync().read().bits(), 2);
        unsafe {
            r.ipc_receive_ack_command()
                .write_with_zero(|w| w.channel0().set_bit())
        };
        assert_eq!(r.ipc_receive_ack_sync().read().bits(), 1);
    }
}

#[test]
fn ipc_irqs_are_absolute_cause_values() {
    use ws53_pac::interrupt::ExternalInterrupt;
    assert_eq!(ExternalInterrupt::APP_INT0 as u16, 26);
    assert_eq!(ExternalInterrupt::APP_INT1 as u16, 27);
}
