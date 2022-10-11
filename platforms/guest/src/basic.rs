pub struct MacOS;

use std::{
    io::{Read, Write},
    process::exit,
    thread,
};

use rawsock::open_best_library;

use core::time::Duration;

pub const ITERF_NAME: &str = "en0";

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
        if let Ok(lib) = open_best_library() {
            if let Ok(mut iterf) = lib.open_interface(&ITERF_NAME) {
                if let Ok(packet) = iterf.receive() {
                    buf.fill(0);
                    for i in 0..packet.len() {
                        buf[0] = packet[i];
                    }
                    return packet.len();
                }
            }
        }
        0
    }

    #[inline]
    fn net_transmit(_buf: &mut [u8]) {
        if let Ok(lib) = open_best_library() {
            if let Ok(iterf) = lib.open_interface(&ITERF_NAME) {
                iterf.send(_buf).unwrap_or(());
            }
        }
    }

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
    fn spawn<F>(f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        thread::spawn(f);
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
