#![cfg(not(target_arch = "riscv32"))]

use ws53_pac::system_memory::{
    self, ccore_logic_clock_release_command::CcoreLogicClockReleaseCommandSpec as Command,
    ccore_reset_status::CcoreResetStatusSpec as Status,
};

// Compile-time ambiguity rejects newly introduced traits without trying MMIO.
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
lacks_trait!(Command, ws53_pac::generic::Readable);
lacks_trait!(Command, ws53_pac::generic::Resettable);
lacks_trait!(Status, ws53_pac::generic::Writable);
lacks_trait!(Status, ws53_pac::generic::Resettable);

#[test]
fn command_and_status_are_full_word_aliases_at_the_sdk_address() {
    fn word<T: ws53_pac::generic::RegisterSpec<Ux = u32>>() {}
    word::<Command>();
    word::<Status>();
    let regs: system_memory::RegisterBlock = unsafe { core::mem::zeroed() };
    let base = &regs as *const _ as usize;
    let status = regs.ccore_reset_status();
    let command = regs.ccore_logic_clock_release_command();
    assert_eq!(ws53_pac::SystemMemory::PTR as usize, 0x5703_0000);
    assert_eq!(status as *const _ as usize - base, 0x96c);
    assert_eq!(command as *const _ as usize - base, 0x96c);
    assert_eq!(core::mem::size_of_val(status), 4);
    assert_eq!(core::mem::size_of_val(command), 4);
}

#[test]
fn full_word_release_overwrites_the_owned_host_word_with_exactly_three() {
    // Host RAM only: this establishes encoding, not silicon reset or ownership.
    let regs: system_memory::RegisterBlock = unsafe { core::mem::zeroed() };
    for seed in [0, u32::MAX, 0xa55a_1357, 7, 0x8000_0000] {
        // Unsafe raw writes deliberately demonstrate that SVD constraints are
        // not runtime guards; the future HAL must prevent these operations.
        unsafe {
            regs.ccore_logic_clock_release_command()
                .write_with_zero(|w| w.bits(seed));
            regs.ccore_logic_clock_release_command()
                .write_with_zero(|w| w.logic_clock_release().bits(3));
        }
        assert_eq!(regs.ccore_reset_status().read().bits(), 3);
        assert_eq!(
            regs.ccore_reset_status().read().released_domains().bits(),
            3
        );
    }
}
