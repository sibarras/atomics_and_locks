use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

pub fn use_condvar() {
    let queue = Mutex::new(VecDeque::new());
    let finish = Mutex::new(false);
    let not_empty = Condvar::new();
    thread::scope(|s| {
        s.spawn(|| 'a: loop {
            let mut q = queue.lock().unwrap();
            let item = loop {
                if let Some(item) = q.pop_front() {
                    break item;
                } else {
                    q = not_empty.wait(q).unwrap();
                    if *finish.lock().unwrap() {
                        break 'a;
                    }
                }
            };
            drop(q);
            dbg!(item);
        });

        let start = SystemTime::now();
        for i in 0.. {
            queue.lock().unwrap().push_back(i);
            if SystemTime::now() - Duration::from_secs(5) > start {
                *finish.lock().unwrap() = true;
                not_empty.notify_one();
                break;
            }
            not_empty.notify_one();
            thread::sleep(Duration::from_secs(1));
        }
    });
}
