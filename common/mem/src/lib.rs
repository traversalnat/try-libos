#![no_std]
#![feature(alloc_error_handler)]

#![allow(dead_code)]
#![allow(unused_imports)]

use buddy_system_allocator::LockedHeap;

#[cfg(not(feature = "std"))]
#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

/// initiate heap allocator
pub fn init_heap(_heap_base: usize, _heap_size: usize) {
    #[cfg(not(feature = "std"))]
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(_heap_base, _heap_size);
    }
}

pub extern crate alloc;
