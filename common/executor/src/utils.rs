/// Future yield
use core::pin::Pin;
use core::{
    future::Future,
    task::{Context, Poll},
    time::Duration,
};
use alloc::string::String;

use alloc::boxed::Box;
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            self.get_mut().yielded = true;
            Poll::Pending
        }
    }
}

pub async fn async_yield() {
    Yield::new().await;
}

struct SleepFuture {
    dur: Duration,
}

impl SleepFuture {
    pub fn new(dur: Duration) -> Self {
        let now = get_time_ms();
        let dur = dur.as_millis() as usize;

        Self {
            dur: Duration::from_millis((now + dur) as u64),
        }
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if get_time_ms() >= self.dur.as_millis().try_into().unwrap() {
            return Poll::Ready(());
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub async fn async_wait(dur: Duration) {
    SleepFuture::new(dur).await;
}

pub async fn async_wait_some<F>(f: F) -> bool
where
    F: Fn() -> bool,
{
    while !f() {
        async_yield().await;
    }

    true
}

type PinnedFuture<R> = Pin<Box<dyn Future<Output = R> + Send + 'static>>;

struct Timeout<R> {
    dur: Duration,
    future: PinnedFuture<R>,
}

impl<R> Timeout<R> {
    pub fn new(dur: Duration, future: PinnedFuture<R>) -> Self {
        let now = get_time_ms();
        let dur = dur.as_millis() as usize;

        Self {
            dur: Duration::from_millis((now + dur) as u64),
            future: future,
        }
    }
}

impl<R> Future for Timeout<R> {
    type Output = Result<R, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let dur = self.dur;
        let check_handle = unsafe { Pin::new_unchecked(&mut self.get_mut().future) };
        match Future::poll(check_handle, cx) {
            Poll::Ready(val) => {
                return Poll::Ready(Ok(val));
            },
            _ => {
                if get_time_ms() >= dur.as_millis().try_into().unwrap() {
                    return Poll::Ready(Err("time out".into()));
                }
            }
        };
        // waker will be wake by self.future
        Poll::Pending
    }
}

pub async fn async_timeout<F, R>(f: F, dur: Duration) -> Result<R, String>
where
    F: Future<Output = R> + Send + 'static,
{
    Timeout::new(dur, Box::pin(f)).await
}
