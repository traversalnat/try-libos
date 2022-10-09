#![no_std]
#![no_main]

use platform::{Platform, PlatformImpl};

#[no_mangle]
fn obj_main() {
    mem::init_heap();
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&Stdio);
    net::init(&PhyNet);
    app::app_main();
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    stdio::log::error!("{info}");
    PlatformImpl::shutdown(true);
    loop {}
}

struct Stdio;
struct PhyNet;

impl stdio::Stdio for Stdio {
    #[inline]
    fn put_char(&self, c: u8) {
        PlatformImpl::console_putchar(c);
    }

    #[inline]
    fn put_str(&self, s: &str) {
        PlatformImpl::console_put_str(s);
    }

    #[inline]
    fn get_char(&self) -> u8 {
        PlatformImpl::console_getchar()
    }
}

impl net::PhyNet for PhyNet {
    fn receive(&self, buf: &mut [u8]) -> usize {
        PlatformImpl::net_receive(buf)
    }

    fn transmit(&self, buf: &mut [u8]) {
        PlatformImpl::net_transmit(buf);
    }
}
