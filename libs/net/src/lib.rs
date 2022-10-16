#![no_std]

mod ethernet;

use core::result::Result;
pub use ethernet::Duration as Duration;
pub use ethernet::Instant as Instant;
pub use ethernet::SocketHandle;
pub use smoltcp::wire::{IpAddress, IpEndpoint};
use ethernet::GlobalEthernetDriver;
use spin::Once;

/// 这个接口定义了网络物理层receive, transmit
pub trait PhyNet: Sync {
    // 将数据全部发送出去
    fn receive(&self, buf: &mut [u8]) -> usize;
    fn transmit(&self, buf: &mut [u8]);
}

// 网络物理设备
static PHYNET: Once<&'static dyn PhyNet> = Once::new();

pub static ETHERNET: GlobalEthernetDriver = GlobalEthernetDriver::uninitialized();

/// 主要是给 obj 确认使用哪个 platform 提供的函数来注入 PhyNet
pub fn init(net: &'static dyn PhyNet) {
    PHYNET.call_once(|| net);
    ETHERNET.initialize();
}

pub struct SocketState {
    pub is_active: bool,
    pub is_listening: bool,
    pub can_send: bool,
    pub can_recv: bool,
}

pub fn sys_sock_create() -> SocketHandle {
    ETHERNET.add_socket()
}

pub fn sys_sock_status(sock: SocketHandle) -> SocketState {
    ETHERNET.with_socket(sock, |socket| SocketState {
        is_active: socket.is_active(),
        is_listening: socket.is_listening(),
        can_send: socket.can_send(),
        can_recv: socket.can_recv(),
    })
}

pub fn sys_sock_connect(
    sock: SocketHandle,
    remote_endpoint: impl Into<IpEndpoint>,
) -> Result<(), smoltcp::Error> {
    if let Some(port) = ETHERNET.get_ephemeral_port() {
        ETHERNET
            .with_socket_and_context(sock, |socket, cx| socket.connect(cx, remote_endpoint, port))
    } else {
        Err(smoltcp::Error::NotSupported)
    }
}

pub fn sys_sock_listen(sock: SocketHandle, local_port: u16) -> Result<(), smoltcp::Error> {
    if let Some(port) = ETHERNET.mark_port(local_port) {
        ETHERNET.with_socket(sock, |socket| socket.listen(port))
    } else {
        Err(smoltcp::Error::NotSupported)
    }
}

// -> Result<usize, smoltcp::Error>
pub fn sys_sock_send(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    ETHERNET.with_socket(sock, |socket| {
        if socket.can_send() {
            match socket.send_slice(va) {
                Ok(size) => Some(size),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Receives data from a connected socket.
pub fn sys_sock_recv(sock: SocketHandle, va: &mut [u8]) -> Option<usize> {
    ETHERNET.with_socket(sock, |socket| {
        if socket.can_recv() {
            match socket.recv_slice(va) {
                Ok(size) => Some(size),
                _ => None,
            }
        } else {
            None
        }
    })
}
