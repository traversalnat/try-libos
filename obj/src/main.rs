#![no_std]
#![no_main]
#![allow(unused)]
#![feature(fn_align)]

extern crate alloc;

use alloc::boxed::Box;
use core::{future::Future, pin::Pin, time::Duration};

use platform::{Platform, PlatformImpl, MACADDR};
use stdio::log::info;

#[no_mangle]
#[repr(align(2))]
fn obj_main() {
    init_ethernet();
    thread::init(&ThreadImpl);
    PlatformImpl::spawn(async { app::app_main().await }, true);
}

fn init_ethernet() {
    net::init(&PhyNet, &MACADDR);
    // 网络栈需要不断poll
    PlatformImpl::spawn(
        async {
            loop {
                let val = PlatformImpl::rdtime() as i64;
                net::ETHERNET.poll(net::Instant::from_millis(val));
                let delay = Duration::from_millis(100);
                PlatformImpl::wait(delay);
            }
        },
        true,
    );
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    stdio::log::error!("{info}");
    loop {}
    // PlatformImpl::shutdown(true);
    // unreachable!();
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
    fn spawn(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>, is_io: bool) -> usize {
        PlatformImpl::spawn(f, is_io)
    }
    fn append_task(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>) -> usize {
        PlatformImpl::append_task(f)
    }

    fn yields(&self) {
        PlatformImpl::sys_yield();
    }
}
