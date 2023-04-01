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
mod plic;
mod syscall;
mod tasks;
mod thread;
mod timer;
mod trap;
mod virt;

extern crate alloc;
extern crate timer as crate_timer;

use qemu_virt_ld as linker;

use riscv::register::*;
use sbi_rt::*;
use stdio::log::{self};
use thread::*;

use uart_16550::MmioSerialPort;

pub use platform::Platform;
use virt::Virt;
pub use virt::{Virt as PlatformImpl, MACADDR};

use crate::{
    e1000::async_recv,
    plic::{plic_claim, plic_complete, E1000_IRQ},
    tasks::{add_task_to_queue, add_task_transient, get_task_from_queue, NUM_SLICES_LEVELS},
    timer::check_timer,
};

const MM_SIZE: usize = 32 << 20;

#[linkage = "weak"]
#[no_mangle]
fn obj_main() {
    panic!()
}

linker::boot0!(rust_main; stack = 4096 * 12);

extern "C" fn rust_main() -> ! {
    let layout = linker::KernelLayout::locate();
    unsafe {
        layout.zero_bss();
    }
    let (heap_base, heap_size) = Virt::heap();
    mm::init_heap(heap_base, MM_SIZE);
    mem::init_heap(heap_base + MM_SIZE, heap_size - MM_SIZE);

    virt::init(unsafe { MmioSerialPort::new(0x1000_0000) });

    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&virt::Stdio);

    crate_timer::init(&virt::TimeProvider);
    executor::init(&virt::Executor);

    pci::pci_init();
    // 中断
    plic::plic_init();
    plic::plic_init_hart();

    log::info!("init kthread");

    Virt::spawn(async {
        loop {
            async_recv().await
        }
    });

    Virt::spawn(async { obj_main() });

    let mut t = TaskControlBlock::ZERO;
    t.init(schedule as usize);
    unsafe {
        t.execute();
    }
    unreachable!()
}

#[inline]
fn get_slice(slice: usize) -> u64 {
    core::cmp::min(slice * slice, 10) as u64
}

extern "C" fn schedule() -> ! {
    // WARNING: 调度器不能与线程争夺资源，包括全局内存分配器，TIMERS, THREADS, 等的锁

    unsafe {
        sie::set_stimer();
        sie::set_sext();
    }

    loop {
        let task = get_task_from_queue();
        let mut task = task.expect("no task, Shutdown");

        let ticks = task.ticks(); // 用于task在给定时间片内是否切换协程

        // 计算密集型任务执行线程优先级更高、但时间片更少
        if task.status() == TaskStatus::Blocking {
            set_timer(Virt::rdtime() as u64 + 12500 * get_slice(task.slice));
            task.set_status(TaskStatus::Running);
        }
        task.run();

        use scause::{Exception, Interrupt, Trap};
        match scause::read().cause() {
            Trap::Interrupt(Interrupt::SupervisorTimer) => {
                set_timer(u64::MAX);
                check_timer();

                task.set_status(TaskStatus::Blocking);

                let new_ticks = task.ticks();
                // 时间片应该降低
                if new_ticks > ticks {
                    task.slice = core::cmp::max(task.slice - 1, 1);
                } else {
                    task.slice = core::cmp::min(task.slice + 1, NUM_SLICES_LEVELS);
                    // 时间片最大时，仍然不让出，考虑将线程中其他协程取出放入新线程执行
                    if task.slice == NUM_SLICES_LEVELS {
                        if let Some(task) = task.steal() {
                            add_task_to_queue(task);
                        }
                    }
                }

                add_task_to_queue(task);
            }
            Trap::Interrupt(Interrupt::SupervisorExternal) => {
                if let Some(irq) = plic_claim() {
                    match irq as usize {
                        E1000_IRQ => {
                            e1000::handle_interrupt();
                        }
                        _ => {}
                    }
                    plic_complete(irq);
                }
                add_task_transient(task);
            }
            Trap::Exception(Exception::UserEnvCall) => {
                use thread::TaskStatus::*;
                if let Some(task) = syscall::handle_syscall(task) {
                    if task.status() != Blocking {
                        add_task_transient(task);
                    } else {
                        add_task_to_queue(task);
                    }
                }
            }
            _ => {
                log::info!(
                    "{:#?}, spec {:x}, stval {:x}",
                    scause::read().cause(),
                    sepc::read(),
                    stval::read()
                );
            }
        };
    }
    log::error!("Shutdown\n");
    system_reset(Shutdown, NoReason);
    unreachable!()
}
