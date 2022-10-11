#![no_std]

mod ethernet;

use core::result::Result;
pub use smoltcp::wire::{IpAddress as IpAddress, IpEndpoint as IpEndpoint};
// pub type TcpSocket = ethernet::TcpSocket;
// pub type Interface<T> = ethernet::Interface<T>;
pub use ethernet::Duration;
pub use ethernet::Instant;
pub use ethernet::SocketHandle;
pub use ethernet::TcpState as TcpState;

use ethernet::GlobalEthernetDriver;

use spin::Once;
use stdio::println;

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
    // TODO 使用 PHYNET 提供的发送、接收 raw packet 的方法重写 Loopback 设备
    PHYNET.call_once(|| net);
    ETHERNET.initialize();
}

// TODO 提供与 socket 交互的 api

pub fn sys_sock_create() -> SocketHandle {
    ETHERNET.add_socket()
}

pub fn sys_sock_status(sock: SocketHandle) -> TcpState {
    ETHERNET.with_socket(sock, |socket| {
        socket.state()
    })
}

pub fn sys_sock_connect(sock: SocketHandle, remote_endpoint: impl Into<IpEndpoint>) -> Result<(), smoltcp::Error> {
    if let Some(port) = ETHERNET.get_ephemeral_port() {
        let local_endpoint =
            IpEndpoint::new(IpAddress::v4(127, 0, 0, 1), port);
        ETHERNET.critical(|eth| {
            let cx = eth.get_context(sock);
            ETHERNET.with_socket(sock, |socket| {
                socket.connect(cx, remote_endpoint, local_endpoint)
            })
        })
    } else {
        Err(smoltcp::Error::NotSupported)
    }
}

pub fn sys_sock_listen(sock: SocketHandle, local_port: u16) -> Result<(), smoltcp::Error> {
    if let Some(port) = ETHERNET.mark_port(local_port) {
        let local_endpoint = IpEndpoint::new(IpAddress::v4(127, 0, 0, 1), port);
        ETHERNET.with_socket(sock, |socket| {
            socket.listen(local_endpoint)
        })
    } else {
        Err(smoltcp::Error::NotSupported)
    }
}

pub fn sys_sock_send(sock: SocketHandle, va: &mut [u8]) -> Result<usize, smoltcp::Error> {
    ETHERNET.with_socket(sock, |socket| {
        socket.send_slice(va)
    })
}

/// Receives data from a connected socket.
pub fn sys_sock_recv(sock: SocketHandle, va: &mut [u8]) -> Result<usize, smoltcp::Error> {
    ETHERNET.with_socket(sock, |socket| {
        socket.recv_slice(va)
    })
}

pub fn sys_sock_poll() {
    ETHERNET.poll(Instant::from_millis(0));
}
