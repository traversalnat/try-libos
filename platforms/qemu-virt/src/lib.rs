#![no_std]
#![feature(naked_functions, asm_sym, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]

mod thread;
extern crate alloc;

use kernel_context::LocalContext;
pub use platform::Platform;
use qemu_virt_ld as linker;
pub use Virt as PlatformImpl;

use riscv::register::{mcause::Interrupt, *};
use thread::*;

use alloc::{vec, collections::LinkedList};
use alloc::vec::Vec;
use sbi_rt::*;
use spin::{Mutex, Once};
use uart_16550::MmioSerialPort;

pub const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];
// 物理内存容量
const MEMORY: usize = 24 << 20;

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

    let (heap_base, heap_size) = Virt::heap();
    mem::init_heap(heap_base, heap_size);

    Virt::console_put_str("init kthread\n");

    Virt::spawn(obj_main);

    Virt::spawn(|| loop {
        Virt::console_put_str("loop AAAAAAA\n");
    });

    Virt::spawn(|| loop {
        Virt::console_put_str("loop BBBBBBB\n");
    });

    let mut t = TaskControlBlock::ZERO;
    t.init(schedule as usize);
    unsafe {
        t.execute();
    }

    Virt::console_put_str("error shutdown\n");
    system_reset(Shutdown, NoReason);
    unreachable!()
}

pub struct Virt;

impl platform::Platform for Virt {
    #[inline]
    fn console_getchar() -> u8 {
        // 无法解决 static 变量需要 Mutex 的问题
        // 要么在时钟中断时放弃锁（复杂）
        // 要么在开始强制解除锁（成本太高）
        // 关键在于打印一个字符需要的上锁成本较高
        sbi_rt::legacy::console_getchar() as _
    }

    #[inline]
    fn console_putchar(c: u8) {
        sbi_rt::legacy::console_putchar(c as usize);
    }

    #[inline]
    fn net_receive(_buf: &mut [u8]) -> usize {
        0
    }

    #[inline]
    fn net_transmit(_buf: &mut [u8]) {}

    #[inline]
    fn schedule_with_delay<F>(_delay: core::time::Duration, mut _cb: F)
    where
        F: 'static + FnMut() + Send + Sync,
    {
        // TODO thread sleep
        Self::spawn(_cb);
    }

    // thread
    #[inline]
    fn spawn<F>(_f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut t = TaskControlBlock::ZERO;
        let address = <F as core::ops::FnOnce<()>>::call_once as usize;
        t.init(address);
        THREADS.lock().push_back(t);
    }

    #[inline]
    fn wait(_delay: core::time::Duration) {}

    #[inline]
    fn heap() -> (usize, usize) {
        let layout = linker::KernelLayout::locate();
        (layout.end(), MEMORY - layout.len())
    }

    #[inline]
    fn frequency() -> usize {
        12_500_000
    }

    #[inline]
    fn rdtime() -> usize {
        riscv::register::time::read()
    }

    #[inline]
    fn shutdown(error: bool) {
        if error {
            system_reset(Shutdown, SystemFailure);
        } else {
            system_reset(Shutdown, NoReason);
        }
    }
}

extern "C" fn schedule() -> ! {
    unsafe {
        sie::set_stimer();
    }
    while !THREADS.lock().is_empty() {
        let mut ctx = THREADS.lock().pop_front().unwrap();
        set_timer(Virt::rdtime() as u64 + 12500);
        loop {
            if ctx.finish {
                break;
            }
            unsafe {
                ctx.execute();
            }

            use scause::{Exception, Interrupt, Trap};
            let finish = match scause::read().cause() {
                Trap::Interrupt(Interrupt::SupervisorTimer) => {
                    set_timer(u64::MAX);
                    false
                }
                Trap::Exception(e) => true,
                Trap::Interrupt(ir) => true,
            };

            ctx.finish = finish;
            break;
        }
        if !ctx.finish {
            THREADS.lock().push_back(ctx);
        }
    }
    Virt::console_put_str("Shutdown\n");
    system_reset(Shutdown, NoReason);
    unreachable!()
}
