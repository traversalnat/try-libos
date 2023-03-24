#![no_std]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![allow(unused_imports)]

// 物理内存容量
// const KERNEL_HEAP_SIZE: usize = 0x30_0000;
// static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

use buddy_system_allocator::LockedHeap;

#[cfg(not(feature = "std"))]
#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
/// initiate heap allocator
pub fn init_heap(base: usize, len: usize) {
    #[cfg(not(feature = "std"))]
    unsafe {
        HEAP_ALLOCATOR.lock()
            .init(base, len);
    }
}

// use good_memory_allocator::SpinLockedAllocator;
//
// #[cfg(not(feature = "std"))]
// #[global_allocator]
// static HEAP_ALLOCATOR: SpinLockedAllocator = SpinLockedAllocator::empty();
//
//
// /// initiate heap allocator used by dispatcher
// pub fn init_heap() {
//     unsafe {
//         HEAP_ALLOCATOR.init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
//     }
// }

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub extern crate alloc;
