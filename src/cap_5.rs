// building our own Channels

use std::thread;

mod mutex_based_channel {
    use std::{
        collections::VecDeque,
        sync::{Condvar, Mutex},
    };

    pub struct Channel<T> {
        queue: Mutex<VecDeque<T>>,
        item_ready: Condvar,
    }

    impl<T> Channel<T> {
        pub fn new() -> Self {
            Self {
                queue: Mutex::new(VecDeque::new()),
                item_ready: Condvar::new(),
            }
        }

        fn send(&self, message: T) {
            self.queue.lock().unwrap().push_back(message);
            self.item_ready.notify_one();
        }

        fn receive(&self) -> T {
            ///! My comment
            let mut b = self.queue.lock().unwrap();
            loop {
                if let Some(message) = b.pop_front() {
                    return message;
                }
                b = self.item_ready.wait(b).unwrap();
            }
        }
    }
}

mod unsafe_one_shot_channel {
    //! This is a channel who only sends one message from one thread to another.

    use std::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        sync::atomic::{AtomicBool, Ordering},
    };

    pub struct Channel<T> {
        message: UnsafeCell<MaybeUninit<T>>,
        ready: AtomicBool,
    }

    unsafe impl<T> Sync for Channel<T> where T: Send {}
    impl<T> Channel<T> {
        pub const fn new() -> Self {
            Self {
                message: UnsafeCell::new(MaybeUninit::uninit()),
                ready: AtomicBool::new(false),
            }
        }

        pub unsafe fn send(&self, message: T) {
            //! Safety: Only call this once!
            (*self.message.get()).write(message);
            self.ready.store(true, Ordering::Release);
        }

        pub fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Acquire)
        }

        /// Safety: Only call this once,
        /// and only after is_ready() returns true!
        pub unsafe fn receive(&self) -> T {
            (*self.message.get()).assume_init_read()
        }
    }
}
mod safety_through_runtime_checks {
    //! This is a channel who only sends one message from one thread to another.

    use std::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        sync::atomic::{AtomicBool, Ordering},
    };

    pub struct Channel<T> {
        message: UnsafeCell<MaybeUninit<T>>,
        ready: AtomicBool,
        in_use: AtomicBool,
    }

    unsafe impl<T> Sync for Channel<T> where T: Send {}
    impl<T> Channel<T> {
        pub const fn new() -> Self {
            Self {
                message: UnsafeCell::new(MaybeUninit::uninit()),
                ready: AtomicBool::new(false),
                in_use: AtomicBool::new(false),
            }
        }

        /// Panics when trying to send more than one mesage
        pub fn send(&self, message: T) {
            if self.in_use.swap(true, Ordering::Relaxed) {
                panic!("can't send more than one message!")
            }
            unsafe { (*self.message.get()).write(message) };
            self.ready.store(true, Ordering::Release);
        }

        pub fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Relaxed)
        }

        /// Panics if no message is available yet.
        /// or if the message is already consumed.
        ///
        /// Tip: Use `is_ready` to check first.
        pub fn receive(&self) -> T {
            if !self.ready.swap(false, Ordering::Acquire) {
                panic!("No message available!");
            }
            unsafe { (*self.message.get()).assume_init_read() }
        }
    }

    impl<T> Drop for Channel<T> {
        fn drop(&mut self) {
            if *self.ready.get_mut() {
                unsafe { self.message.get_mut().assume_init_drop() }
            }
        }
    }
}
pub fn main() {
    use safety_through_runtime_checks::Channel;
    let channel = Channel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("Hello World!");
            // channel.send("Hello World!"); // This will make the program panic!!
            t.unpark();
        });
        while !channel.is_ready() {
            thread::park();
        }

        assert_eq!(channel.receive(), "Hello World!");
    })
}
