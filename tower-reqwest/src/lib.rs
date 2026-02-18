#![cfg_attr(docsrs, feature(doc_cfg))]

//! # Overview
//!
#![doc = include_utils::include_md!("README.md:description")]

use tower_layer::Layer;

mod adapters;
#[cfg(feature = "auth")]
pub mod auth;
#[cfg(feature = "set-header")]
pub mod set_header;

/// Adapter type to creating Tower HTTP services from the various clients.
#[derive(Debug, Clone)]
pub struct HttpClientService<S>(S);

impl<S> HttpClientService<S> {
    /// Creates a new HTTP client service wrapper.
    pub const fn new(inner: S) -> Self {
        Self(inner)
    }
}

/// Layer that creates [`HttpClientService`] from the inner service.
///
/// # Examples
///
#[doc = include_utils::include_md!("README.md:description")]
///
#[derive(Debug, Clone, Copy)]
pub struct HttpClientLayer;

impl<S> Layer<S> for HttpClientLayer {
    type Service = HttpClientService<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpClientService(service)
    }
}
