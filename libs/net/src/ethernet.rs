use mem::alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::socket::TcpSocketBuffer;
use smoltcp::Result;

use spin::Mutex;

use stdio::println;
use var_bitmap::Bitmap;

pub type TcpSocket = smoltcp::socket::TcpSocket<'static>;
pub type Interface<T> = smoltcp::iface::Interface<'static, T>;
pub type InterfaceInner = smoltcp::iface::Context<'static>;
pub use smoltcp::time::Instant as Instant;
pub use smoltcp::time::Duration as Duration;
pub use smoltcp::iface::SocketHandle as SocketHandle;
pub use smoltcp::socket::TcpState as TcpState;

/// A loopback device.
#[derive(Debug)]
pub struct Loopback {
    queue: VecDeque<Vec<u8>>,
    medium: Medium,
}

#[allow(clippy::new_without_default)]
impl Loopback {
    /// Creates a loopback device.
    ///
    /// Every packet transmitted through this device will be received through it
    /// in FIFO order.
    pub fn new(medium: Medium) -> Loopback {
        Loopback {
            queue: VecDeque::new(),
            medium,
        }
    }
}

impl<'a> Device<'a> for Loopback {
    type RxToken = RxToken;
    type TxToken = TxToken<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 65535;
        caps.max_burst_size = Some(1);
        caps.medium = self.medium;
        caps
    }

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        self.queue.pop_front().map(move |buffer| {
            let rx = RxToken { buffer };
            let tx = TxToken {
                queue: &mut self.queue,
            };
            (rx, tx)
        })
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            queue: &mut self.queue,
        })
    }
}

#[doc(hidden)]
pub struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        f(&mut self.buffer)
    }
}

#[doc(hidden)]
pub struct TxToken<'a> {
    queue: &'a mut VecDeque<Vec<u8>>,
}

impl<'a> phy::TxToken for TxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = Vec::new();
        buffer.resize(len, 0);
        let result = f(&mut buffer);
        self.queue.push_back(buffer);
        result
    }
}

/// Creates and returns a new interface using `Loopback` struct.
pub fn create_interface() -> Interface<smoltcp::phy::Loopback> {
    let device = smoltcp::phy::Loopback::new(Medium::Ethernet);
    let hw_addr = smoltcp::wire::EthernetAddress::default();
    let neighbor_cache = smoltcp::iface::NeighborCache::new(BTreeMap::new());
    let ip_addrs = [];
    smoltcp::iface::InterfaceBuilder::new(device, vec![])
        .hardware_addr(hw_addr.into())
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .finalize()
}

const PORTS_NUM: usize = 65536;

pub struct EthernetDriver {
    /// Bitmap to track the port usage
    port_map: Bitmap,
    /// Internal ethernet interface
    ethernet: Interface<smoltcp::phy::Loopback>,
}

impl EthernetDriver {
    /// Creates a fresh ethernet driver.
    fn new() -> EthernetDriver {
        EthernetDriver {
            port_map: Bitmap::with_size(PORTS_NUM),
            ethernet: create_interface(),
        }
    }

    /// Polls the ethernet interface.
    /// See also `smoltcp::iface::Interface::poll()`.
    #[allow(unused)]
    fn poll(&mut self, timestamp: Instant) {
        self.ethernet.poll(timestamp);
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

    pub fn get_context(&mut self, handle: SocketHandle) -> &mut InterfaceInner {
        self.ethernet.get_socket_and_context::<TcpSocket>(handle).1
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

    pub fn initialize(&self) {
        let mut lock = self.0.lock();
        *lock = Some(EthernetDriver::new());
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
