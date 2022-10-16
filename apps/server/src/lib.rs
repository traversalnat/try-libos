#![no_std]

use alloc::{borrow::ToOwned, string::String, vec};
use mem::*;
use net::*;
use stdio::*;

fn get_line() -> String {
    let mut buffer: String = String::new();
    loop {
        match get_char() {
            b'\r' | b'\n' => {
                buffer.push('\n');
                break;
            },
            ch => buffer.push(ch as char)
        }
    }
    buffer
}

pub fn app_main() {
    let sender = sys_sock_create();
    let receiver = sys_sock_create();
    // let remote_endpoint = IpEndpoint::new(IpAddress::v4(10, 42, 0, 1), 6000);
    // let remote_endpoint = IpEndpoint::new(IpAddress::v4(49, 235, 113, 66), 6000);
    // let remote_endpoint = IpEndpoint::new(IpAddress::v4(127, 0, 0, 1), 6000);
    let remote_endpoint = IpEndpoint::new(IpAddress::v4(192, 168, 1, 110), 6000);
    // sys_sock_listen(sender, 6000).unwrap();
    // println!("listend");
    if let Ok(_) = sys_sock_connect(receiver, remote_endpoint) {};
    println!("connected");

    unsafe {
            let mut tx: String = "hello, world".to_owned();
            let mut rx = vec![0 as u8; tx.len()];

            println!("read status");
            while !sys_sock_status(receiver).can_send {
                // println!("cannot send!");
            }
            println!("sending");
            if let Some(size) = sys_sock_send(receiver, tx.as_bytes_mut()) {
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
