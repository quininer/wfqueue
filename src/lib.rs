#![deny(unsafe_code)]

#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::sync;
}

#[cfg(feature = "loom")]
use loom;

pub mod queue;

/// # Wait-free queue codegen macro
///
/// Generate a queue type and
/// use a separate thread local storage to store the context.
///
/// # Example
///
/// ```rust
/// use std::num::NonZeroUsize;
///
/// fn box_into_nonzero(input: Box<usize>) -> NonZeroUsize {
///     let input = Box::into_raw(input) as usize;
///
///     unsafe {
///         NonZeroUsize::new_unchecked(input)
///     }
/// }
///
/// fn box_from_nonzero(output: NonZeroUsize) -> Box<usize> {
///     let output = output.get() as *mut usize;
///
///     unsafe {
///         Box::from_raw(output)
///     }
/// }
///
/// wfqueue::codegen! {
///     pub struct BoxQueue(Box<usize>);
///
///     fn into_nonzero = box_into_nonzero;
///     fn from_nonzero = box_from_nonzero;
/// }
///
/// # #[cfg(not(feature = "loom"))] {
/// let queue = BoxQueue::new(3);
///
/// queue.push(Box::new(0x42)).unwrap();
/// let output = queue.pop().unwrap();
///
/// assert_eq!(*output, 0x42);
/// # }
/// ```
#[macro_export]
macro_rules! codegen {
    (
        pub struct $name:ident ( $item:ty );

        fn into_nonzero = $into:expr;
        fn from_nonzero = $from:expr;
    ) => {
        pub struct $name {
            queue: $crate::queue::WfQueue
        }

        #[cfg(not(feature = "loom"))]
        paste::item! {
            std::thread_local! {
                #[allow(non_upper_case_globals)]
                static [<$name _CTX>]: $crate::Context = $crate::Context::new();
            }
        }

        #[cfg(feature = "loom")]
        paste::item! {
            loom::thread_local! {
                static [<$name _CTX>]: $crate::Context = $crate::Context::new();
            }
        }

        impl $name {
            #[inline]
            pub fn new(cap: usize) -> $name {
                $name {
                    queue: $crate::queue::WfQueue::new(cap)
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

            pub fn push(&self, input: $item) -> Result<(), $item> {
                let input: std::num::NonZeroUsize = $into(input);

                if paste::expr!{ &[<$name _CTX>] }
                    .with(|ctx| self.queue.try_enqueue(&ctx.enq, input))
                {
                    Ok(())
                } else {
                    Err($from(input))
                }
            }

            pub fn pop(&self) -> Option<$item> {
                let output = paste::expr!{ &[<$name _CTX>] }
                    .with(|ctx| self.queue.try_dequeue(&ctx.deq))?;
                Some($from(output))
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                while self.pop().is_some() {}
            }
        }
    }
}

#[doc(hidden)]
pub struct Context {
    pub enq: queue::EnqueueCtx,
    pub deq: queue::DequeueCtx
}

impl Context {
    pub const fn new() -> Context {
        Context {
            enq: queue::EnqueueCtx::new(),
            deq: queue::DequeueCtx::new()
        }
    }
}
