#![no_std]
extern crate alloc;

use core::time::Duration;
use executor::{async_yield, async_wait, async_block_on};
use spin::Lazy;
use stdio::{*, log::info};
use thread::spawn;

use alloc::boxed::Box;

pub async fn app_main() {
    use mpmc::channel;
    use core::{future::Future, pin::Pin};
    use core::hint::spin_loop;

    type AsyncFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

    let (tx, rx) = channel::<AsyncFuture>();

    for _ in 0..8 {
        let q1 = rx.clone();
        spawn(async move {
            loop {
                match q1.recv() {
                    Ok(task) => {
                        async_block_on(task);
                    },
                    _ => spin_loop(),
                }
            }
        });
    }

    for i in 0..100 {
        tx.send(Box::pin(say_hi(i))).unwrap();
        tx.send(Box::pin(say_goodbye(i))).unwrap();
    }

    loop {
        async_yield().await;
    }
}

fn fb(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    return fb(n - 1) + fb(n - 2);
}

async fn say_hi(index: u32) {
    fb(32);
    println!("hi {index}");
}

async fn say_goodbye(index: u32) {
    println!("goodbye {index}");
}
