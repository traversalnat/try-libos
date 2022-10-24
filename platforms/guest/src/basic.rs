pub struct MacOS;

use std::{
    io::{Read, Write},
    process::exit,
    thread,
};

use core::time::Duration;
use crate::eth::EthDevice;
use chrono::Local;
use spin::Mutex;
use lazy_static::*;

lazy_static!{
    static ref ETH_DEVICE: Mutex<EthDevice> =  {
        Mutex::new(EthDevice::new())
    };
}

const KERNEL_HEAP_SIZE: usize = 0x300_0000;
/// heap space ([u8; KERNEL_HEAP_SIZE])
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

impl platform::Platform for MacOS {
    #[inline]
    fn console_getchar() -> u8 {
        let mut buf = [0; 1];
        let mut stdin = std::io::stdin();
        if let Ok(_e) = stdin.read(&mut buf) {
            buf[0]
        } else {
            unimplemented!("stdin broken")
        }
    }

    #[inline]
    fn console_putchar(c: u8) {
        let buf = [c; 1];
        let mut stdout = std::io::stdout();
        if let Ok(_e) = stdout.write(&buf) {}
    }

    /// 构建一个 NAT 设备
    #[inline]
    fn net_receive(buf: &mut [u8]) -> usize {
        let mut eth = ETH_DEVICE.lock();
        eth.recv(buf)
    }

    #[inline]
    fn net_transmit(buf: &mut [u8]) {
        let mut eth = ETH_DEVICE.lock();
        eth.send(buf);
    }

    #[inline]
    fn schedule_with_delay<F>(_delay: Duration, mut cb: F)
    where
        F: 'static + FnMut() + Send + Sync,
    {
        thread::spawn(move || loop {
            thread::sleep(_delay);
            cb();
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
        thread::sleep(_delay);
    }

    #[inline]
    fn heap() -> (usize, usize) {
        unsafe {
            (HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE)
        }
    }

    #[inline]
    fn frequency() -> usize {
        0
    }

    #[inline]
    fn rdtime() -> usize {
        let dt = Local::now();
        dt.timestamp_millis() as usize
    }

    #[inline]
    fn shutdown(error: bool) {
        if error {
            exit(-1)
        } else {
            exit(0)
        }
    }
}
