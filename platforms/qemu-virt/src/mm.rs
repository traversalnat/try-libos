#![allow(dead_code)]
#![allow(unused_imports)]

use core::alloc::{Allocator, GlobalAlloc, Layout};
use good_memory_allocator::SpinLockedAllocator;

use crate::trap::{pop_on, push_off};

pub(crate) static HEAP_ALLOCATOR: SpinLockedAllocator = SpinLockedAllocator::empty();

/// initiate heap allocator used by dispatcher
pub(crate) fn init_heap(_heap_base: usize, _heap_size: usize) {
    unsafe {
        HEAP_ALLOCATOR.init(_heap_base, _heap_size);
    }
}

struct GlobalAllocator; 
/// global allocator
#[cfg(not(feature = "std"))]
#[global_allocator]
static GLOBAL_ALLOCATOR_IMPL: GlobalAllocator = GlobalAllocator {};

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let sstatus = push_off();
        let ret = HEAP_ALLOCATOR.alloc(layout);
        pop_on(sstatus);
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let sstatus = push_off();
        let ret = HEAP_ALLOCATOR.dealloc(ptr, layout);
        pop_on(sstatus);
        ret
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let sstatus = push_off();
        let ret = HEAP_ALLOCATOR.realloc(ptr, layout, new_size);
        pop_on(sstatus);
        ret
    }
}

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub extern crate alloc;
