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

fn box_into_nonzero(input: Box<usize>) -> NonZeroUsize {
    let input = Box::into_raw(input) as usize;

    unsafe {
        NonZeroUsize::new_unchecked(input)
    }
}

fn box_from_nonzero(output: NonZeroUsize) -> Box<usize> {
    let output = output.get() as *mut usize;

    unsafe {
        Box::from_raw(output)
    }
}

wfqueue::codegen! {
    pub struct BoxQueue(Box<usize>);

    fn into_nonzero = box_into_nonzero;
    fn from_nonzero = box_from_nonzero;
}

#[test]
fn test_codegen_simple() {
    loom::model(|| {
        let queue = BoxQueue::new(3);

        queue.push(Box::new(0x42)).unwrap();
        let output = queue.pop().unwrap();

        assert_eq!(*output, 0x42);
    });
}

#[test]
fn test_codegen_thread() {
    loom::model(|| {
        let queue = Arc::new(BoxQueue::new(3));
        let queue2 = queue.clone();

        let h = thread::spawn(move || {
            for i in 0..5 {
                let mut val = Box::new(i);

                loop {
                    match queue2.push(val) {
                        Ok(()) => break,
                        Err(val2) => {
                            val = val2;
                            loom::sync::atomic::spin_loop_hint();
                        }
                    }
                }
            }
        });

        for i in 0..5 {
            loop {
                match queue.pop() {
                    Some(val) => {
                        assert_eq!(*val, i);
                        break
                    },
                    None => loom::sync::atomic::spin_loop_hint()
                }
            }
        }

        h.join().unwrap();
    });
}

#[test]
fn test_codegen_drop() {
    loom::model(|| {
        let queue = BoxQueue::new(3);

        queue.push(Box::new(0x42)).unwrap();
        queue.push(Box::new(0x43)).unwrap();

        let h = thread::spawn(move || {
            let _queue = queue;
        });

        h.join().unwrap();
    });
}
