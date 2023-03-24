#![no_std]

use alloc::{string::String, vec, format};
use thread::spawn;
use log::info;
use mem::*;
use net::*;
use stdio::*;

async fn echo_client(index: usize, sender: SocketHandle) {
    let mut tx: String = format!("{index} hello, world");
    let mut rx = vec![0 as u8; tx.len()];
    loop {
        info!("{index} try send");
        if let Some(size) = async_send(sender, unsafe { tx.as_bytes_mut() }).await {
            info!("{index} send {size} words");
        }
        if let Some(_) = async_recv(sender, rx.as_mut_slice()).await {
            info!("{index} receive {tx}");
        }
        if !sys_sock_status(sender).is_active {
            info!("echo stopped");
            break;
        }
    }
    sys_sock_close(sender);
}

pub async fn app_main() {
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(47, 92, 33, 237), 6000);
    for i in 0..10 {
        let receiver = sys_sock_create();
        if let Ok(_) = async_connect(receiver, remote_endpoint).await {
            spawn(echo_client(i, receiver));
        };
    }
}
