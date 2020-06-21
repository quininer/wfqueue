# 0-unsafe Wait-free Queue

FAA-based wait-free bounded queue, and 0-unsafe.

## Usage

```rust
use std::num::NonZeroUsize;

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

let queue = BoxQueue::new(3);

queue.push(Box::new(0x42)).unwrap();
let output = queue.pop().unwrap();

assert_eq!(*output, 0x42);
```

## reference

* https://github.com/Taymindis/wfqueue
