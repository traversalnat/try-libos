#![no_std]

mod net_io;

use alloc::{borrow::ToOwned, string::String, vec};
use mem::*;
use net::*;
use stdio::*;
use executor::{spawn, join, block_on};
use net_io::{async_recv, async_send};

pub fn app_main() {
    let sender = sys_sock_create();
    sys_sock_listen(sender, 6000);
    println!("listening");

    unsafe {
        loop {
            let handle = spawn(async move {
                let mut rx = vec![0; 1024];
                let mut recv_size = 0;
                if let Some(size) = async_recv(sender, rx.as_mut_slice()).await {
                    println!("receive {size} words");
                    recv_size = size;
                }
                if let Some(size) = async_send(sender, &mut rx[..recv_size]).await {
                    println!("send {size} words");
                }
            });

            block_on(handle);
        }
    };
}
