#![no_std]

extern crate alloc;

use alloc::{
    alloc::{alloc, dealloc},
    sync::Arc,
    vec,
    vec::Vec,
};

use core::alloc::Layout;
use spin::{Lazy, Mutex};

use isomorphic_drivers::net::ethernet::intel::e1000::E1000;
use isomorphic_drivers::net::ethernet::structs::EthernetAddress;
use isomorphic_drivers::provider;

pub const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];

/// 使用 E1000<provider> 作为driver: provider 需要提供 alloc_dma 和 dealloc_dma功能
/// 发送可以使用 driver 的 send 方法
/// 接收: 中断来临时才可以收到数据, 如何处理中断
///

// pub static DRIVER: Lazy<Mutex<E1000<Provider>>> =
//     Lazy::new(|| Mutex::new(E1000::new(header, size, mac)));

pub fn send(buf: &[u8]) -> usize {
    0
}

// pub fn recv() -> &[u8] {
//     &[1, 2, 3]
// }
//

pub fn e1000_init(addr: u32) {

}
