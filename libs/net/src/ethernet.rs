use mem::alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use smoltcp::iface::{Route, Routes};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::socket::Dhcpv4Event;
use smoltcp::socket::Dhcpv4Socket;
use smoltcp::socket::TcpSocketBuffer;
use smoltcp::wire::Ipv4Address;
use smoltcp::wire::{IpCidr};
use smoltcp::Result;

use spin::Mutex;

use stdio::*;
use var_bitmap::Bitmap;

pub type TcpSocket = smoltcp::socket::TcpSocket<'static>;
pub type Interface<T> = smoltcp::iface::Interface<'static, T>;
pub type InterfaceInner = smoltcp::iface::Context<'static>;
pub use smoltcp::iface::SocketHandle;
pub use smoltcp::socket::TcpState;
pub use smoltcp::time::Duration;
pub use smoltcp::time::Instant;

use self::EthernetDevice as NetDevice;
use crate::PHYNET;

const MTU: usize = 1500;
const PORTS_NUM: usize = 65536;

#[derive(Debug)]
pub struct EthernetDevice {
    rx_buffer: [u8; MTU],
    tx_buffer: [u8; MTU],
    medium: Medium,
}

#[allow(clippy::new_without_default)]
impl EthernetDevice {
    /// Every packet transmitted through this device will be received through it
    /// in FIFO order.
    pub fn new(medium: Medium) -> EthernetDevice {
        EthernetDevice {
            rx_buffer: [0; MTU],
            tx_buffer: [0; MTU],
            medium,
        }
    }
}

impl<'a> Device<'a> for EthernetDevice {
    type RxToken = RxToken<'a>;
    type TxToken = TxToken<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = MTU;
        caps.max_burst_size = Some(1);
        caps.medium = self.medium;
        caps
    }

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        match PHYNET.get().map(|net| net.receive(&mut self.rx_buffer)) {
            Some(size) => {
                if size != 0 {
                    Some((
                        RxToken(&mut self.rx_buffer[..size]),
                        TxToken(&mut self.tx_buffer[..]),
                    ))
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken(&mut self.tx_buffer[..]))
    }
}

#[doc(hidden)]
pub struct RxToken<'a>(&'a mut [u8]);

impl<'a> phy::RxToken for RxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        f(&mut self.0)
    }
}

#[doc(hidden)]
pub struct TxToken<'a>(&'a mut [u8]);

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let result = f(&mut self.0[..len]);
        PHYNET.get().map(|net| net.transmit(&mut self.0[..len]));
        result
    }
}

pub fn create_interface(macaddr: &[u8; 6]) -> Interface<NetDevice> {
    let device = NetDevice::new(Medium::Ethernet);
    let hw_addr = smoltcp::wire::EthernetAddress::from_bytes(macaddr);
    let neighbor_cache = smoltcp::iface::NeighborCache::new(BTreeMap::new());
    let ip_addrs = [IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0)];
    static mut ROUTES_STORAGE: [Option<(IpCidr, Route)>; 1] = [None; 1];
    let routes = unsafe { Routes::new(&mut ROUTES_STORAGE[..]) };

    smoltcp::iface::InterfaceBuilder::new(device, vec![])
        .hardware_addr(hw_addr.into())
        .routes(routes)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .finalize()
}

pub struct EthernetDriver {
    /// Bitmap to track the port usage
    port_map: Bitmap,
    /// Internal ethernet interface
    ethernet: Interface<NetDevice>,
    /// Internal dhcp socket
    dhcp: SocketHandle,
}

impl EthernetDriver {
    /// Creates a fresh ethernet driver.
    fn new(macaddr: &[u8; 6]) -> EthernetDriver {
        let mut dhcp = Dhcpv4Socket::new();
        dhcp.set_max_lease_duration(Some(Duration::from_secs(10)));
        // const MACADDR: [u8; 6] = [0x12, 0x13, 0x89, 0x89, 0xdf, 0x53];
        let mut ethernet = create_interface(&macaddr);
        let dhcp = ethernet.add_socket(dhcp);

        EthernetDriver {
            port_map: Bitmap::with_size(PORTS_NUM),
            ethernet,
            dhcp,
        }
    }

    /// Polls the ethernet interface.
    /// See also `smoltcp::iface::Interface::poll()`.
    #[allow(unused)]
    fn poll(&mut self, timestamp: Instant) {
        self.ethernet.poll(timestamp);
        // poll dhcp to get ip addr and route gateway
        let dhcp = self.ethernet.get_socket::<Dhcpv4Socket>(self.dhcp);
        match dhcp.poll() {
            // TODO 请求一次就足够
            Some(Dhcpv4Event::Configured(config)) => {
                self.ethernet.update_ip_addrs(|addrs| {
                    addrs.iter_mut().next().map(|addr| {
                        *addr = IpCidr::Ipv4(config.address);
                    });
                });
                if let Some(router) = config.router {
                    self.ethernet.routes_mut().add_default_ipv4_route(router).unwrap();
                } 
                // else {
                //     self.ethernet.routes_mut().remove_default_ipv4_route();
                // }
            },
            // Some(Dhcpv4Event::Deconfigured) => {
            //     self.ethernet.update_ip_addrs(|addrs| {
            //         addrs.iter_mut().next().map(|addr| {
            //             *addr = IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0);
            //         });
            //     });
            //     self.ethernet.routes_mut().remove_default_ipv4_route();
            // },
            _ => {}
        }
    }

