#![no_std]

extern crate alloc;
use alloc::vec::Vec;

// the no_std version of var-bitmap v0.1.0

pub struct Bitmap {
    size: usize,
    bits: Vec<u8>,
}

impl Bitmap {
    /// Create a new bitmap with initial size of 0
    pub fn new() -> Self {
        Self::with_size(0)
    }

    /// Create a new bitmap with initial size `size`
    pub fn with_size(size: usize) -> Self {
        let mut bits = Vec::new();
        let bits_size;
        if size == 0 {
            bits_size = 0;
        } else {
            bits_size = (size - 1) / 8 + 1;
        }
        for _ in 0..bits_size {
            bits.push(0);
        }

        Self { size, bits }
    }

    /// Get bitmap size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get bit at index `idx`, panic if `idx` out of bound
    pub fn get(&self, idx: usize) -> bool {
        if idx >= self.size {
            panic!("Out of bound error");
        }
        let byte_idx = idx >> 3; // idx / 8
        let offset = idx & 0b111; // idx % 8
        let byte = self.bits[byte_idx];
        (byte >> (7 - offset)) & 1 == 1
    }

    /// Set bit at index `idx`
    pub fn set(&mut self, idx: usize, value: bool) {
        if idx >= self.size {
            panic!("Out of bound error");
        }
        let byte_idx = idx >> 3; // idx / 8
        let offset = idx & 0b111; // idx % 8
        let mut byte = self.bits[byte_idx];

        let curval = (byte >> (7 - offset)) & 1;
        let mask;
        if value {
            mask = 1 ^ curval;
        } else {
            mask = 0 ^ curval;
        }
        byte = byte ^ (mask << (7 - offset)); // Bit flipping
        self.bits[byte_idx] = byte;
    }

    /// Push a bit
    pub fn push(&mut self, value: bool) {
        if self.size & 0b111 == 0 {
            // size % 8 == 0
            // Add new byte
            self.bits.push(0);
        }
        let idx = self.size;
        self.size += 1;
        self.set(idx, value);
    }
}
