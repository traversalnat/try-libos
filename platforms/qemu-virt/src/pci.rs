#![no_std]
#![feature(asm_sym)]

use core::ptr::{read, write};

pub fn pci_init() {
    let e1000_regs: u32 = 0x40000000;
    let ecam: u32 = 0x30000000;

    unsafe {
        for dev in 0..32 {
            let mut bus: u32 = 0;
            let func: u32 = 0;
            let offset: u32 = 0;
            let off: u32 = (bus << 16) | (dev << 11) | (func << 8) | (offset);
            let base = (ecam + off) as *mut u32;
            let base = core::slice::from_raw_parts_mut(base, 10);
            let id: u32 = base[0];

            // // 100e:8086 is an e1000
            if id == 0x100e8086 {
                // command and status register.
                // bit 0 : I/O access enable
                // bit 1 : memory access enable
                // bit 2 : enable mastering
                base[1] = 7;
                __sync_synchronize();
                for i in 0..6 {
                    let old = base[4 + i];
                    base[4 + i] = 0xffffffff;
                    __sync_synchronize();
                    base[4 + i] = old;
                }
                // tell the e1000 to reveal its registers at
                // physical address 0x40000000.
                base[4 + 0] = e1000_regs;
                // e1000_init(e1000_regs);
            }
        }
    }
}

#[inline]
unsafe fn __sync_synchronize() {
    core::arch::asm!("", "memory", "volatile");
}
