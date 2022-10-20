#![no_std]
#![no_main]

use core::{time::Duration};

use platform::{Platform, PlatformImpl, MACADDR};

#[no_mangle]
fn obj_main() {
    // 连接 platform 和 libs
    let (heap_base, heap_size) = PlatformImpl::heap();
    mem::init_heap(heap_base, heap_size);
    stdio::set_log_level(option_env!("LOG"));
    stdio::init(&Stdio);
    init_ethernet();
    // 初始化运行环境后，跳转至 app_main
    app::app_main();
}

fn init_ethernet() {
    net::init(&PhyNet, &MACADDR);
    // 网络栈需要不断poll
    // TODO 使用 poll_delay 来决定下一次 poll 的时间
    PlatformImpl::schedule_with_delay(Duration::from_micros(1), move || {
        let val = PlatformImpl::rdtime() as i64;
        net::ETHERNET.poll(net::Instant::from_millis(val));
        let delay = net::ETHERNET.poll_delay(net::Instant::from_millis(val));
        PlatformImpl::wait(delay.into());
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
