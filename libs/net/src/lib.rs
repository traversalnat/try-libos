#![no_std]

mod ethernet;
mod net_io;
mod socket;

extern crate alloc;
use alloc::{borrow::ToOwned, fmt, format, string::String};
use core::result::Result;
use ethernet::GlobalEthernetDriver;
pub use smoltcp::socket::TcpState;
pub use ethernet::{Duration, Instant, SocketHandle};
pub use smoltcp::wire::{IpAddress, IpEndpoint};
pub use socket::TcpListener;
use spin::Once;

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
    pub is_establised: bool,
    pub is_close_wait: bool,
    pub can_send: bool,
    pub can_recv: bool,
    pub state: TcpState,
}

impl fmt::Debug for SocketState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SocketState")
            .field("is_active", &self.is_active)
            .field("is_listening", &self.is_listening)
            .field("is_establised", &self.is_establised)
            .field("is_close_wait", &self.is_close_wait)
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
        is_establised: socket.state() == TcpState::Established,
        is_close_wait: socket.state() == TcpState::CloseWait,
        can_send: socket.can_send(),
        can_recv: socket.can_recv(),
        state: socket.state(),
    })
}

pub fn sys_sock_connect(
    sock: SocketHandle,
    remote_endpoint: impl Into<IpEndpoint>,
) -> Result<(), String> {
    if let Some(port) = ETHERNET.get_ephemeral_port() {
        ETHERNET.mark_port(port).unwrap();
        return ETHERNET.with_socket_and_context(sock, |socket, cx| {
            socket
                .connect(cx, remote_endpoint, port)
                .map_err(|err| format!("{:?}", err))
        });
    } else {
        return Err("No ephemeral port".to_owned());
    }
}

pub fn sys_sock_listen(sock: SocketHandle, local_port: u16) -> Option<TcpListener> {
    if let Some(port) = ETHERNET.mark_port(local_port) {
        return Some(TcpListener::new(sock, port));
    }
    None
}

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

/// Close a connected socket.
pub fn sys_sock_close(sock: SocketHandle) {
    ETHERNET.close_socket(sock);
}

/// async version
pub use net_io::*;
