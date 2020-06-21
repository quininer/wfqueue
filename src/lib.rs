#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::sync;
}

#[cfg(feature = "loom")]
use loom;

pub mod queue;

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
                self.0.len()
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
