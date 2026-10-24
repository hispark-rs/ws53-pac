#![cfg(not(target_arch = "riscv32"))]

// Local RAM checks of the generated ABI. Reference: fixed WS53 SDK 163d9150,
// hal_sfc_v150_regs_def.h and the independently compiled original C helpers in
// hisi-registers/scripts/check_ws53_sfc_contract.py. This is not a hardware
// transfer, reset-value, read-side-effect, NOR readiness or DMA drain test.
use ws53_pac::sfc_cfg;

fn offset<B, R>(base: &B, register: &R) -> usize {
    register as *const R as usize - base as *const B as usize
}

#[test]
fn command_and_dma_windows_preserve_physical_word_layout() {
    let r: sfc_cfg::RegisterBlock = unsafe { core::mem::zeroed() };
    assert_eq!(ws53_pac::SfcCfg::PTR as usize, 0x5300_0000);
    for (actual, expected) in [
        (offset(&r, r.global_config()), 0x100),
        (offset(&r, r.timing()), 0x110),
        (offset(&r, r.bus_config1()), 0x200),
        (offset(&r, r.bus_config2()), 0x204),
        (offset(&r, r.bus_dma_ctrl()), 0x240),
        (offset(&r, r.bus_dma_mem_saddr()), 0x244),
        (offset(&r, r.bus_dma_flash_saddr()), 0x248),
        (offset(&r, r.bus_dma_len()), 0x24c),
        (offset(&r, r.bus_dma_ahb_ctrl()), 0x250),
        (offset(&r, r.cmd_config()), 0x300),
        (offset(&r, r.cmd_ins()), 0x308),
        (offset(&r, r.cmd_addr()), 0x30c),
        (offset(&r, r.cmd_databuf_0()), 0x400),
        (offset(&r, r.cmd_databuf_15()), 0x43c),
    ] {
        assert_eq!(actual, expected);
    }
    assert_eq!(core::mem::size_of_val(r.cmd_config()), 4);
    assert_eq!(core::mem::size_of_val(r.bus_dma_ctrl()), 4);
}

#[test]
fn generated_command_fields_encode_original_c_length_and_direction() {
    let r: sfc_cfg::RegisterBlock = unsafe { core::mem::zeroed() };
    for bytes in 1..=64u8 {
        // Standard SPI, CS1, read, data present, no address or dummy bytes.
        unsafe {
            r.cmd_config().write_with_zero(|w| {
                w.sel_cs()
                    .set_bit()
                    .rw()
                    .set_bit()
                    .data_en()
                    .set_bit()
                    .data_cnt()
                    .bits(bytes - 1)
                    .start()
                    .set_bit()
            });
        }
        assert_eq!(
            r.cmd_config().read().bits(),
            0x183 | (u32::from(bytes - 1) << 9)
        );
        assert_eq!(r.cmd_config().read().data_cnt().bits(), bytes - 1);
    }
    // The two controller START fields are distinct. Neither describes NOR WIP.
    unsafe {
        r.bus_dma_ctrl().write_with_zero(|w| {
            w.dma_sel_cs()
                .set_bit()
                .dma_rw()
                .set_bit()
                .dma_start()
                .set_bit()
        });
    }
    assert_eq!(r.bus_dma_ctrl().read().bits(), 0x13);
    assert!(r.cmd_config().read().start().bit());
    assert!(r.bus_dma_ctrl().read().dma_start().bit());
}

#[test]
fn additional_sdk_fields_preserve_unrelated_bits() {
    let r: sfc_cfg::RegisterBlock = unsafe { core::mem::zeroed() };
    unsafe {
        r.global_config().write_with_zero(|w| w.bits(0xa55a_0001));
        r.global_config().modify(|_, w| w.sample_test().bits(9));
        r.cmd_ins().write_with_zero(|w| w.bits(0xa55a_0005));
        r.cmd_ins()
            .modify(|_, w| w.cmd_ddr_mode().set_bit().ddr_dummy_byte_cnt().bits(10));
        r.bus_config2().write_with_zero(|w| w.bits(0xa55a_0003));
        r.bus_config2().modify(|_, w| {
            w.bus_ddr_mode()
                .set_bit()
                .ddr_rd_dummy_bytes()
                .bits(5)
                .ddr_wr_dummy_bytes()
                .bits(9)
                .sfc_ddr_tx_delay_enable()
                .set_bit()
        });
    }
    assert_eq!(r.global_config().read().bits(), 0xa55a_0241);
    assert_eq!(r.cmd_ins().read().bits(), 0xa55a_a105);
    assert_eq!(r.bus_config2().read().bits(), 0xa55a_195b);
}

macro_rules! no_reset {
    ($ty:ty) => {
        const _: fn() = || {
            trait Ambiguous<A> {
                fn check() {}
            }
            impl<T: ?Sized> Ambiguous<()> for T {}
            struct HasReset;
            impl<T: ?Sized + ws53_pac::generic::Resettable> Ambiguous<HasReset> for T {}
            let _ = <$ty as Ambiguous<_>>::check;
        };
    };
}
no_reset!(sfc_cfg::cmd_config::CmdConfigSpec);
no_reset!(sfc_cfg::bus_dma_ctrl::BusDmaCtrlSpec);
no_reset!(sfc_cfg::cmd_databuf_0::CmdDatabuf0Spec);
