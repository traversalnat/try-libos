#![no_std]
#![no_main]

use core::sync::atomic::{AtomicI64, Ordering};

use platform::{Platform, PlatformImpl};

use stdio::{log, println};

#[no_mangle]
fn obj_main() {
    // 连接 platform 和 libs
    mem::init_heap();
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&Stdio);
    init_ethernet();
    // 初始化运行环境后，跳转至 app_main
    app::app_main();
}

fn init_ethernet() {
    net::init(&PhyNet);
    let delay = net::ETHERNET.poll_delay(net::Instant::from_secs(0));
    PlatformImpl::schedule_with_delay(delay.into(), move || {
        println!("hello");
        static TIMESTAMP: AtomicI64 = AtomicI64::new(0);
        let val = TIMESTAMP.fetch_add(1, Ordering::SeqCst);
        net::ETHERNET.poll(net::Instant::from_millis(val));
    });
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
