#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::sync;
}

#[cfg(feature = "loom")]
use loom;

pub mod queue;

use std::num::NonZeroUsize;
use std::marker::PhantomData;
use per_thread_object::ThreadLocal;


pub struct WfQueue<T: Queueable> {
    queue: queue::WfQueue,
    context: ThreadLocal<Context>,
    _phantom: PhantomData<T>
}

pub trait Queueable {
    fn into_nonzero(self) -> NonZeroUsize;

    /// # Safety
    ///
    /// Unsafe conversion from `NonZeroUsize`.
    unsafe fn from_nonzero(n: NonZeroUsize) -> Self;
}

struct Context {
    enq: queue::EnqueueCtx,
    deq: queue::DequeueCtx
}

impl<T: Queueable> WfQueue<T> {
    pub fn new(cap: usize) -> WfQueue<T> {
        WfQueue {
            queue: queue::WfQueue::new(cap),
            context: ThreadLocal::new(),
            _phantom: PhantomData
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.queue.is_full()
    }

    pub fn push(&self, val: T) -> Result<(), T> {
        let ctx = self.context.get_or(Context::new);
        let val = val.into_nonzero();

        if self.queue.try_enqueue(&ctx.enq, val) {
            Ok(())
        } else {
            unsafe {
                Err(T::from_nonzero(val))
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        let ctx = self.context.get_or(Context::new);
        let val = self.queue.try_dequeue(&ctx.deq)?;

        unsafe {
            Some(T::from_nonzero(val))
        }
    }
}

impl<T: Queueable> Drop for WfQueue<T> {
    #[inline]
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

impl Context {
    pub const fn new() -> Context {
        Context {
            enq: queue::EnqueueCtx::new(),
            deq: queue::DequeueCtx::new()
        }
    }
}

// impl Queueable

impl Queueable for NonZeroUsize {
    #[inline]
    fn into_nonzero(self) -> NonZeroUsize {
        self
    }

    #[inline]
    unsafe fn from_nonzero(n: NonZeroUsize) -> Self {
        n
    }
}

impl<T> Queueable for &'static T {
    #[inline]
    fn into_nonzero(self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(self as *const T as usize)
        }
    }

    #[inline]
    unsafe fn from_nonzero(n: NonZeroUsize) -> Self {
        &*(n.get() as *const T)
    }
}

impl<T> Queueable for Box<T> {
    #[inline]
    fn into_nonzero(self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(Box::into_raw(self) as usize)
        }
    }

    #[inline]
    unsafe fn from_nonzero(n: NonZeroUsize) -> Self {
        Box::from_raw(n.get() as *mut _)
    }
}

use loom::sync::Arc;

impl<T> Queueable for Arc<T> {
    #[inline]
    fn into_nonzero(self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(Arc::into_raw(self) as usize)
        }
    }

    #[inline]
    unsafe fn from_nonzero(n: NonZeroUsize) -> Self {
        Arc::from_raw(n.get() as *mut _)
    }
}

use std::ptr::NonNull;

impl<T> Queueable for NonNull<T> {
    #[inline]
    fn into_nonzero(self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(self.as_ptr() as usize)
        }
    }

    #[inline]
    unsafe fn from_nonzero(n: NonZeroUsize) -> Self {
        NonNull::new_unchecked(n.get() as *mut _)
    }
}
