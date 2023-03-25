extern crate alloc;
use alloc::{
    alloc::{alloc, dealloc},
    vec::{Vec},
};
use core::alloc::Layout;
use e1000_driver::e1000::{E1000Device as E1000, KernelFunc};

use spin::{Lazy, Mutex};


// pub const E1000_IRQ: usize = 33;

pub struct Provider;

impl KernelFunc for Provider {
    const PAGE_SIZE: usize = 4096;

    fn dma_alloc_coherent(&mut self, pages: usize) -> (usize, usize) {
        let paddr = unsafe {
            alloc(Layout::from_size_align(pages * Self::PAGE_SIZE, Self::PAGE_SIZE).unwrap())
                as usize
        };
        (paddr, paddr)
    }

    fn dma_free_coherent(&mut self, vaddr: usize, pages: usize) {
        unsafe {
            dealloc(
                vaddr as *mut u8,
                Layout::from_size_align(pages * Self::PAGE_SIZE, Self::PAGE_SIZE).unwrap(),
            );
        }
    }
}

pub static E1000_DRIVER: Lazy<Mutex<Option<E1000<Provider>>>> = Lazy::new(|| Mutex::new(None));

pub fn init() {
    e1000_driver::pci::pci_init();

    let provider = Provider {};

    let e1000 = e1000_driver::e1000::E1000Device::<Provider>::new(
        provider,
        e1000_driver::pci::E1000_REGS as usize,
    )
    .unwrap();

    let mut lock = E1000_DRIVER.lock();
    *lock = Some(e1000);
}

/// data arrival
pub fn handle_interrupt() -> bool {
    true
}

static RING: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn recv(buf: &mut [u8]) -> usize {
    if let Some(mut packets) = E1000_DRIVER
        .lock()
        .as_mut()
        .expect("E1000 Driver uninit")
        .e1000_recv()
    {
        RING.lock().append(&mut packets);
    }

    if let Some(packet) = RING.lock().pop() {
        let len = packet.len();
        buf[..len].copy_from_slice(&packet);
        return len;
    }
    0
}

pub fn can_send() -> bool {
    true
}

pub fn can_recv() -> bool {
    true
}

pub fn send(buf: &[u8]) {
    E1000_DRIVER
        .lock()
        .as_mut()
        .expect("E1000 Driver uninit")
        .e1000_transmit(buf);
}