    /// Returns an advisory wait time to call `poll()` the next time.
    /// See also `smoltcp::iface::Interface::poll_delay()`.
    fn poll_delay(&mut self, timestamp: Instant) -> Duration {
        match self.ethernet.poll_delay(timestamp) {
            Some(dur) => dur,
            _ => Duration::from_millis(1),
        }
    }

    /// Marks a port as used. Returns `Some(port)` on success, `None` on failure.
    pub fn mark_port(&mut self, port: u16) -> Option<u16> {
        if self.port_map.get(port.into()) || port as usize >= PORTS_NUM {
            None
        } else {
            self.port_map.set(port.into(), true);
            Some(port)
        }
    }

    /// Clears used bit of a port. Returns `Some(port)` on success, `None` on failure.
    pub fn erase_port(&mut self, port: u16) -> Option<u16> {
        if self.port_map.get(port.into()) {
            Some(port)
        } else {
            None
        }
    }

    /// Returns the first open port between the ephemeral port range 49152 ~ 65535.
    /// Note that this function does not mark the returned port.
    pub fn get_ephemeral_port(&mut self) -> Option<u16> {
        for port in 49152..=65535 {
            if self.port_map.get(port) == false {
                return Some(port as u16);
            }
        }
        None
    }

    /// Finds a socket with a `SocketHandle`.
    pub fn get_socket(&mut self, handle: SocketHandle) -> &mut TcpSocket {
        self.ethernet.get_socket::<TcpSocket>(handle)
    }

    pub fn get_socket_and_context(
        &mut self,
        handle: SocketHandle,
    ) -> (&mut TcpSocket, &mut InterfaceInner) {
        self.ethernet.get_socket_and_context::<TcpSocket>(handle)
    }

    /// This function creates a new TCP socket, adds it to the internal socket
    /// set, and returns the `SocketHandle` of the new socket.
    pub fn add_socket(&mut self) -> SocketHandle {
        let rx_buffer = TcpSocketBuffer::new(vec![0; 16384]);
        let tx_buffer = TcpSocketBuffer::new(vec![0; 16384]);
        let tcp_socket = TcpSocket::new(rx_buffer, tx_buffer);
        self.ethernet.add_socket(tcp_socket)
    }

    /// Releases a socket from the internal socket set.
    pub fn release(&mut self, handle: SocketHandle) {
        self.ethernet.remove_socket(handle);
    }
}

/// A thread-safe wrapper for `EthernetDriver`.
pub struct GlobalEthernetDriver(Mutex<Option<EthernetDriver>>);

impl GlobalEthernetDriver {
    pub const fn uninitialized() -> GlobalEthernetDriver {
        GlobalEthernetDriver(Mutex::new(None))
    }

    pub fn initialize(&self, macaddr: &[u8; 6]) {
        let mut lock = self.0.lock();
        *lock = Some(EthernetDriver::new(macaddr));
    }

    pub fn poll(&self, timestamp: Instant) {
        self.0
            .lock()
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .poll(timestamp);
    }

    pub fn poll_delay(&self, timestamp: Instant) -> Duration {
        self.0
            .lock()
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .poll_delay(timestamp)
    }

    pub fn mark_port(&self, port: u16) -> Option<u16> {
        self.0
            .lock()
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .mark_port(port)
    }

    pub fn release_port(&self, port: u16) {
        self.0
            .lock()
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .erase_port(port);
    }

    // 获取短暂使用的端口
    pub fn get_ephemeral_port(&self) -> Option<u16> {
        self.0
            .lock()
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .get_ephemeral_port()
    }

    pub fn add_socket(&self) -> SocketHandle {
        self.0
            .lock()
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .add_socket()
    }

    pub fn close_socket(&self, handle: SocketHandle) {
        let mut guard = self.0.lock();
        let socket = guard
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .get_socket(handle);
        let port = socket.local_endpoint().port;
        // TODO 完全关闭 socket, 目前 close 和 abort 两个方法都无法使服务器一端断开
        socket.abort();
        self.release_port(port);
        self.critical(|eth| {
            eth.release(handle);
        });
    }

    /// Enters a critical region and execute the provided closure with a mutable
    /// reference to the socket.
    pub fn with_socket<F, R>(&self, handle: SocketHandle, f: F) -> R
    where
        F: FnOnce(&mut TcpSocket) -> R,
    {
        let mut guard = self.0.lock();
        let mut socket = guard
            .as_mut()
            .expect("Uninitialized EthernetDriver")
            .get_socket(handle);

        f(&mut socket)
    }

    /// Enters a critical region and execute the provided closure with a mutable
    /// reference to the socket.
    pub fn with_socket_and_context<F, R>(&self, handle: SocketHandle, f: F) -> R
    where
        F: FnOnce(&mut TcpSocket, &mut InterfaceInner) -> R,
    {
        let mut guard = self.0.lock();
        let (socket, cx) = guard.as_mut().unwrap().get_socket_and_context(handle);
        f(socket, cx)
    }

    /// Enters a critical region and execute the provided closure with a mutable
    /// reference to the inner ethernet driver.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut EthernetDriver) -> R,
    {
        let mut guard = self.0.lock();
        let mut ethernet = guard.as_mut().expect("Uninitialized EthernetDriver");

        f(&mut ethernet)
    }
}
