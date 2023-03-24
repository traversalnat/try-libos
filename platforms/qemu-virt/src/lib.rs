#![no_std]
#![feature(naked_functions, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]
#![feature(allocator_api)]
#![allow(unreachable_code)]

mod async_utils;
mod e1000;
mod pci;
mod tasks;
mod timer;
mod virt;

extern crate alloc;
use alloc::vec::Vec;
use alloc::vec;
use core::time::Duration;
use stdio::log::info;

use qemu_virt_ld as linker;

use uart_16550::MmioSerialPort;

pub use platform::Platform;

pub use virt::{Virt as PlatformImpl, MACADDR};

#[linkage = "weak"]
#[no_mangle]
fn obj_main() {
    panic!()
}

linker::boot0!(rust_main; stack = 4096 * 3);

extern "C" fn rust_main() -> ! {
    let layout = linker::KernelLayout::locate();
    unsafe {
        layout.zero_bss();
    }

    mem::init_heap(layout.end(), 0x30_0000);

    virt::init(unsafe { MmioSerialPort::new(0x1000_0000) });

    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&virt::Stdio);

    pci::pci_init();

    obj_main();

    tasks::block_on(async {
        loop {
            tasks::spawn(async {
                let mut v = vec![1];
                v.push(2);
                info!("async block on {}", v[1]);
            });
            async_utils::async_wait(Duration::from_secs(1)).await;
        }
    });

    unreachable!()
}
