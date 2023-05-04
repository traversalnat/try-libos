#![no_std]
#![feature(error_in_core)]
//! Multi-producer multi-consumer channels.

// This module is not currently exposed publicly, but is used
// as the implementation for the channels in `sync::mpsc`. The
// implementation comes from the crossbeam-channel crate:
//
// Copyright (c) 2019 The Crossbeam Project Developers
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

mod array;
mod counter;
mod error;
mod list;
mod select;
mod utils;
// TODO "move time to time crate"
mod time;

extern crate alloc;

use core::fmt;
use core::panic::{RefUnwindSafe, UnwindSafe};

pub use error::*;
pub use alloc::boxed::Box;

/// Creates a channel of unbounded capacity.
///
/// This channel has a growable buffer that can hold any number of messages at a time.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (s, r) = counter::new(list::Channel::new());
    let s = Sender { flavor: SenderFlavor::List(s) };
    let r = Receiver { flavor: ReceiverFlavor::List(r) };
    (s, r)
}

/// Creates a channel of bounded capacity.
///
/// This channel has a buffer that can hold at most `cap` messages at a time.
///
/// receive operations must appear at the same time in order to pair up and pass the message over.
pub fn sync_channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    assert_ne!(cap, 0);
    let (s, r) = counter::new(array::Channel::with_capacity(cap));
    let s = Sender { flavor: SenderFlavor::Array(s) };
    let r = Receiver { flavor: ReceiverFlavor::Array(r) };
    (s, r)
}

/// The sending side of a channel.
pub struct Sender<T> {
    flavor: SenderFlavor<T>,
}

/// Sender flavors.
enum SenderFlavor<T> {
    /// Bounded channel based on a preallocated array.
    Array(counter::Sender<array::Channel<T>>),

    /// Unbounded channel implemented as a linked list.
    List(counter::Sender<list::Channel<T>>),
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}

impl<T> UnwindSafe for Sender<T> {}
impl<T> RefUnwindSafe for Sender<T> {}

impl<T> Sender<T> {
    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        match &self.flavor {
            SenderFlavor::Array(chan) => chan.try_send(msg),
            SenderFlavor::List(chan) => chan.try_send(msg),
        }
    }

    pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
        match &self.flavor {
            SenderFlavor::Array(chan) => chan.send(msg, None),
            SenderFlavor::List(chan) => chan.send(msg, None),
        }
        .map_err(|err| match err {
            SendTimeoutError::Disconnected(msg) => SendError(msg),
            SendTimeoutError::Timeout(_) => unreachable!(),
        })
    }
}

