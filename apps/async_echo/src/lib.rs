#![no_std]
use executor::{Runner, async_yield};
use stdio::*;
use spin::Lazy;

pub fn app_main() {
    println!("async echo");

    static EX: Lazy<Runner> = Lazy::new(|| Runner::new());

    EX.block_on(async {
        EX.spawn(async {
            loop {
                print!("hello ");
                async_yield().await;
            }
        });

        EX.spawn(async {
            loop {
                print!("world\n");
                async_yield().await;
            }
        });
    });
}
