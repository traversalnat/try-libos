extern crate alloc;
use alloc::vec::Vec;
use core::alloc::{Allocator};
use alloc::alloc::Global;

const INITIAL_CAPACITY: usize = 7; // 2^3 - 1

/// 最小堆, 支持 Allocator
pub struct Heap<T, A: Allocator = Global> {
    data: Vec<T, A>,
}

impl<T: PartialOrd> Heap<T> {
    /// Create a new empty heap.
    pub fn new() -> Heap<T> {
        Heap {
            data: Vec::<T>::new(),
        }
    }
}

impl<T: PartialOrd, A: Allocator> Heap<T, A> {
    pub fn new_in(alloc: A) -> Heap<T, A> {
        Heap {
            data: Vec::<T, A>::with_capacity_in(INITIAL_CAPACITY, alloc),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Remove the top item from the heap and return it,
    /// or None if the heap is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() {
            None
        } else {
            let i = self.data.len() - 1;
            self.data.as_mut_slice().swap(0, i);
            let result = self.data.remove(i);
            self.sift_down(0);
            return Some(result);
        }
    }

    /// Return a clone of the top item in the heap, or None if the heap is
    /// empty. Does not modify the heap.
    pub fn peek(&self) -> Option<&T> {
        if self.data.is_empty() {
            None
        } else {
            self.data.get(0)
        }
    }

    /// Insert the given value into the heap. The heap will automatically
    /// grow capacity if necessary.
    pub fn push(&mut self, value: T) {
        // Double the capacity of the heap if it is currently full.
        if self.data.len() == self.data.capacity() {
            let cap = self.data.capacity();
            self.data.reserve_exact(cap);
        }
        self.data.push(value);
        let i = self.data.len() - 1;
        self.sift_up(i);
    }

    pub fn insert_vec(&mut self, values: Vec<T>) {
        for value in values.into_iter() {
            self.push(value);
        }
    }

    /// Shrink the capacity of the heap as much as possible,
    /// so that there is no unused capacity.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    fn sift_up(&mut self, i: usize) {
        match parent_index(i) {
            Some(parent) => {
                if self.data.get(i).lt(&self.data.get(parent)) {
                    self.data.as_mut_slice().swap(i, parent);
                    self.sift_up(parent);
                }
            }
            None => {}
        }
    }

    fn sift_down(&mut self, i: usize) {
        let len = self.data.len();
        let l = left_child_index(i);
        let r = right_child_index(i);

        let child = if l >= len && r >= len {
            return;
        }
        // no children, nothing to do
        else if l >= len {
            r
        } else if r >= len {
            l
        } else if self.data.get(l).lt(&self.data.get(r)) {
            l
        } else {
            r
        };

        if self.data.get(child).lt(&self.data.get(i)) {
            self.data.as_mut_slice().swap(child, i);
            self.sift_down(child);
        }
    }
}

/// Returns the index of the given index's parent,
/// or None if it has no parent (i.e. i is 0).
fn parent_index(i: usize) -> Option<usize> {
    if i == 0 {
        None
    } else {
        Some((i - 1) / 2)
    }
}

/// Returns the index of the given index's left child. The returned index
/// may be larger than the current capacity of the heap.
fn left_child_index(i: usize) -> usize {
    (i + 1) * 2 - 1
}

/// Returns the index of the given index's right child. The returned index
/// may be larger than the current capacity of the heap.
fn right_child_index(i: usize) -> usize {
    (i + 1) * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_heap() {
        let mut heap: Heap<usize> = Heap::new();
        heap.push(3);
        heap.push(4);
        heap.push(1);
        heap.push(9);
        heap.push(2);
        assert_eq!(Some(1), heap.pop());
        assert_eq!(Some(2), heap.pop());
        assert_eq!(Some(3), heap.pop());
        assert_eq!(Some(4), heap.pop());
        assert_eq!(Some(9), heap.pop());
        assert_eq!(None, heap.pop());
    }
}