// The methods below are not used by `sync::mpsc`, but
// are useful and we'll likely want to expose them
// eventually
#[allow(unused)]
impl<T> Sender<T> {
    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        match &self.flavor {
            SenderFlavor::Array(chan) => chan.is_empty(),
            SenderFlavor::List(chan) => chan.is_empty(),
        }
    }

    /// Returns `true` if the channel is full.
    pub fn is_full(&self) -> bool {
        match &self.flavor {
            SenderFlavor::Array(chan) => chan.is_full(),
            SenderFlavor::List(chan) => chan.is_full(),
        }
    }

    /// Returns the number of messages in the channel.
    pub fn len(&self) -> usize {
        match &self.flavor {
            SenderFlavor::Array(chan) => chan.len(),
            SenderFlavor::List(chan) => chan.len(),
        }
    }

    /// If the channel is bounded, returns its capacity.
    pub fn capacity(&self) -> Option<usize> {
        match &self.flavor {
            SenderFlavor::Array(chan) => chan.capacity(),
            SenderFlavor::List(chan) => chan.capacity(),
        }
    }

    /// Returns `true` if senders belong to the same channel.
    pub fn same_channel(&self, other: &Sender<T>) -> bool {
        match (&self.flavor, &other.flavor) {
            (SenderFlavor::Array(ref a), SenderFlavor::Array(ref b)) => a == b,
            (SenderFlavor::List(ref a), SenderFlavor::List(ref b)) => a == b,
            _ => false,
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        unsafe {
            match &self.flavor {
                SenderFlavor::Array(chan) => chan.release(|c| c.disconnect_senders()),
                SenderFlavor::List(chan) => chan.release(|c| c.disconnect_senders()),
            }
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let flavor = match &self.flavor {
            SenderFlavor::Array(chan) => SenderFlavor::Array(chan.acquire()),
            SenderFlavor::List(chan) => SenderFlavor::List(chan.acquire()),
        };

        Sender { flavor }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Sender { .. }")
    }
}

/// The receiving side of a channel.
pub struct Receiver<T> {
    flavor: ReceiverFlavor<T>,
}

/// Receiver flavors.
enum ReceiverFlavor<T> {
    /// Bounded channel based on a preallocated array.
    Array(counter::Receiver<array::Channel<T>>),

    /// Unbounded channel implemented as a linked list.
    List(counter::Receiver<list::Channel<T>>),
}

unsafe impl<T: Send> Send for Receiver<T> {}
unsafe impl<T: Send> Sync for Receiver<T> {}

impl<T> UnwindSafe for Receiver<T> {}
impl<T> RefUnwindSafe for Receiver<T> {}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        match &self.flavor {
            ReceiverFlavor::Array(chan) => chan.try_recv(),
            ReceiverFlavor::List(chan) => chan.try_recv(),
        }
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        match &self.flavor {
            ReceiverFlavor::Array(chan) => chan.recv(None),
            ReceiverFlavor::List(chan) => chan.recv(None),
        }
        .map_err(|_| RecvError)
    }
}

// The methods below are not used by `sync::mpsc`, but
// are useful and we'll likely want to expose them
// eventually
#[allow(unused)]
impl<T> Receiver<T> {
    /// Returns `true` if the channel is empty.
    pub fn is_empty(&self) -> bool {
        match &self.flavor {
            ReceiverFlavor::Array(chan) => chan.is_empty(),
            ReceiverFlavor::List(chan) => chan.is_empty(),
        }
    }

    /// Returns `true` if the channel is full.
    ///
    pub fn is_full(&self) -> bool {
        match &self.flavor {
            ReceiverFlavor::Array(chan) => chan.is_full(),
            ReceiverFlavor::List(chan) => chan.is_full(),
        }
    }

    /// Returns the number of messages in the channel.
    pub fn len(&self) -> usize {
        match &self.flavor {
            ReceiverFlavor::Array(chan) => chan.len(),
            ReceiverFlavor::List(chan) => chan.len(),
        }
    }

    /// If the channel is bounded, returns its capacity.
    pub fn capacity(&self) -> Option<usize> {
        match &self.flavor {
            ReceiverFlavor::Array(chan) => chan.capacity(),
            ReceiverFlavor::List(chan) => chan.capacity(),
        }
    }

    /// Returns `true` if receivers belong to the same channel.
    pub fn same_channel(&self, other: &Receiver<T>) -> bool {
        match (&self.flavor, &other.flavor) {
            (ReceiverFlavor::Array(a), ReceiverFlavor::Array(b)) => a == b,
            (ReceiverFlavor::List(a), ReceiverFlavor::List(b)) => a == b,
            _ => false,
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        unsafe {
            match &self.flavor {
                ReceiverFlavor::Array(chan) => chan.release(|c| c.disconnect_receivers()),
                ReceiverFlavor::List(chan) => chan.release(|c| c.disconnect_receivers()),
            }
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        let flavor = match &self.flavor {
            ReceiverFlavor::Array(chan) => ReceiverFlavor::Array(chan.acquire()),
            ReceiverFlavor::List(chan) => ReceiverFlavor::List(chan.acquire()),
        };

        Receiver { flavor }
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Receiver { .. }")
    }
}
