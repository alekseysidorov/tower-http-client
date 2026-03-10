//! # Overview
//!
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_utils::include_md!("README.md:description")]
//!
//! An example of multi-threaded concurrent requests sending routine with the requests rate limit.
//!
//! ```rust
#![doc = include_str!("../examples/rate_limiter.rs")]
//! ```

pub use client::{RequestBuilderExt, ResponseExt, ServiceExt};

pub mod client;
#[cfg(feature = "rewrite-uri")]
#[cfg_attr(docsrs, doc(cfg(feature = "rewrite-uri")))]
pub mod rewrite_uri;
