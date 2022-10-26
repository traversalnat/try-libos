#![no_std]

pub trait Platform {
    fn console_getchar() -> u8;
    fn console_putchar(c: u8);
    #[inline]
    fn console_put_str(str: &str) {
        for c in str.bytes() {
            Self::console_putchar(c);
        }
    }

    // net: 默认不要求实现
    fn net_receive(_buf: &mut [u8]) -> usize {
        0
    }

    fn net_transmit(_buf: &mut [u8]) {}

    fn net_can_send() -> bool {
        true
    }

    fn net_can_recv() -> bool {
        true
    }

    // timer:
    fn schedule_with_delay<F>(_delay: core::time::Duration, mut _cb: F)
    where
        F: 'static + FnMut() + Send + Sync,
    {
    }

    // thread
    fn spawn<F>(_f: F)
    where
        F: FnOnce() + Send + 'static,
    {
    }

    fn wait(_delay: core::time::Duration) {}

    // mem: return the heap base and heap size
    fn heap() -> (usize, usize) {
        (0, 0)
    }

    // machine
    fn frequency() -> usize;
    fn rdtime() -> usize;
    fn shutdown(error: bool);
}
