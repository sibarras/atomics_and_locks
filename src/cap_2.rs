mod stop_flag {
    use std::{
        sync::atomic::{AtomicBool, Ordering::Relaxed},
        thread,
    };
    pub(super) fn main() {
        static STOP: AtomicBool = AtomicBool::new(false);

        let background_thread = thread::spawn(|| {
            while !STOP.load(Relaxed) {
                some_work();
            }
        });

        for line in std::io::stdin().lines() {
            match line.unwrap().as_str() {
                "help" => println!("Commands: help, stop"),
                "stop" => break,
                cmd => println!("unknown command {cmd}"),
            }
        }
        STOP.store(true, Relaxed);
        background_thread.join().unwrap();
    }
    fn some_work() {
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

mod progress_reporting {
    // process 100 items by another thread while main thread is giving the progress..
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::Relaxed;
    use std::thread;

    fn process_item(_i: usize) {
        thread::sleep(std::time::Duration::from_millis(30));
    }

    pub fn main() {
        let num_done = AtomicUsize::new(0);

        thread::scope(|s| {
            s.spawn(|| {
                for i in 0..100 {
                    process_item(i);
                    num_done.store(i + 1, Relaxed);
                }
            });

            loop {
                let n = num_done.load(Relaxed);
                if n == 100 {
                    break;
                };
                println!("Working.. {n}/100 done");
                thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        println!("done!");
    }

    pub fn with_sync() {
        let num_done: AtomicUsize = 0.into();
        let main_thread = thread::current();

        thread::scope(|s| {
            s.spawn(|| {
                for i in 0..100 {
                    process_item(0);
                    num_done.store(i + 1, Relaxed);
                    main_thread.unpark();
                }
            });

            loop {
                let n = num_done.load(Relaxed);
                if n == 100 {
                    break;
                };
                println!("Working.. {n:02}/100 done");
                thread::park_timeout(std::time::Duration::from_secs(1));
            }
        });

        println!("done!");
    }
}

mod lazy_initialization {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

    fn calculate_x() -> u64 {
        9
    }
    pub fn get_x() -> u64 {
        static X: AtomicU64 = AtomicU64::new(0);
        let mut x = X.load(Relaxed);
        if x == 0 {
            x = calculate_x();
            X.store(x, Relaxed);
        }
        x
    }
}
mod multiple_threads_reporting {
    use std::{
        sync::atomic::{AtomicUsize, Ordering::Relaxed},
        thread,
        time::Duration,
    };

    fn process_item(_v: i32) {
        thread::sleep(Duration::from_millis(100));
    }
    pub fn main() {
        let num_done = &AtomicUsize::new(0);
        thread::scope(|s| {
            for t in 0..4 {
                s.spawn(move || {
                    for i in 0..25 {
                        process_item(t * 25 + i);
                        num_done.fetch_add(1, Relaxed);
                    }
                });
            }
            loop {
                let n = num_done.load(Relaxed);
                if n == 100 {
                    break;
                }
                println!("Working.. {n:02}/100 done");
                thread::sleep(Duration::from_secs(1));
            }
        })
    }
}

mod statistics {
    use std::{
        sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed},
        thread,
        time::{Duration, Instant},
    };

    fn process_item(_i: i32) {
        thread::sleep(Duration::from_millis(50));
    }
    pub fn main() {
        let num_done = &AtomicUsize::new(0);
        let total_time = &AtomicU64::new(0);
        let max_time = &AtomicU64::new(0);

        thread::scope(|s| {
            for t in 0..4 {
                s.spawn(move || {
                    for i in 0..25 {
                        let start = Instant::now();
                        process_item(t * 25 + i);
                        let time_taken = start.elapsed().as_micros() as u64;
                        num_done.fetch_add(1, Relaxed);
                        total_time.fetch_add(time_taken, Relaxed);
                        max_time.fetch_max(time_taken, Relaxed);
                    }
                });
            }

            loop {
                let total_time = Duration::from_micros(total_time.load(Relaxed));
                let max_time = Duration::from_micros(max_time.load(Relaxed));
                let n = num_done.load(Relaxed);
                if n == 100 {
                    break;
                }
                if n == 0 {
                    println!("Working.. nothing done yet.");
                } else {
                    println!(
                        "Working.. {n:02}/100 done, {:?} average, {:?} peak",
                        total_time / n as u32,
                        max_time
                    );
                }
                thread::sleep(Duration::from_millis(100));
            }
        })
    }
}

mod id_allocation {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

    pub fn allocate_new_id() -> u32 {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        let mut id = NEXT_ID.load(Relaxed);
        loop {
            assert!(id < 1000, "too many IDS!");
            match NEXT_ID.compare_exchange_weak(id, id + 1, Relaxed, Relaxed) {
                Ok(_) => return id,
                Err(v) => id = v,
            }
        }
    }
}
mod get_random_key {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    fn generate_random_key() -> u64 {
        3
    }
    pub fn get_key() -> u64 {
        static KEY: AtomicU64 = AtomicU64::new(0);
        let key = KEY.load(Relaxed);
        if key == 0 {
            let new_key = generate_random_key();
            match KEY.compare_exchange(0, new_key, Relaxed, Relaxed) {
                Ok(_) => new_key,
                Err(k) => k,
            }
        } else {
            key
        }
    }
}
pub fn main() {
    stop_flag::main();
    progress_reporting::main();
    progress_reporting::with_sync();
    multiple_threads_reporting::main();
    statistics::main();
    lazy_initialization::get_x();
    id_allocation::allocate_new_id();
    get_random_key::get_key();
}
