use crate::{
    sys_sock_close, sys_sock_create, sys_sock_listen, sys_sock_status, SocketState, ETHERNET,
};
use smoltcp::{
    iface::SocketHandle,
    wire::{IpAddress, IpEndpoint, IpProtocol, IpVersion, Ipv4Address},
};
use stdio::log::{info, warn};

pub struct TcpListener {
    handle: SocketHandle,
    local_port: u16,
}

fn listen(handle: SocketHandle, port: u16) {
    ETHERNET.with_socket(handle, |socket| match socket.listen(port) {
        Err(e) => warn!("listen error"),
        _ => {}
    });
}

impl TcpListener {
    pub fn new(handle: SocketHandle, local_port: u16) -> Self {
        listen(handle, local_port);
        TcpListener {
            handle,
            local_port: local_port,
        }
    }

    pub fn accept(&mut self) -> Option<SocketHandle> {
        if sys_sock_status(self.handle).is_active {
            let new_handle = sys_sock_create();
            listen(new_handle, self.local_port);
            return Some(core::mem::replace(&mut self.handle, new_handle));
        }

        None
    }
}
