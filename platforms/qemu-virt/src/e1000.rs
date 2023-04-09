extern crate alloc;
use alloc::{
    alloc::{alloc, dealloc},
};

use core::alloc::Layout;
use isomorphic_drivers::{
    net::ethernet::{intel::e1000::E1000, structs::EthernetAddress as DriverEthernetAddress},
    provider,
};
use spin::{Lazy, Mutex};


use futures::task::AtomicWaker;

pub static ASYNC_WAIT_WAKER: AtomicWaker = AtomicWaker::new();

// pub const E1000_IRQ: usize = 33;

pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = 4096;

    fn alloc_dma(size: usize) -> (usize, usize) {
        let layout = Layout::from_size_align(size, Self::PAGE_SIZE).unwrap();
        let paddr = unsafe { alloc(layout) as usize };
        (paddr, paddr)
    }

    fn dealloc_dma(vaddr: usize, size: usize) {
        let layout = Layout::from_size_align(size, Self::PAGE_SIZE).unwrap();
        unsafe {
            dealloc(vaddr as *mut u8, layout);
        }
    }
}

pub static E1000_DRIVER: Lazy<Mutex<Option<E1000<Provider>>>> = Lazy::new(|| Mutex::new(None));

pub fn init(header: usize, size: usize) {
    let e1000 = E1000::new(
        header,
        size,
        DriverEthernetAddress::from_bytes(&crate::MACADDR),
    );

    let mut lock = E1000_DRIVER.lock();
    *lock = Some(e1000);
}

#[inline]
pub fn has_interrupt() -> bool {
    E1000_DRIVER
        .lock()
        .as_mut()
        .expect("E1000 Driver uninit")
        .handle_interrupt()
}

pub fn recv(buf: &mut [u8]) -> usize {
    if let Some(block) = E1000_DRIVER
        .lock()
        .as_mut()
        .expect("E1000 Driver uninit")
        .receive()
    {
        let len = core::cmp::min(buf.len(), block.len());
        buf[..len].copy_from_slice(&block[..len]);
        return len;
    }
    0
}

pub fn can_send() -> bool {
    E1000_DRIVER
        .lock()
        .as_mut()
        .expect("E1000 Driver uninit")
        .can_send()
}

pub fn can_recv() -> bool {
    has_interrupt()
}

pub fn send(buf: &[u8]) {
    E1000_DRIVER
        .lock()
        .as_mut()
        .expect("E1000 Driver uninit")
        .send(buf);
}

pub fn handle_interrupt() {
    ASYNC_WAIT_WAKER.wake();
}
