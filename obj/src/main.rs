#![no_std]
#![no_main]
#![allow(unused)]

extern crate alloc;

use core::time::Duration;
use alloc::boxed::Box;

use platform::{Platform, PlatformImpl, MACADDR};
use stdio::log::info;

#[no_mangle]
fn obj_main() {
    // 连接 platform 和 libs
    init_ethernet();
    thread::init(&ThreadImpl);
    // 初始化运行环境后，跳转至 app_main
    app::app_main();
}

fn init_ethernet() {
    net::init(&PhyNet, &MACADDR);
    // 网络栈需要不断poll
    PlatformImpl::spawn(|| {
        loop {
            let val = PlatformImpl::rdtime() as i64;
            net::ETHERNET.poll(net::Instant::from_millis(val));
            let delay = Duration::from_millis(100);
            PlatformImpl::wait(delay);
        }
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

    fn can_send(&self) -> bool {
        PlatformImpl::net_can_send()
    }

    fn can_recv(&self) -> bool {
        PlatformImpl::net_can_recv()
    }
}

struct ThreadImpl;

impl thread::Thread for ThreadImpl {
    fn spawn(&self, f: Box<dyn FnOnce() + Send>) {
        PlatformImpl::spawn(f);
    }

    fn yields(&self) {
        PlatformImpl::sys_yield();
    }
}
