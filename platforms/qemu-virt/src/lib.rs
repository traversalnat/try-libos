#![no_std]
#![feature(naked_functions, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]
#![feature(allocator_api)]
#![allow(unreachable_code)]

mod async_executor;
mod e1000;
mod mm;
mod pci;
mod syscall;
mod tasks;
mod thread;
mod timer;
mod trap;
mod virt;

extern crate alloc;

use qemu_virt_ld as linker;

use riscv::register::*;
use sbi_rt::*;
use stdio::log::{self, info};
use thread::*;

use uart_16550::MmioSerialPort;

pub use platform::Platform;
use virt::Virt;
pub use virt::{Virt as PlatformImpl, MACADDR};

use tasks::QUEUES;

use crate::{syscall::{syscall}, tasks::add_task_to_queue, timer::check_timer};

const MM_SIZE: usize = 32 << 20;

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
    mm::init_heap(heap_base, MM_SIZE);
    mem::init_heap(heap_base + MM_SIZE, heap_size - MM_SIZE);

    virt::init(unsafe { MmioSerialPort::new(0x1000_0000) });

    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&virt::Stdio);

    executor::init(&virt::Executor);

    pci::pci_init();
    log::info!("init kthread");

    Virt::spawn(async {
        obj_main()
    });

    let mut t = TaskControlBlock::ZERO;
    t.init(schedule as usize);
    unsafe {
        t.execute();
    }
    unreachable!()
}

extern "C" fn schedule() -> ! {
    // WARNING: 调度器不能与线程争夺资源，包括全局内存分配器，TIMERS, THREADS, 等的锁

    unsafe {
        sie::set_stimer();
    }

    let level: usize = 0;
    loop {
        let task = QUEUES.lock()[level].pop_front();
        // TODO:  实现 MLFQ 算法
        let mut task = task.expect("no task, Shutdown");

        let ticks = task.ticks(); // 用于task在给定时间片内是否切换协程

        info!("{} run", task.tid);
        set_timer(Virt::rdtime() as u64 + 12500 * task.slice as u64);
        task.run();
        info!("{} runned", task.tid);

        use scause::{Exception, Interrupt, Trap};
        match scause::read().cause() {
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                check_timer();
                set_timer(u64::MAX);

                let new_ticks = task.ticks();
                // 时间片应该降低
                if new_ticks > ticks {
                    task.slice = core::cmp::min(task.slice - 1, 1);
                } else {
                    task.slice = core::cmp::min(task.slice + 1, 5);
                }

                // TODO 对于最低层 task，要切出其它协程
                add_task_to_queue(task);
            }
            Trap::Exception(Exception::UserEnvCall) => {
                if let Some(task) = syscall::handle_syscall(task) {
                    add_task_to_queue(task);
                }
            }
            _ => {
                log::info!("{:#?}, spec {:x}, stval {:x}", scause::read().cause(), sepc::read(), stval::read());
            }
        };
    }
    log::error!("Shutdown\n");
    system_reset(Shutdown, NoReason);
    unreachable!()
}
