#![no_std]
#![feature(naked_functions, asm_sym, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]

mod e1000;
mod pci;
mod thread;
mod timer;
mod virt;

extern crate alloc;

use qemu_virt_ld as linker;

use riscv::register::*;
use sbi_rt::*;
use thread::*;
use timer::*;
use uart_16550::MmioSerialPort;
use stdio::log;

pub use platform::Platform;
use virt::Virt;
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

    // common 中库由 platform 负责初始化
    // mem
    let (heap_base, heap_size) = Virt::heap();
    mem::init_heap(heap_base, heap_size);

    virt::init(unsafe { MmioSerialPort::new(0x1000_0000) });

    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&virt::Stdio);

    executor::init(&virt::Executor);

    pci::pci_init();
    log::info!("init kthread");

    Virt::spawn(obj_main);

    // idle thread
    Virt::spawn(|| loop {
        // if net is used
        e1000::async_recv();
    });

    Virt::spawn(|| loop {
        timer::sys_yield();
    });

    let mut t = TaskControlBlock::ZERO;
    t.init(schedule as usize);
    unsafe {
        t.execute();
    }

    log::warn!("error shutdown");
    system_reset(Shutdown, NoReason);
    unreachable!()
}

extern "C" fn schedule() -> ! {
    use TaskStatus::*;
    unsafe {
        sie::set_stimer();
    }
    while !RUN_THREADS.lock().is_empty() {
        let ctx = RUN_THREADS.lock().pop_front().unwrap();
        set_timer(Virt::rdtime() as u64 + 12500);
        loop {
            // 设置当前线程
            set_current_thread(ctx.clone());
            // 设置当前线程状态
            ctx.lock().status = Running;
            unsafe {
                ctx.lock().execute();
            }

            use scause::{Interrupt, Trap};
            let finish = match scause::read().cause() {
                Trap::Interrupt(Interrupt::SupervisorTimer) => {
                    set_timer(u64::MAX);
                    check_timer(); // 检查到时线程
                    false
                }
                _ => true,
            };

            if finish {
                ctx.lock().status = Finish;
            }
            break;
        }
        if ctx.lock().status == Running {
            RUN_THREADS.lock().push_back(ctx);
        }
    }
    log::info!("Shutdown\n");
    system_reset(Shutdown, NoReason);
    unreachable!()
}
