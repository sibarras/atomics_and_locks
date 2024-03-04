use std::{
    collections::VecDeque,
    ops::BitAndAssign,
    sync::RwLock,
    thread,
    time::{Duration, SystemTime},
};

pub fn example() {
    let queue = RwLock::new(VecDeque::new());
    let timeout = RwLock::new(false);
    thread::scope(|s| {
        let t2 = s.spawn(|| loop {
            if *timeout.read().unwrap() {
                break;
            }
            let v = queue.write().unwrap().pop_front();
            if let Some(v) = v {
                println!("Consuming {v}");
            } else {
                thread::park();
            }
        });
        let start = SystemTime::now();
        let loop_duration = Duration::from_secs(6);
        loop {
            if (SystemTime::now() - loop_duration) > start {
                *timeout.write().unwrap() = true;
                t2.thread().unpark();
                break;
            }
            queue.write().unwrap().push_back(4);
            t2.thread().unpark();
            thread::sleep(Duration::from_secs(1));
        }
    })
}
