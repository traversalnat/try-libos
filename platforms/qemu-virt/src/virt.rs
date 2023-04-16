extern crate alloc;
extern crate timer;

use crate::{consts::*, e1000, syscall::*, timer::get_time_us};
use alloc::boxed::Box;
use core::{future::Future, task::Context};
use executor::IRQ;
use platform::Platform;
use qemu_virt_ld as linker;
use sbi_rt::*;
use spin::Once;
use uart_16550::MmioSerialPort;

pub struct Virt;

// unsafe: 暂时未找到使用 Mutex 很好的办法
// 1. 自定义 Mutex, 在 lock 失败时让出 CPU
// 2. 使用 try_lock, lock 失败让出 CPU
static mut UART0: Once<MmioSerialPort> = Once::new();

pub fn init(uart: MmioSerialPort) {
    unsafe {
        UART0.call_once(|| uart);
    }
}

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

    // thread
    #[inline]
    fn spawn<F>(f: F, is_io: bool) -> usize
    where
        F: Future<Output = ()> + Send + 'static,
    {
        sys_spawn(f, is_io)
    }

    // append_task to current thread
    fn append_task<F>(f: F, tid: usize) -> usize
    where
        F: Future<Output = ()> + Send + 'static,
    {
        sys_append_task(f, tid)
    }

    #[inline]
    fn wait(_delay: core::time::Duration) {
        sys_sleep(_delay.as_millis() as _);
    }

    #[inline]
    fn sys_yield() {
        sys_yield();
    }

    #[inline]
    fn heap() -> (usize, usize) {
        let layout = linker::KernelLayout::locate();
        (layout.end(), MEMORY_SIZE - layout.len())
    }

    #[inline]
    fn frequency() -> usize {
        CLOCK_FREQ
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

pub struct Stdio;
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

pub struct Executor;
impl executor::Executor for Executor {
    fn sys_cpus(&self) -> usize {
        1
    }

    fn sys_spawn(&self, f: Box<dyn FnOnce() + Send>, is_io: bool) {
        Virt::spawn(async { f() }, is_io);
    }

    fn sys_yield(&self) {
        Virt::sys_yield();
    }

    fn sys_register_irq(&self, cx: &mut Context<'_>, irq: IRQ) {
        match irq {
            IRQ::E1000_IRQ => {
                e1000::ASYNC_WAIT_WAKER.register(cx.waker());
            }
            _ => {
                cx.waker().wake_by_ref();
            }
        }
    }
}

pub struct TimeProvider;
impl timer::Timer for TimeProvider {
    fn get_time_us(&self) -> usize {
        get_time_us()
    }
}
