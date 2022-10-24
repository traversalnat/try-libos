#![no_std]
#![no_main]

extern crate alloc;

use core::{time::Duration};

use platform::{Platform, PlatformImpl, MACADDR};
use alloc::{vec, vec::Vec};

#[no_mangle]
fn obj_main() {
    // 连接 platform 和 libs
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

struct PhyNet;

impl net::PhyNet for PhyNet {
    fn receive(&self, buf: &mut [u8]) -> usize {
        PlatformImpl::net_receive(buf)
    }

    fn transmit(&self, buf: &mut [u8]) {
        PlatformImpl::net_transmit(buf);
    }
}
