#![no_std]
#![no_main]
#![allow(unused)]
#![feature(fn_align)]

extern crate alloc;

use alloc::boxed::Box;
use executor::{async_spawn, async_yield};
use core::pin::Pin;
use core::{future::Future, time::Duration};

use platform::{Platform, PlatformImpl, MACADDR};
use stdio::log::info;

#[no_mangle]
#[repr(align(2))]
fn obj_main() {
    net::init(&PhyNet, &MACADDR);
    init_ethernet();
    thread::init(&ThreadImpl);
    app::app_main();
}

fn init_ethernet() {
    // 网络栈需要不断poll
    PlatformImpl::spawn(async {
        loop {
            let val = PlatformImpl::rdtime() as i64;
            net::ETHERNET.poll(net::Instant::from_millis(val));
            async_yield();
        }
    });
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    stdio::log::error!("{info}");
    // PlatformImpl::shutdown(true);
    // unreachable!();
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
    fn spawn(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>) -> usize {
        PlatformImpl::spawn(f)
    }
    fn append_task(&self, f: Pin<Box<dyn Future<Output = ()> + Send>>) -> usize {
        PlatformImpl::append_task(f)
    }

    fn yields(&self) {
        PlatformImpl::sys_yield();
    }
}
