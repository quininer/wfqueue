use std::{ env, thread };
use std::sync::{ atomic, Arc };
use std::num::NonZeroUsize;
use crossbeam_queue::ArrayQueue;

const COUNT: isize = 128 * 1024 * 1024;

static ENQ_COUNT: atomic::AtomicIsize = atomic::AtomicIsize::new(COUNT);
static DEQ_COUNT: atomic::AtomicIsize = atomic::AtomicIsize::new(COUNT);


fn main() {
    let threadsum = env::args().nth(1).unwrap();
    let threadsum = threadsum.parse::<usize>().unwrap();

    let queue = Arc::new(ArrayQueue::new(64));
    let val = NonZeroUsize::new(0x42).unwrap();
    let mut handles = Vec::new();

    for _ in 0..threadsum {
        let queue = queue.clone();
        let queue2 = queue.clone();

        let h = thread::spawn(move || {
            while ENQ_COUNT.fetch_sub(1, atomic::Ordering::SeqCst) > 1 {
                while queue.push(val).is_err() {}
            }
        });

        let h2 = thread::spawn(move || {
            while DEQ_COUNT.fetch_sub(1, atomic::Ordering::SeqCst) > 1 {
                while queue2.pop().is_err() {}
            }
        });

        handles.push(h);
        handles.push(h2);
    }

    for h in handles {
        h.join().unwrap();
    }
}
