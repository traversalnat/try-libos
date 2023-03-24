#![no_std]
#![feature(naked_functions, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]
#![feature(allocator_api)]
#![allow(unreachable_code)]

mod async_utils;
mod e1000;
mod tasks;
mod timer;
mod virt;

extern crate alloc;

use core::time::Duration;
use stdio::log::info;

use qemu_virt_ld as linker;

use uart_16550::MmioSerialPort;

pub use platform::Platform;

pub use virt::{Virt as PlatformImpl, MACADDR};

use riscv::register::*;

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
        stvec::write(
            show_me_the_reason  as usize,
            stvec::TrapMode::Direct,
        );
    }

    let (base, len) = virt::Virt::heap();
    mem::init_heap(base, len);

    virt::init(unsafe { MmioSerialPort::new(0x1000_0000) });

    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&virt::Stdio);

    e1000::init();

    obj_main();

    // tasks::block_on(async {
    //     loop {
    //         tasks::spawn(async {
    //             info!("async block on");
    //         });
    //         async_utils::async_wait(Duration::from_secs(1)).await;
    //     }
    // });

    unreachable!()
}

pub fn show_me_the_reason() {
    match scause::read().cause() {
        _ => {
            info!(
                "{:#?}, spec {:x}, stval {:x}",
                scause::read().cause(),
                sepc::read(),
                stval::read()
            );
        }
    };
    loop {}
}
