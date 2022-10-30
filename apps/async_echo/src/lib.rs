#![no_std]
use executor::{spawn, block_on, async_yield, join};
use stdio::*;

pub fn app_main() {
    println!("async echo");

    let h1 = spawn(async {
        loop {
            print!("hello ");
            async_yield().await;
        }
    });

    let h2 = spawn(async {
        loop {
            print!("world\n");
            async_yield().await;
        }
    });

    block_on(async {
        join!(h1, h2);
    });
}
