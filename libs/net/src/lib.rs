#![no_std]

mod ethernet;
mod net_io;
mod socket;

extern crate alloc;
use alloc::{borrow::ToOwned, fmt, format, string::String};
use ethernet::GlobalEthernetDriver;
pub use ethernet::{Duration, Instant, SocketHandle};
pub use smoltcp::{
    socket::TcpState,
    wire::{IpAddress, IpEndpoint},
    Error,
};
pub use socket::TcpListener;
use spin::Once;

pub type Result<T> = core::result::Result<T, Error>;

/// 这个接口定义了网络物理层receive, transmit
pub trait PhyNet: Sync {
    fn receive(&self, buf: &mut [u8]) -> usize;
    fn transmit(&self, buf: &mut [u8]);
    fn can_send(&self) -> bool;
    fn can_recv(&self) -> bool;
}

// 网络物理设备
static PHYNET: Once<&'static dyn PhyNet> = Once::new();

pub static ETHERNET: GlobalEthernetDriver = GlobalEthernetDriver::uninitialized();

/// 主要是给 obj 确认使用哪个 platform 提供的函数来注入 PhyNet
pub fn init(net: &'static dyn PhyNet, macaddr: &[u8; 6]) {
    PHYNET.call_once(|| net);
    ETHERNET.initialize(macaddr);
}

pub struct SocketState {
    pub is_active: bool,
    pub is_listening: bool,
    pub is_open: bool,
    pub can_send: bool,
    pub can_recv: bool,
    pub state: TcpState,
}

impl fmt::Debug for SocketState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SocketState")
            .field("is_active", &self.is_active)
            .field("is_listening", &self.is_listening)
            .field("is_open", &self.is_open)
            .field("can_send", &self.can_send)
            .field("can_recv", &self.can_recv)
            .field("state", &self.state)
            .finish()
    }
}

pub fn sys_sock_create() -> SocketHandle {
    ETHERNET.add_socket()
}

pub fn sys_sock_status(sock: SocketHandle) -> SocketState {
    ETHERNET.with_socket(sock, |socket| SocketState {
        is_active: socket.is_active(),
        is_listening: socket.is_listening(),
        is_open: socket.is_open(),
        can_send: socket.can_send(),
        can_recv: socket.can_recv(),
        state: socket.state(),
    })
}

pub fn sys_sock_connect(sock: SocketHandle, remote_endpoint: impl Into<IpEndpoint>) -> Result<()> {
    if let Some(port) = ETHERNET.get_ephemeral_port() {
        ETHERNET.mark_port(port).unwrap();
        return ETHERNET
            .with_socket_and_context(sock, |socket, cx| socket.connect(cx, remote_endpoint, port));
    } else {
        return Err(Error::Unaddressable);
    }
}

pub fn sys_sock_listen(sock: SocketHandle, local_port: u16) -> Result<TcpListener> {
    if let Some(port) = ETHERNET.mark_port(local_port) {
        return Ok(TcpListener::new(sock, port)?);
    } else {
        return Err(Error::Exhausted);
    }
}

pub fn sys_sock_send(sock: SocketHandle, va: &mut [u8]) -> Result<usize> {
    ETHERNET.with_socket(sock, |socket| socket.send_slice(va))
}

/// Receives data from a connected socket.
pub fn sys_sock_recv(sock: SocketHandle, va: &mut [u8]) -> Result<usize> {
    ETHERNET.with_socket(sock, |socket| socket.recv_slice(va))
}

/// Close a connected socket.
pub fn sys_sock_close(sock: SocketHandle) {
    ETHERNET.with_socket(sock, |socket| socket.close());
}

pub fn sys_sock_release(sock: SocketHandle) {
    ETHERNET.release_socket(sock);
}

use core::task::Context;
/// async version
pub use net_io::*;

pub fn sys_sock_register_recv(cx: &mut Context<'_>, sock: SocketHandle) {
    ETHERNET.with_socket(sock, |socket| {
        socket.register_recv_waker(cx.waker());
    })
}

pub fn sys_sock_register_send(cx: &mut Context<'_>, sock: SocketHandle) {
    ETHERNET.with_socket(sock, |socket| {
        socket.register_send_waker(cx.waker());
    })
}
