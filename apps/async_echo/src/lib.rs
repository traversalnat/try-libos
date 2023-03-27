#![no_std]
use core::time::Duration;

use executor::{async_yield, async_wait};
use spin::Lazy;
use stdio::*;
use thread::spawn;

pub async fn app_main() {
    println!("async echo");
    spawn(async {
        loop {
            print!("hello ");
            async_wait(Duration::from_secs(1)).await;
        }
    });

    spawn(async {
        loop {
            print!("world\n");
            async_wait(Duration::from_secs(1)).await;
        }
    });
}
