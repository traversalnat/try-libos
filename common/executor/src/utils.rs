/// Future yield
use core::pin::Pin;
use core::time::Duration;
use core::{
    future::Future,
    task::{Context, Poll},
};
use timer::get_time_ms;


struct Yield {
    yielded: bool,
}

impl Yield {
    pub fn new() -> Self {
        Yield { yielded: false }
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.get_mut().yielded = true;
            Poll::Pending
        }
    }
}

pub async fn async_yield() {
    Yield::new().await;
}

struct SleepFuture {
    dur: Duration
}

impl SleepFuture {
    pub fn new(dur: Duration) -> Self {
        let now = get_time_ms();
        let dur = dur.as_millis() as usize;

        Self {
            dur: Duration::from_millis((now + dur) as u64)
        }
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if get_time_ms() >= self.dur.as_millis().try_into().unwrap() {
            return Poll::Ready(());
        }
        Poll::Pending
    }
}

pub async fn async_wait(dur: Duration) {
    SleepFuture::new(dur).await;
}
