#![deny(unsafe_code)]

#[cfg(not(feature = "loom"))]
mod loom {
    pub use std::sync;
}

#[cfg(feature = "loom")]
use loom;

pub mod queue;
