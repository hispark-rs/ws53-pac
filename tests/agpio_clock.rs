#![cfg(not(target_arch = "riscv32"))]

#[test]
fn shared_agpio_clock_has_same_address_halfword_and_word_views() {
    let registers: ws53_pac::system_memory::RegisterBlock = unsafe { core::mem::zeroed() };
    let base = &registers as *const _ as usize;
    assert_eq!(ws53_pac::SystemMemory::PTR as usize, 0x5703_0000);
    assert_eq!(registers.gpio_clock() as *const _ as usize - base, 0x24);
    assert_eq!(
        registers.gpio_clock_word() as *const _ as usize - base,
        0x24
    );
    assert_eq!(core::mem::size_of_val(registers.gpio_clock()), 2);
    assert_eq!(core::mem::size_of_val(registers.gpio_clock_word()), 4);
    assert_eq!(registers.bram_share() as *const _ as usize - base, 0x234);
}

#[test]
fn halfword_enable_preserves_source_reserved_bits_and_upper_word() {
    // Private host storage, not silicon reset data or real clock qualification.
    let registers: ws53_pac::system_memory::RegisterBlock = unsafe { core::mem::zeroed() };
    for value in [0, 2, 0xffff_8002, 0xa5a5_fffe] {
        // This complete synthetic word is written only to private host storage.
        unsafe {
            registers
                .gpio_clock_word()
                .write_with_zero(|w| w.bits(value))
        };
        registers.gpio_clock().modify(|_, w| w.enable().set_bit());
        assert_eq!(registers.gpio_clock_word().read().bits(), value | 1);
        assert!(registers.gpio_clock().read().enable().bit());
        assert_eq!(
            registers.gpio_clock().read().slow_clock_select().bit(),
            value & 2 != 0
        );
    }
}
