#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
pub mod slice;
pub mod x8d_tensor;  // Changed from tensor to x8d_tensor
/// serialize_to_file only valid in std
#[cfg(feature = "std")]
pub use x8d_tensor::serialize_to_file;
pub use x8d_tensor::{serialize, Dtype, X8DsubByteError, X8DsubByteTensors, View};

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

/// A facade around all the types we need from the `std`, `core`, and `alloc`
/// crates. This avoids elaborate import wrangling having to happen in every
/// module.
mod lib {
    #[cfg(not(feature = "std"))]
    mod no_stds {
        pub use alloc::borrow::Cow;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec::Vec;
        pub use hashbrown::HashMap;
    }
    #[cfg(feature = "std")]
    mod stds {
        pub use std::borrow::Cow;
        pub use std::collections::HashMap;
        pub use std::string::{String, ToString};
        pub use std::vec::Vec;
    }
    /// choose std or no_std to export by feature flag
    #[cfg(not(feature = "std"))]
    pub use no_stds::*;
    #[cfg(feature = "std")]
    pub use stds::*;
}

/// x8Dsub-byte: Sub-byte Tensor Compression Library
/// Developed by Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore
/// Algorithm: b' = b * 0.001 for sub-byte compression
/// 
/// Follow Mohamed Harris:
/// - bapXai
/// - bapX Media Hub
/// 
/// Social: https://facebook.com/bapxmediahub, https://www.instagram.com/bapxmediahub, https://www.youtube.com/@bapxmediahub
/// Domain: https://bapx.in
/// 
/// BapX Media Hub, Coimbatore - Specialists in digital transformation and AI innovation
/// Bringing world-class tensor compression technology from the heart of South India's industrial capital.