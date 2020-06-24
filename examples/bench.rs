use std::{ env, thread };
use std::sync::{ atomic, Arc };
use std::num::NonZeroUsize;
use wfqueue::queue::{ WfQueue, EnqueueCtx, DequeueCtx };

const MAX_COUNT: usize = 128 * 1024 * 1024;

static ENQ_COUNT: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
static DEQ_COUNT: atomic::AtomicUsize = atomic::AtomicUsize::new(0);


fn main() {
    let threadsum = env::args().nth(1).unwrap();
    let threadsum = threadsum.parse::<usize>().unwrap();

    let queue = Arc::new(WfQueue::new(64));
    let val = NonZeroUsize::new(0x42).unwrap();
    let mut handles = Vec::new();

    for _ in 0..threadsum {
        let queue = queue.clone();
        let queue2 = queue.clone();

        let h = thread::spawn(move || {
            let mut enq = EnqueueCtx::new();

            while ENQ_COUNT.fetch_add(1, atomic::Ordering::SeqCst) < MAX_COUNT {
                while !queue.try_enqueue(&mut enq, val) {}
            }
        });

        let h2 = thread::spawn(move || {
            let mut deq = DequeueCtx::new();

            while DEQ_COUNT.fetch_add(1, atomic::Ordering::SeqCst) < MAX_COUNT {
                while queue2.try_dequeue(&mut deq).is_none() {}
            }
        });

        handles.push(h);
        handles.push(h2);
    }

    for h in handles {
        h.join().unwrap();
    }
}
