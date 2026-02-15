#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![warn(missing_docs)]

//! # Overview
//!
#![doc = include_utils::include_md!("README.md:description")]
//!
//! An example of multi-threaded concurrent requests sending routine with the requests rate limit.
//!
//! ```rust
#![doc = include_str!("../examples/rate_limiter.rs")]
//! ```

pub use client::{RequestBuilderExt, ResponseExt, ServiceExt};

pub mod client;
