#![no_std]

use mem::alloc;

use alloc::vec::Vec;
use alloc::vec;

pub async fn app_main() {
    let mut v = vec![1, 2, 3];
    v.push(2);
    stdio::println!("Hello, world! {}", v[2]);
}
