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
                static [<$name _ENQ_CTX>]: $crate::queue::EnqueueCtx = $crate::queue::EnqueueCtx::new();
                static [<$name _DEQ_CTX>]: $crate::queue::DequeueCtx = $crate::queue::DequeueCtx::new();
            }
        }

        #[cfg(feature = "loom")]
        paste::item! {
            loom::thread_local! {
                static [<$name _ENQ_CTX>]: $crate::queue::EnqueueCtx = $crate::queue::EnqueueCtx::new();
                static [<$name _DEQ_CTX>]: $crate::queue::DequeueCtx = $crate::queue::DequeueCtx::new();
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

            pub fn push(&self, input: $item) -> Result<(), $item> {
                let input: std::num::NonZeroUsize = $into(input);

                if paste::expr!{ &[<$name _ENQ_CTX>] }
                    .with(|ctx| self.queue.try_enqueue(ctx, input))
                {
                    Ok(())
                } else {
                    Err($from(input))
                }
            }

            pub fn pop(&self) -> Option<$item> {
                let output = paste::expr!{ &[<$name _DEQ_CTX>] }
                    .with(|ctx| self.queue.try_dequeue(ctx))?;
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
