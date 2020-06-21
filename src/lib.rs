#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::sync;
}

use std::num::NonZeroUsize;
use loom::sync::atomic::{ AtomicUsize, Ordering };
use cache_padded::CachePadded;


#[cfg(not(feature = "loom"))]
const MAX_TRY: usize = 128;

#[cfg(feature = "loom")]
const MAX_TRY: usize = 1;

pub struct WfQueue {
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
    nptr: Box<[CachePadded<AtomicUsize>]>
}

pub struct EnqueueCtx {
    index: Index
}

pub struct DequeueCtx {
    index: Index
}

struct Index(usize);

impl WfQueue {
    pub fn new(cap: usize) -> WfQueue {
        let mut nptr = Vec::with_capacity(cap);

        for _ in 0..cap {
            nptr.push(CachePadded::new(AtomicUsize::new(0)));
        }

        let nptr = nptr.into_boxed_slice();

        WfQueue {
            head: CachePadded::new(AtomicUsize::new(0)),
            tail: CachePadded::new(AtomicUsize::new(0)),
            nptr
        }
    }

    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        head.checked_sub(tail).unwrap_or(0)
    }

    pub fn try_enqueue(&self, ctx: &mut EnqueueCtx, val: NonZeroUsize) -> bool {
        macro_rules! enqueue {
            ( $ptr:expr, $val:expr ; $ok:expr ; $fail:expr ) => {
                let mut curr = $ptr.load(Ordering::Acquire);

                for _ in 0..MAX_TRY {
                    if curr == 0 {
                        if $ptr.compare_exchange_weak(curr, $val.get(), Ordering::Release, Ordering::Relaxed).is_ok() {
                            $ok;
                            return true;
                        }
                    } else {
                        curr = $ptr.load(Ordering::Acquire);
                    }
                }

                $fail;
                return false;
            }
        }

        if let Some(index) = ctx.index.load() {
            let nptr = &self.nptr[index];

            enqueue!{
                nptr, val;
                {
                    ctx.index.clean();
                };
                {}
            }
        }

        let head = self.head.fetch_add(1, Ordering::Relaxed) % self.nptr.len();
        let nptr = &self.nptr[head];

        enqueue!{
            nptr, val;
            {};
            {
                ctx.index.store(head);
            }
        }
    }

    pub fn try_dequeue(&self, ctx: &mut DequeueCtx) -> Option<NonZeroUsize> {
        macro_rules! dequeue {
            ( $ptr:expr ; $ok:expr ; $fail:expr ) => {
                let mut val = $ptr.load(Ordering::Acquire);

                for _ in 0..MAX_TRY {
                    match NonZeroUsize::new(val) {
                        Some(nzval) => if $ptr.compare_exchange_weak(val, 0, Ordering::Release, Ordering::Relaxed).is_ok() {
                            $ok;
                            return Some(nzval);
                        },
                        None => {
                            val = $ptr.load(Ordering::Acquire);
                        }
                    }
                }

                $fail;
                return None;
            }
        }

        if let Some(index) = ctx.index.load() {
            let nptr = &self.nptr[index];

            dequeue!{
                nptr;
                {
                    ctx.index.clean();
                };
                {}
            }
        }

        let tail = self.tail.fetch_add(1, Ordering::Relaxed) % self.nptr.len();
        let nptr = &self.nptr[tail];

        dequeue!{
            nptr;
            {};
            {
                ctx.index.store(tail);
            }
        }
    }
}

impl EnqueueCtx {
    pub const fn new() -> EnqueueCtx {
        EnqueueCtx { index: Index(0) }
    }
}

impl DequeueCtx {
    pub const fn new() -> DequeueCtx {
        DequeueCtx { index: Index(0) }
    }
}

impl Index {
    #[inline]
    pub fn load(&self) -> Option<usize> {
        self.0.checked_sub(1)
    }

    #[inline]
    pub fn clean(&mut self) {
        self.0 = 0;
    }

    #[inline]
    pub fn store(&mut self, val: usize) {
        self.0 = val + 1;
    }
}
