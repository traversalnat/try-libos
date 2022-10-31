#![no_std]

mod net_io;

use alloc::{borrow::ToOwned, string::String, vec};
use executor::{block_on, spawn, join};
use mem::*;
use net::*;
use net_io::{async_recv, async_send};
use stdio::*;

async fn echo(sender: SocketHandle) {
    loop {
        let mut rx = vec![0; 1024];
        let mut recv_size = 0;
        if let Some(size) = async_recv(sender, rx.as_mut_slice()).await {
            println!("receive {size} words");
            recv_size = size;
        }
        if let Some(size) = async_send(sender, &mut rx[..recv_size]).await {
            println!("send {size} words");
        }
    }
}

pub fn app_main() {
    let sender1 = sys_sock_create();
    let sender2 = sys_sock_create();
    sys_sock_listen(sender1, 6000);
    sys_sock_listen(sender2, 6001);
    println!("listening");

    unsafe {
        let handle_1 = spawn(echo(sender1));
        let handle_2 = spawn(echo(sender2));
        block_on(async move {
            join!(handle_1, handle_2);
        });
    };
}
