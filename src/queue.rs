//! wfqueue implemention

use std::cell::Cell;
use std::num::NonZeroUsize;
use cache_padded::CachePadded;
use crate::loom::sync::atomic::{ AtomicUsize, Ordering };


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

struct Index(Cell<usize>);

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

        head.saturating_sub(tail)
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.nptr.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == self.nptr.len()
    }

    /// Each queue should use a fixed enqueue context in each thread.
    /// If the wrong context is used, it may lead to logic confusion.
    pub fn try_enqueue(&self, ctx: &EnqueueCtx, val: NonZeroUsize) -> bool {
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

    /// Each queue should use a fixed enqueue context in each thread.
    /// If the wrong context is used, it may lead to logic confusion.
    pub fn try_dequeue(&self, ctx: &DequeueCtx) -> Option<NonZeroUsize> {
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
        EnqueueCtx { index: Index(Cell::new(0)) }
    }
}

impl DequeueCtx {
    pub const fn new() -> DequeueCtx {
        DequeueCtx { index: Index(Cell::new(0)) }
    }
}

impl Index {
    #[inline]
    pub fn load(&self) -> Option<usize> {
        self.0.get().checked_sub(1)
    }

    #[inline]
    pub fn clean(&self) {
        self.0.set(0);
    }

    #[inline]
    pub fn store(&self, val: usize) {
        self.0.set(val + 1);
    }
}
