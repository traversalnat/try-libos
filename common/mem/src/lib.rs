#![no_std]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![allow(unused_imports)]



use good_memory_allocator::SpinLockedAllocator;
#[cfg(not(feature = "std"))]
#[global_allocator]
static HEAP_ALLOCATOR: SpinLockedAllocator = SpinLockedAllocator::empty();
/// initiate heap allocator used by dispatcher
pub fn init_heap(_heap_base: usize, _heap_size: usize) {
    unsafe {
        HEAP_ALLOCATOR.init(_heap_base, _heap_size);
    }
}

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub extern crate alloc;
