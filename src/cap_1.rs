use crate::condition_variables;
use crate::parking;
use std::{sync::Arc, thread};

fn f() {
    println!("Hi from thread {:?}", thread::current().id());
}

fn run_without_knowing_if_completed() {
    thread::spawn(f);
    thread::spawn(f);

    println!("Running but I dont know if finished..");
}

fn run_checking_if_completed() {
    let t1 = thread::spawn(f);
    let t2 = thread::spawn(f);
    println!("Running but I will check that both finishes.");
    t1.join().unwrap();
    t2.join().unwrap();
}

fn better_join() {
    let t1 = thread::spawn(f);
    let t2 = thread::spawn(f);

    println!("Joining but not blocking in case one is not finished.");

    while !(t1.is_finished() && t2.is_finished()) {
        continue;
    }
}

const fn calc_sum(v: &[usize]) -> usize {
    let mut i = 0;
    let mut total = 0;

    while i < v.len() {
        total += v[i];
        i += 1;
    }

    total
}

const fn calc_max(v: &[usize]) -> usize {
    let mut i = 0;
    let mut max = 0;

    while i < v.len() {
        let vi = v[i];
        max = if max < vi { vi } else { max };
        i += 1;
    }

    max
}

fn double_calculation() {
    let values = vec![1, 2, 3, 4, 5];
    let calcs = thread::scope(|s| {
        let total = s.spawn(|| calc_sum(&values));
        let maximum = s.spawn(|| calc_max(&values));
        (total.join().unwrap(), maximum.join().unwrap())
    });

    println!("calcs: {:?}", calcs);
}

fn double_arc_calculation() {
    let values = Arc::new([1, 2, 3, 4, 5]);
    let v2 = values.clone();
    let total = thread::spawn(move || values.into_iter().sum::<usize>());
    let maximum = thread::spawn(move || v2.into_iter().max().unwrap());
    println!(
        "calcs with arc: {:?}",
        (total.join().unwrap(), maximum.join().unwrap())
    );
}

fn cell_mutability() {
    use std::cell::Cell;

    fn f(a: &Cell<i32>, b: &Cell<i32>) {
        let before = a.get();
        b.set(b.get() + 1);
        let after = a.get(); // This can be different than in the begining.

        if before != after {
            // this can happen...
            println!("those are different!");
        }
    }

    let a = Cell::new(2);
    f(&a, &a);
}
fn main() {
    run_without_knowing_if_completed();
    run_checking_if_completed();
    better_join();
    double_calculation();
    double_arc_calculation();
    cell_mutability();
    parking::example();
}
