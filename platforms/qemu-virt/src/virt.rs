use crate::e1000;
use crate::thread;
use crate::timer;
use platform::Platform;
use qemu_virt_ld as linker;
use sbi_rt::*;
use spin::Once;
use uart_16550::MmioSerialPort;

pub const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];

// 物理内存容量
const MEMORY: usize = 24 << 20;

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
        timer::sys_sleep(_delay.as_millis() as _);
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

    fn sys_yield(&self) {
        Virt::sys_yield();
    }
}