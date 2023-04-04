#![allow(non_camel_case_types)]
use crate::EXECUTOR;
use core::{
    future::{Future},
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Clone, Copy)]
pub enum IRQ {
    UART0_IRQ,
    VIRTIO0_IRQ,
    E1000_IRQ,
}

struct IRQ_EVENT {
    irq: IRQ,
    ready: bool,
}

impl IRQ_EVENT {
    pub fn new(irq: IRQ) -> Self {
        Self {
            irq: irq,
            ready: false,
        }
    }
}

impl Future for IRQ_EVENT {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let irq = self.irq;
        if self.ready {
            Poll::Ready(())
        } else {
            self.get_mut().ready = true;
            EXECUTOR.wait().sys_register_irq(cx, irq);
            Poll::Pending
        }
    }
}

pub async fn async_wait_irq(irq: IRQ) {
    IRQ_EVENT::new(irq).await
}
