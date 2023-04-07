use crate::{sys_sock_create, sys_sock_status, Result, ETHERNET};
use smoltcp::iface::SocketHandle;
use stdio::log::warn;

pub struct TcpListener {
    /// socket handle
    pub handle: SocketHandle,
    /// port
    pub local_port: u16,
}

fn listen(handle: SocketHandle, port: u16) -> Result<()> {
    ETHERNET.with_socket(handle, |socket| socket.listen(port))
}

impl TcpListener {
    pub fn new(handle: SocketHandle, local_port: u16) -> Result<Self> {
        listen(handle, local_port)?;
        Ok(TcpListener {
            handle,
            local_port: local_port,
        })
    }

    pub fn accept(&mut self) -> Result<SocketHandle> {
        let new_handle = sys_sock_create();
        listen(new_handle, self.local_port)?;
        return Ok(core::mem::replace(&mut self.handle, new_handle));
    }
}
