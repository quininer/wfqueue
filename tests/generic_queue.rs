#![cfg(not(feature = "loom"))]

use std::thread;
use std::sync::Arc;
use wfqueue::WfQueue;


#[test]
fn test_codegen_simple() {
    let queue = WfQueue::new(3);

    queue.push(Box::new(0x42)).unwrap();
    let output = queue.pop().unwrap();

    assert_eq!(*output, 0x42);
}

#[test]
fn test_codegen_thread() {
    let queue = Arc::new(WfQueue::new(3));
    let queue2 = queue.clone();

    let h = thread::spawn(move || {
        for i in 0..5 {
            let mut val = Box::new(i);

            loop {
                match queue2.push(val) {
                    Ok(()) => break,
                    Err(val2) => {
                        val = val2;
                        std::sync::atomic::spin_loop_hint();
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
                None => std::sync::atomic::spin_loop_hint()
            }
        }
    }

    h.join().unwrap();
}

#[test]
fn test_codegen_drop() {
    let queue = WfQueue::new(3);

    queue.push(Box::new(0x42)).unwrap();
    queue.push(Box::new(0x43)).unwrap();

    let h = thread::spawn(move || {
        let _queue = queue;
    });

    h.join().unwrap();
}
