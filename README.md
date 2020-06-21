# 0-unsafe Wait-free Queue

FAA-based wait-free bounded queue, and 0-unsafe.

## Usage

```rust
use wfqueue::WfQueue;

let queue = WfQueue::new(3);

queue.push(Box::new(0x42)).unwrap();
let output = queue.pop().unwrap();

assert_eq!(*output, 0x42);
```

## reference

* https://github.com/Taymindis/wfqueue
