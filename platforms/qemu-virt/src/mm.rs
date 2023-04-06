#![allow(dead_code)]
use core::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout},
    ptr::{slice_from_raw_parts_mut, NonNull},
};
use good_memory_allocator::SpinLockedAllocator;

use crate::trap::{pop_on, push_off};

pub(crate) static HEAP_ALLOCATOR: SpinLockedAllocator = SpinLockedAllocator::empty();

/// initiate heap allocator used by dispatcher
pub(crate) fn init_heap(_heap_base: usize, _heap_size: usize) {
    unsafe {
        HEAP_ALLOCATOR.init(_heap_base, _heap_size);
    }
}

/// 调度器专用内存分配器
pub struct KAllocator;

unsafe impl Allocator for KAllocator {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let sstatus = push_off();
        let result = unsafe {
            let ptr: *mut u8 = HEAP_ALLOCATOR.alloc(layout);
            let ptr: *mut [u8] = slice_from_raw_parts_mut(ptr, layout.size());
            match NonNull::new(ptr) {
                Some(nonull) => Ok(nonull),
                _ => Err(AllocError),
            }
        };
        pop_on(sstatus);
        result
    }

    #[inline]
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let sstatus = push_off();
        HEAP_ALLOCATOR.dealloc(ptr.as_ptr(), layout);
        pop_on(sstatus);
    }
}