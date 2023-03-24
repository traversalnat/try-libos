extern crate alloc;

use crate::{e1000, tasks, timer};
use alloc::boxed::Box;
use core::future::Future;
use platform::Platform;

use sbi_rt::*;
use spin::Once;
use uart_16550::MmioSerialPort;
use qemu_virt_ld as linker;

pub const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];

pub struct Virt;

static mut UART0: Once<MmioSerialPort> = Once::new();

const MEMORY: usize = 128 << 20 - 1;

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
    fn spawn<F>(f: F) -> usize
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tasks::spawn(f);
        0
    }

    // append_task to current thread
    fn append_task<F>(f: F) -> usize
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tasks::spawn(f);
        0
    }

    #[inline]
    fn wait(_delay: core::time::Duration) {}

    #[inline]
    fn sys_yield() {}

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

    fn sys_spawn(&self, f: Box<dyn FnOnce() + Send>) {
        Virt::spawn(async { f() });
    }

    fn sys_yield(&self) {}
}
