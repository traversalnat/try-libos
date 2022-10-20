#![no_std]
#![feature(naked_functions, asm_sym, asm_const)]
#![feature(linkage)]

mod thread;

pub use platform::Platform;
pub use Virt as PlatformImpl;
use qemu_virt_ld as linker;

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
    UART0.call_once(|| Mutex::new(unsafe { MmioSerialPort::new(0x1000_0000) }));
    obj_main();
    system_reset(RESET_TYPE_SHUTDOWN, RESET_REASON_NO_REASON);
    unreachable!()
}

pub struct Virt;

static UART0: Once<Mutex<MmioSerialPort>> = Once::new();

impl platform::Platform for Virt {
    #[inline]
    fn console_getchar() -> u8 {
        UART0.wait().lock().receive()
    }

    #[inline]
    fn console_putchar(c: u8) {
        UART0.wait().lock().send(c)
    }

    #[inline]
    fn console_put_str(str: &str) {
        let mut uart = UART0.wait().lock();
        for c in str.bytes() {
            uart.send(c);
        }
    }

    #[inline]
    fn net_receive(_buf: &mut [u8]) -> usize {
        0
    }

    #[inline]
    fn net_transmit(_buf: &mut [u8]) {
    }

    #[inline]
    fn schedule_with_delay<F>(_delay: core::time::Duration, mut _cb: F)
    where
        F: 'static + FnMut() + Send + Sync,
    {
    }

    // thread
    #[inline]
    fn spawn<F>(_f: F)
    where
        F: FnOnce() + Send + 'static,
    {
    }

    #[inline]
    fn wait(_delay: core::time::Duration) {
    }

    #[inline]
    fn heap() -> (usize, usize) {
        let layout = linker::KernelLayout::locate();
        unsafe {
            (layout.end(), MEMORY - layout.len())
        }
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
        system_reset(
            RESET_TYPE_SHUTDOWN,
            if error {
                RESET_REASON_SYSTEM_FAILURE
            } else {
                RESET_REASON_NO_REASON
            },
        );
    }
}
