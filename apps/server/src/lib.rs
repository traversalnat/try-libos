#![no_std]

use alloc::{borrow::ToOwned, string::String, vec};
use mem::*;
use net::*;
use stdio::*;

pub fn app_main() {
    let sender = sys_sock_create();
    let receiver = sys_sock_create();
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(10, 42, 117, 199), 6000);
    sys_sock_listen(sender, 6000).unwrap();
    if let Ok(_) = sys_sock_connect(receiver, remote_endpoint) {};
    println!("connected");

    unsafe {
            let mut tx: String = "hello, world".to_owned();
            let mut rx = vec![0 as u8; tx.len()];

            println!("read status");
            while !sys_sock_status(sender).can_send {
                // println!("cannot send!");
            }
            println!("sending");
            if let Some(size) = sys_sock_send(sender, tx.as_bytes_mut()) {
                println!("send {size} words");
            }

            while !sys_sock_status(receiver).can_recv {
                // println!("cannot recv");
            }
            println!("recving");
            if let Some(size) = sys_sock_recv(receiver, rx.as_mut_slice()) {
                println!("receive {size} words");
            }
            println!("{}", core::str::from_utf8_unchecked(&rx));
    };
}
