mod relaxed_ordering {
    use core::sync::atomic::AtomicI32;
    use core::sync::atomic::Ordering::Relaxed;
    static X: AtomicI32 = AtomicI32::new(0);

    pub fn main() {
        fn a() {
            X.fetch_add(5, Relaxed);
            X.fetch_add(10, Relaxed);
        }

        fn b() {
            let a = X.load(Relaxed);
            let b = X.load(Relaxed);
            let c = X.load(Relaxed);
            let d = X.load(Relaxed);
            println!("{a}, {b}, {c}, {d}");
        }

        let a = std::thread::spawn(a);
        let b = std::thread::spawn(b);
        a.join().unwrap();
        b.join().unwrap();
    }
}

mod out_of_thin_air {
    use std::{
        sync::atomic::{AtomicI32, Ordering::Relaxed},
        thread,
    };

    static X: AtomicI32 = AtomicI32::new(0);
    static Y: AtomicI32 = AtomicI32::new(0);

    pub fn main() {
        let a = thread::spawn(|| {
            let x = X.load(Relaxed);
            Y.store(x, Relaxed);
        });
        let b = thread::spawn(|| {
            let y = Y.load(Relaxed);
            X.store(y, Relaxed);
        });

        a.join().unwrap();
        b.join().unwrap();

        assert_eq!(X.load(Relaxed), 0);
        assert_eq!(Y.load(Relaxed), 0);
    }
}

mod release_and_acquire_ordering {
    use std::sync::atomic::Ordering::Relaxed;
    use std::thread;
    use std::{
        sync::atomic::{
            AtomicBool, AtomicU64,
            Ordering::{Acquire, Release},
        },
        time::Duration,
    };

    static DATA: AtomicU64 = AtomicU64::new(0);
    static READY: AtomicBool = AtomicBool::new(false);

    pub fn main() {
        thread::spawn(|| {
            DATA.store(123, Relaxed);
            READY.store(true, Release);
        });

        while !READY.load(Acquire) {
            thread::sleep(Duration::from_millis(100));
            println!("Waiting...");
        }
        println!("{}", DATA.load(Relaxed));
    }
}

mod unsafe_ordering {
    use std::{sync::atomic::AtomicBool, time::Duration};

    static mut DATA: u64 = 0;
    static READY: AtomicBool = AtomicBool::new(false);

    pub(crate) fn main() {
        std::thread::spawn(|| {
            unsafe { DATA = 123 };
            READY.store(true, std::sync::atomic::Ordering::Release);
        });

        while !READY.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(100));
            println!("Waiting...");
        }
        println!("{}", unsafe { DATA });
    }
}

mod proof_a_concept_about_same_thread_order {
    // The thing is not working. It always give me this in the right order. No matter the relaxed thing.
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering::Relaxed;
    use std::thread;
    use std::{sync::atomic::AtomicBool, time::Duration};

    static V1: AtomicBool = AtomicBool::new(false);
    static V2: AtomicBool = AtomicBool::new(false);
    static V3: AtomicBool = AtomicBool::new(false);
    static DONE: AtomicBool = AtomicBool::new(false);

    pub fn main() {
        thread::spawn(|| {
            // Want to know if this can happen in different order.
            V1.store(true, Relaxed);
            V3.store(true, Relaxed);
            V2.store(true, Relaxed);
            DONE.store(true, Relaxed);
        });

        // while !READY.load(Relaxed) {
        // thread::sleep(Duration::from_millis(100));
        // println!("Waiting...");
        // }

        while !DONE.load(Relaxed) {
            continue;
        }
        println!(
            "v1: {}, v3: {}, v2: {}",
            V1.load(Relaxed),
            V3.load(Relaxed),
            V2.load(Relaxed)
        );
    }
}

mod pattern_used_on_mutexes {
    use std::{
        sync::atomic::{
            AtomicBool,
            Ordering::{Acquire, Relaxed, Release},
        },
        thread,
    };

    static mut DATA: String = String::new();
    static LOCKED: AtomicBool = AtomicBool::new(false);

    fn f() {
        if LOCKED
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_ok()
        {
            unsafe { DATA.push('!') };
            LOCKED.store(false, Release);
        }
    }

    pub fn main() {
        thread::scope(|s| {
            for _ in 0..100 {
                s.spawn(f);
            }
        })
    }
}

mod lazy_initialization_with_indirection {
    // we will use atomicpointer to do this

    use std::sync::atomic::AtomicPtr;
    use std::sync::atomic::Ordering;
    use std::thread;

    struct Data;
    fn generate_data() -> Data {
        Data {}
    }
    fn get_data() -> &'static Data {
        static PTR: AtomicPtr<Data> = AtomicPtr::new(std::ptr::null_mut());

        let mut p = PTR.load(Ordering::Acquire);

        if p.is_null() {
            p = Box::into_raw(Box::new(generate_data()));
            if let Err(e) = PTR.compare_exchange(
                std::ptr::null_mut(),
                p,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                drop(unsafe { Box::from_raw(p) });
                p = e;
            }
        }

        unsafe { &*p }
    }
}

pub fn main() {
    println!("Here from cap 3!");
    // relaxed_ordering::main();
    // out_of_thin_air::main();
    // release_and_acquire_ordering::main();
    // unsafe_ordering::main();
    // proof_a_concept_about_same_thread_order::main();
    // pattern_used_on_mutexes::main();
}
