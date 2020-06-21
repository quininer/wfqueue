#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::thread;
    pub use std::sync;

    pub fn model<F>(f: F)
    where
        F: Fn() + Sync + Send + 'static
    {
        f()
    }
}


use std::num::NonZeroUsize;
use loom::sync::Arc;
use loom::thread;
use wfqueue::{ WfQueue, EnqueueCtx, DequeueCtx };


#[test]
fn one_thread() {
    loom::model(|| {
        let queue = WfQueue::new(3);
        let mut ecx = EnqueueCtx::new();
        let mut dcx = DequeueCtx::new();

        let val = NonZeroUsize::new(0x42).unwrap();
        assert!(queue.try_enqueue(&mut ecx, val));
        let val2 = queue.try_dequeue(&mut dcx).unwrap();
        assert_eq!(val, val2);

        assert!(queue.try_dequeue(&mut dcx).is_none());

        let valx = NonZeroUsize::new(0x42).unwrap();
        let valy = NonZeroUsize::new(0x43).unwrap();
        let valz = NonZeroUsize::new(0x44).unwrap();
        assert!(queue.try_enqueue(&mut ecx, valx));
        assert!(queue.try_enqueue(&mut ecx, valy));
        assert!(queue.try_enqueue(&mut ecx, valz));
        assert!(!queue.try_enqueue(&mut ecx, valz));

        let valx2 = queue.try_dequeue(&mut dcx).unwrap();
        let valy2 = queue.try_dequeue(&mut dcx).unwrap();
        let valz2 = queue.try_dequeue(&mut dcx).unwrap();
        assert_eq!(valx, valx2);
        assert_eq!(valy, valy2);
        assert_eq!(valz, valz2);
    });
}

#[test]
fn two_thread() {
    loom::model(|| {
        let queue = Arc::new(WfQueue::new(3));
        let queue2 = queue.clone();

        let h = thread::spawn(move || {
            let mut ecx = EnqueueCtx::new();

            for i in 0..5 {
                let val = NonZeroUsize::new(0x42 + i).unwrap();

                while !queue2.try_enqueue(&mut ecx, val) {
                    loom::sync::atomic::spin_loop_hint();
                }
            }
        });

        let mut dcx = DequeueCtx::new();

        for i in 0..5 {
            let val = NonZeroUsize::new(0x42 + i).unwrap();

            loop {
                if let Some(val2) = queue.try_dequeue(&mut dcx) {
                    assert_eq!(val, val2);
                    break
                }

                loom::sync::atomic::spin_loop_hint();
            }
        }

        h.join().unwrap();
    });
}

#[test]
fn three_thread_s2r1() {
    loom::model(|| {
        let queue = Arc::new(WfQueue::new(3));
        let queue2 = queue.clone();
        let queue3 = queue.clone();

        let h = thread::spawn(move || {
            let mut ecx = EnqueueCtx::new();

            for i in 0..2 {
                let val = NonZeroUsize::new(0x42 + i).unwrap();

                while !queue2.try_enqueue(&mut ecx, val) {
                    loom::sync::atomic::spin_loop_hint();
                }
            }
        });

        let h2 = thread::spawn(move || {
            let mut ecx = EnqueueCtx::new();

            for i in 2..4 {
                let val = NonZeroUsize::new(0x42 + i).unwrap();

                while !queue3.try_enqueue(&mut ecx, val) {
                    loom::sync::atomic::spin_loop_hint();
                }
            }
        });

        let mut dcx = DequeueCtx::new();
        let mut output = Vec::new();

        for _ in 0..4 {
            loop {
                if let Some(val) = queue.try_dequeue(&mut dcx) {
                    output.push(val.get());
                    break
                }

                loom::sync::atomic::spin_loop_hint();
            }
        }

        h.join().unwrap();
        h2.join().unwrap();

        // check
        output.sort();
        let expected = (0..4).map(|n| 0x42 + n).collect::<Vec<usize>>();
        assert_eq!(expected, output.as_slice());
    });
}

#[test]
fn three_thread_s1r2() {
    loom::model(|| {
        let queue = Arc::new(WfQueue::new(3));
        let queue2 = queue.clone();
        let queue3 = queue.clone();

        let h = thread::spawn(move || {
            let mut ecx = EnqueueCtx::new();

            for i in 0..4 {
                let val = NonZeroUsize::new(0x42 + i).unwrap();

                while !queue2.try_enqueue(&mut ecx, val) {
                    loom::sync::atomic::spin_loop_hint();
                }
            }
        });

        let h2 = thread::spawn(move || {
            let mut dcx = DequeueCtx::new();
            let mut output = Vec::new();

            for _ in 0..2 {
                loop {
                    if let Some(val) = queue3.try_dequeue(&mut dcx) {
                        output.push(val.get());
                        break
                    }

                    loom::sync::atomic::spin_loop_hint();
                }
            }

            output
        });

        let mut dcx = DequeueCtx::new();
        let mut output = Vec::new();

        for _ in 0..2 {
            loop {
                if let Some(val) = queue.try_dequeue(&mut dcx) {
                    output.push(val.get());
                    break
                }

                loom::sync::atomic::spin_loop_hint();
            }
        }

        h.join().unwrap();
        let mut output2 = h2.join().unwrap();

        // check
        output.append(&mut output2);
        output.sort();
        let expected = (0..4).map(|n| 0x42 + n).collect::<Vec<usize>>();
        assert_eq!(expected, output.as_slice());
    });
}
