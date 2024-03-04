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
pub fn main() {
    stop_flag::main();
}
