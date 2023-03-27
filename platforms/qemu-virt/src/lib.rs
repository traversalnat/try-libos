#![no_std]
#![feature(naked_functions, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]
#![feature(allocator_api)]
#![allow(unreachable_code)]

mod async_utils;
mod e1000;
mod tasks;
mod time;
mod virt;

extern crate alloc;

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

// const KERNEL_HEAP_SIZE: usize = 128 << 20;
// static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

linker::boot0!(rust_main; show_me_the_reason; stack = 4096 * 12);

extern "C" fn rust_main() -> ! {
    let layout = linker::KernelLayout::locate();
    unsafe {
        layout.zero_bss();
    }

    let (heap, len) = virt::Virt::heap();
    mem::init_heap(heap, len);

    virt::init(unsafe { MmioSerialPort::new(0x1000_0000) });

    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&virt::Stdio);

    timer::init(&virt::TimeProvider);

    e1000::init();

    obj_main();

    tasks::block_on(async {
        // idle task
        // loop {
        //     async_utils::async_yield().await;
        // }
    });

    unreachable!()
}

pub fn show_me_the_reason() {
    match scause::read().cause() {
        cause => {
            info!(
                "{:#?}, spec {:x}, stval {:x}",
                cause,
                sepc::read(),
                stval::read()
            );
        }
    };
    loop {}
}
