#![no_std]
#![feature(naked_functions, asm_sym, asm_const)]
#![feature(linkage)]
#![feature(unboxed_closures, fn_traits)]

mod e1000;
mod pci;
mod thread;
mod timer;
extern crate alloc;

use alloc::boxed::Box;
use kernel_context::LocalContext;
pub use platform::Platform;
use qemu_virt_ld as linker;
pub use Virt as PlatformImpl;

use alloc::format;
use alloc::vec::Vec;
use alloc::{collections::LinkedList, sync::Arc, vec};
use core::fmt::Arguments;
use core::time::Duration;
use riscv::register::*;
use sbi_rt::*;
use spin::{Mutex, Once};
use stdio::log::info;
use stdio::*;
use thread::*;
use timer::*;
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

    // common 中库由 platform 负责初始化
    // mem
    let (heap_base, heap_size) = Virt::heap();
    mem::init_heap(heap_base, heap_size);

    unsafe {
        UART0.call_once(|| (unsafe { MmioSerialPort::new(0x1000_0000) }));
    }

    // stdio
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&Stdio);

    pci::pci_init();
    log::info!("init kthread");

    Virt::spawn(obj_main);

    // idle thread
    Virt::spawn(|| loop {
        // if net is used
        e1000::async_recv();
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

pub struct Virt;

// unsafe: 暂时未找到使用 Mutex 很好的办法
// 1. 自定义 Mutex, 在 lock 失败时让出 CPU
// 2. 使用 try_lock, lock 失败让出 CPU
static mut UART0: Once<MmioSerialPort> = Once::new();

impl platform::Platform for Virt {
    #[inline]
    fn console_getchar() -> u8 {
        unsafe { UART0.get_mut().unwrap().receive() }
    }

    #[inline]
    fn console_putchar(c: u8) {
        unsafe {
            UART0.get_mut().unwrap().send(c);
        }
    }

    #[inline]
    fn net_receive(buf: &mut [u8]) -> usize {
        e1000::recv(buf)
    }

    #[inline]
    fn net_transmit(buf: &mut [u8]) {
        e1000::send(buf);
    }

    #[inline]
    fn net_can_send() -> bool {
        e1000::can_send()
    }

    #[inline]
    fn net_can_recv() -> bool {
        e1000::can_recv()
    }

    #[inline]
    fn schedule_with_delay<F>(_delay: core::time::Duration, mut _cb: F)
    where
        F: 'static + FnMut() + Send + Sync,
    {
        Self::spawn(move || loop {
            _cb();
            Self::wait(_delay);
        });
    }

    // thread
    #[inline]
    fn spawn<F>(f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(f);
    }

    #[inline]
    fn wait(_delay: core::time::Duration) {
        sys_sleep(_delay.as_millis() as _);
    }

    #[inline]
    fn heap() -> (usize, usize) {
        let layout = linker::KernelLayout::locate();
        (layout.end(), MEMORY - layout.len())
    }

    #[inline]
    fn frequency() -> usize {
        timer::CLOCK_FREQ
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

struct Stdio;
impl stdio::Stdio for Stdio {
    #[inline]
    fn put_char(&self, c: u8) {
        Virt::console_putchar(c);
    }

    #[inline]
    fn put_str(&self, s: &str) {
        Virt::console_put_str(s);
    }

    #[inline]
    fn get_char(&self) -> u8 {
        Virt::console_getchar()
    }
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

            use scause::{Exception, Interrupt, Trap};
            let finish = match scause::read().cause() {
                Trap::Interrupt(Interrupt::SupervisorTimer) => {
                    set_timer(u64::MAX);
                    check_timer(); // 检查到时线程
                    false
                }
                Trap::Exception(Exception::Breakpoint) => {
                    ctx.lock().move_next();
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
