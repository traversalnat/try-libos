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
use stdio::log::{self};
use thread::*;

use uart_16550::MmioSerialPort;

pub use platform::Platform;
use virt::Virt;
pub use virt::{Virt as PlatformImpl, MACADDR};

use tasks::{NUM_SLICES_LEVELS, QUEUES};

use crate::{tasks::add_task_to_queue, timer::check_timer};

const MM_SIZE: usize = 2 << 20;

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

    Virt::spawn(async { obj_main() });

    // idle thread
    Virt::spawn(async {
        loop {
            // if net is used
            e1000::async_recv();
        }
    });

    Virt::spawn(async {
        loop {
            syscall::sys_sleep(1000);
            timer::sys_yield();
        }
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

    let mut level: usize = 0;
    loop {
        let task = QUEUES.lock()[level].pop_front();
        if task.is_none() {
            level += 1;
            level %= NUM_SLICES_LEVELS;
            continue;
        }

        let mut task = task.unwrap();
        let ticks = task.ticks(); // 用于task在给定时间片内是否切换协程

        set_timer(Virt::rdtime() as u64 + 12500 * task.slice as u64);
        task.run();

        use scause::{Exception, Interrupt, Trap};
        let opt_task = match scause::read().cause() {
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                set_timer(u64::MAX);
                check_timer();

                let new_ticks = task.ticks();
                // 时间片应该降低
                if new_ticks > ticks {
                    task.slice = core::cmp::min(task.slice - 1, 1);
                } else {
                    task.slice = core::cmp::min(task.slice + 1, 5);
                }

                // TODO 对于最低层 task，要切出其它协程
                Some(task)
            }
            Trap::Exception(Exception::UserEnvCall) => syscall::handle_syscall(task),
            _ => None,
        };

        if opt_task.is_some() {
            add_task_to_queue(opt_task.unwrap());
        }
    }
    log::error!("Shutdown\n");
    system_reset(Shutdown, NoReason);
    unreachable!()
}
