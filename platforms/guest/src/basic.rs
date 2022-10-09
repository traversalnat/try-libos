pub struct MacOS;

use std::{io::{Read, Write}, process::exit};

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

    #[inline]
    fn net_receive(buf: &mut [u8]) -> usize {
        // TODO 实现接收 raw 数据的方法
        // 目前 smoltcp 物理层使用 loopback 设备(本地回环), 暂不实现这个方法
        0
    }

    #[inline]
    fn net_transmit(buf: &mut [u8]) {
        // TODO 实现发送 raw 数据的方法
    }

    #[inline]
    fn frequency() -> usize {
        0
    }

    #[inline]
    fn rdtime() -> usize {
        0
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
