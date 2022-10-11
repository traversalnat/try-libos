#![no_std]
#![feature(alloc_error_handler)]

#![allow(dead_code)]
#![allow(unused_imports)]

use buddy_system_allocator::LockedHeap;

#[cfg(not(feature = "std"))]
#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const KERNEL_HEAP_SIZE: usize = 0x300_0000;

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

/// heap space ([u8; KERNEL_HEAP_SIZE])
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// initiate heap allocator
pub fn init_heap() {
    #[cfg(not(feature = "std"))]
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

pub extern crate alloc;
