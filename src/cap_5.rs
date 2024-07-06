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

    pub fn main() {
        use std::thread;
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
}
mod single_atomic_for_channel_state {
    //! This is a channel who only sends one message from one thread to another.
    const EMPTY: u8 = 0;
    const WRITING: u8 = 1;
    const READY: u8 = 2;
    const READING: u8 = 3;

    use std::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        sync::atomic::{AtomicU8, Ordering},
    };

    pub struct Channel<T> {
        message: UnsafeCell<MaybeUninit<T>>,
        state: AtomicU8,
    }

    unsafe impl<T> Sync for Channel<T> where T: Send {}
    impl<T> Channel<T> {
        pub const fn new() -> Self {
            Self {
                message: UnsafeCell::new(MaybeUninit::uninit()),
                state: AtomicU8::new(EMPTY),
            }
        }

        /// Panics when trying to send more than one mesage
        pub fn send(&self, message: T) {
            if self
                .state
                .compare_exchange(EMPTY, WRITING, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
            {
                panic!("can't send more than one message!")
            }
            unsafe { (*self.message.get()).write(message) };
            self.state.store(READY, Ordering::Release);
        }

        pub fn is_ready(&self) -> bool {
            self.state.load(Ordering::Relaxed) == READY
        }

        /// Panics if no message is available yet.
        /// or if the message is already consumed.
        ///
        /// Tip: Use `is_ready` to check first.
        pub fn receive(&self) -> T {
            if self
                .state
                .compare_exchange(READY, READING, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                panic!("No message available!");
            }
            unsafe { (*self.message.get()).assume_init_read() }
        }
    }

    impl<T> Drop for Channel<T> {
        fn drop(&mut self) {
            if *self.state.get_mut() == READY {
                unsafe { self.message.get_mut().assume_init_drop() }
            }
        }
    }
}

mod safety_through_types {
    use std::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let a = Arc::new(Channel {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        });

        (Sender { channel: a.clone() }, Receiver { channel: a })
    }

    pub struct Sender<T> {
        channel: Arc<Channel<T>>,
    }
    pub struct Receiver<T> {
        channel: Arc<Channel<T>>,
    }
    struct Channel<T> {
        message: UnsafeCell<MaybeUninit<T>>,
        ready: AtomicBool,
    }

    unsafe impl<T> Sync for Channel<T> where T: Send {}

    impl<T> Sender<T> {
        pub fn send(self, message: T) {
            unsafe { (*self.channel.message.get()).write(message) };
            self.channel.ready.store(true, Ordering::Release)
        }
    }
    impl<T> Receiver<T> {
        pub fn is_ready(&self) -> bool {
            self.channel.ready.load(Ordering::Relaxed)
        }
        pub fn receive(self) -> T {
            if !self.channel.ready.swap(false, Ordering::Acquire) {
                panic!("No Message Available!")
            }
            unsafe { (*self.channel.message.get()).assume_init_read() }
        }
    }

    impl<T> Drop for Channel<T> {
        fn drop(&mut self) {
            if *self.ready.get_mut() {
                unsafe { self.message.get_mut().assume_init_drop() }
            }
        }
    }

    pub fn main() {
        use std::thread;

        thread::scope(|s| {
            let (sender, receiver) = channel();
            let t = thread::current();
            s.spawn(move || {
                sender.send("hello world!");
                t.unpark();
            });
            while !receiver.is_ready() {
                thread::park();
            }
            assert_eq!(receiver.receive(), "hello world!");
        })
    }
}
mod borrowing_to_avoid_allocations {
    use std::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        sync::atomic::{AtomicBool, Ordering},
    };

    pub struct Sender<'a, T> {
        channel: &'a Channel<T>,
    }
    pub struct Receiver<'a, T> {
        channel: &'a Channel<T>,
    }
    struct Channel<T> {
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

        pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
            *self = Self::new();
            (Sender { channel: self }, Receiver { channel: self })
        }
    }

    impl<T> Sender<'_, T> {
        pub fn send(self, message: T) {
            unsafe { (*self.channel.message.get()).write(message) };
            self.channel.ready.store(true, Ordering::Release)
        }
    }
    impl<T> Receiver<'_, T> {
        pub fn is_ready(&self) -> bool {
            self.channel.ready.load(Ordering::Relaxed)
        }
        pub fn receive(self) -> T {
            if !self.channel.ready.swap(false, Ordering::Acquire) {
                panic!("No Message Available!")
            }
            unsafe { (*self.channel.message.get()).assume_init_read() }
        }
    }

    impl<T> Drop for Channel<T> {
        fn drop(&mut self) {
            if *self.ready.get_mut() {
                unsafe { self.message.get_mut().assume_init_drop() }
            }
        }
    }

    pub fn main() {
        use std::thread;

        let mut channel = Channel::new();
        thread::scope(|s| {
            let (sender, receiver) = channel.split();

            let t = thread::current();
            s.spawn(move || {
                sender.send("hello world!");
                t.unpark();
            });
            while !receiver.is_ready() {
                thread::park();
            }
            assert_eq!(receiver.receive(), "hello world!");
        })
    }
}

mod blocking {
    use std::{
        cell::UnsafeCell,
        marker::PhantomData,
        mem::MaybeUninit,
        sync::atomic::{AtomicBool, Ordering},
        thread,
    };

    pub struct Sender<'a, T> {
        channel: &'a Channel<T>,
        receiving_thread: thread::Thread,
    }
    pub struct Receiver<'a, T> {
        channel: &'a Channel<T>,
        _no_data: PhantomData<*const ()>,
    }
    struct Channel<T> {
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

        pub fn split(&mut self) -> (Sender<T>, Receiver<T>) {
            *self = Self::new();
            (
                Sender {
                    channel: self,
                    receiving_thread: thread::current(),
                },
                Receiver {
                    channel: self,
                    _no_data: PhantomData,
                },
            )
        }
    }

    impl<T> Sender<'_, T> {
        pub fn send(self, message: T) {
            unsafe { (*self.channel.message.get()).write(message) };
            self.channel.ready.store(true, Ordering::Release);
            self.receiving_thread.unpark();
        }
    }
    impl<T> Receiver<'_, T> {
        pub fn receive(self) -> T {
            while !self.channel.ready.swap(false, Ordering::Acquire) {
                thread::park();
            }
            unsafe { (*self.channel.message.get()).assume_init_read() }
        }
    }

    impl<T> Drop for Channel<T> {
        fn drop(&mut self) {
            if *self.ready.get_mut() {
                unsafe { self.message.get_mut().assume_init_drop() }
            }
        }
    }

    pub fn main() {
        use std::thread;

        let mut channel = Channel::new();
        thread::scope(|s| {
            let (sender, receiver) = channel.split();
            s.spawn(move || {
                sender.send("hello world!");
            });
            assert_eq!(receiver.receive(), "hello world!");
        })
    }
}

pub fn main() {
    // use safety_through_types::main as m;
    // use borrowing_to_avoid_allocations::main as m;
    use blocking::main as m;

    m();
}
