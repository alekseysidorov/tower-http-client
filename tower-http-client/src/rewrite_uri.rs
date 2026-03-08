//! Middleware for rewriting request URIs.
//!
//! This module provides the [`RewriteUri`] trait and the
//! [`RewriteUriLayer`]/[`RewriteUriService`] pair for composable URI rewriting
//! in Tower middleware stacks.
//!
//! # Overview
//!
//! In many client setups the application builds requests using relative URIs,
//! while the transport layer requires absolute URIs.  The [`RewriteUriLayer`]
//! sits between the caller and the inner service and rewrites each request's
//! URI before forwarding it.
//!
//! # Example
//!
//! Using a closure to prepend a base URL:
//!
//! ```rust
//! use http::Uri;
//! use tower::ServiceBuilder;
//! use tower_http_client::rewrite_uri::RewriteUriLayer;
//!
//! let layer = RewriteUriLayer::new(|uri: &Uri| {
//!     let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
//!     format!("http://example.com{path}").parse::<Uri>().map_err(http::Error::from)
//! });
//! ```
//!
//! Using a struct implementing [`RewriteUri`]:
//!
//! ```rust
//! use http::Uri;
//! use tower::ServiceBuilder;
//! use tower_http_client::rewrite_uri::{RewriteUri, RewriteUriLayer};
//!
//! #[derive(Clone)]
//! struct BaseUri {
//!     base: Uri,
//! }
//!
//! impl RewriteUri for BaseUri {
//!     type Error = http::Error;
//!
//!     fn rewrite_uri(&mut self, uri: &Uri) -> Result<Uri, Self::Error> {
//!         let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
//!         format!(
//!             "{}{}",
//!             self.base,
//!             path
//!         )
//!         .parse::<Uri>()
//!         .map_err(http::Error::from)
//!     }
//! }
//! ```

use std::{
    fmt,
    task::{Context, Poll},
};

use futures_util::future::{Either, Ready, ready};
use tower_layer::Layer;
use tower_service::Service;

/// Trait for rewriting URIs on incoming requests.
///
/// Implement this trait to define custom URI rewriting logic.  A blanket
/// implementation is provided for closures of the form
/// `FnMut(&http::Uri) -> Result<http::Uri, E>`.
pub trait RewriteUri {
    /// The error type returned when rewriting fails.
    type Error;

    /// Rewrite the given URI, returning a new URI or an error.
    fn rewrite_uri(&mut self, uri: &http::Uri) -> Result<http::Uri, Self::Error>;
}

impl<F, E> RewriteUri for F
where
    F: FnMut(&http::Uri) -> Result<http::Uri, E>,
{
    type Error = E;

    fn rewrite_uri(&mut self, uri: &http::Uri) -> Result<http::Uri, Self::Error> {
        self(uri)
    }
}

/// Layer that applies URI rewriting to every request via a [`RewriteUri`] policy.
///
/// Wraps an inner service and rewrites the URI of each incoming request before
/// forwarding it.
pub struct RewriteUriLayer<R> {
    rewrite: R,
}

impl<R> RewriteUriLayer<R> {
    /// Create a new [`RewriteUriLayer`] with the given rewrite policy.
    pub fn new(rewrite: R) -> Self {
        Self { rewrite }
    }
}

impl<R> fmt::Debug for RewriteUriLayer<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RewriteUriLayer")
            .field("rewrite", &std::any::type_name::<R>())
            .finish()
    }
}

impl<R: Clone> Clone for RewriteUriLayer<R> {
    fn clone(&self) -> Self {
        Self {
            rewrite: self.rewrite.clone(),
        }
    }
}

impl<S, R: Clone> Layer<S> for RewriteUriLayer<R> {
    type Service = RewriteUriService<S, R>;

    fn layer(&self, inner: S) -> Self::Service {
        RewriteUriService::new(inner, self.rewrite.clone())
    }
}

/// Middleware that rewrites the URI of each request using a [`RewriteUri`] policy.
pub struct RewriteUriService<S, R> {
    inner: S,
    rewrite: R,
}

impl<S, R> RewriteUriService<S, R> {
    /// Create a new [`RewriteUriService`].
    pub fn new(inner: S, rewrite: R) -> Self {
        Self { inner, rewrite }
    }

    /// Returns a reference to the inner service.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Consumes `self`, returning the inner service.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S, R> fmt::Debug for RewriteUriService<S, R>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RewriteUriService")
            .field("inner", &self.inner)
            .field("rewrite", &std::any::type_name::<R>())
            .finish()
    }
}

impl<S, R> Clone for RewriteUriService<S, R>
where
    S: Clone,
    R: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            rewrite: self.rewrite.clone(),
        }
    }
}

impl<S, R, ReqBody> Service<http::Request<ReqBody>> for RewriteUriService<S, R>
where
    S: Service<http::Request<ReqBody>>,
    R: RewriteUri,
    R::Error: Into<S::Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Either<Ready<Result<S::Response, S::Error>>, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<ReqBody>) -> Self::Future {
        match self.rewrite.rewrite_uri(req.uri()) {
            Ok(new_uri) => {
                *req.uri_mut() = new_uri;
                Either::Right(self.inner.call(req))
            }
            Err(e) => Either::Left(ready(Err(e.into()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use http::{Request, Response, Uri};
    use tower::{ServiceBuilder, service_fn};
    use tower_layer::Layer as _;
    use tower_service::Service as _;

    use super::{RewriteUri, RewriteUriLayer, RewriteUriService};

    /// A minimal service that returns the request URI as the response body.
    fn capture_uri_service() -> impl tower_service::Service<
        Request<()>,
        Response = Response<String>,
        Error = Infallible,
    > {
        service_fn(|req: Request<()>| async move {
            Ok::<_, Infallible>(Response::new(req.uri().to_string()))
        })
    }

    #[tokio::test]
    async fn test_rewrite_uri_with_closure() {
        let mut svc = RewriteUriService::new(
            capture_uri_service(),
            |_uri: &Uri| Ok::<_, Infallible>(Uri::from_static("http://example.com/rewritten")),
        );

        let req = Request::builder().uri("/original").body(()).unwrap();
        let response = svc.call(req).await.unwrap();
        assert_eq!(response.into_body(), "http://example.com/rewritten");
    }

    #[tokio::test]
    async fn test_rewrite_uri_layer() {
        let mut svc = RewriteUriLayer::new(|_uri: &Uri| {
            Ok::<_, Infallible>(Uri::from_static("http://example.com/via-layer"))
        })
        .layer(capture_uri_service());

        let req = Request::builder().uri("/original").body(()).unwrap();
        let response = svc.call(req).await.unwrap();
        assert_eq!(response.into_body(), "http://example.com/via-layer");
    }

    #[tokio::test]
    async fn test_rewrite_uri_service_builder() {
        let mut svc = ServiceBuilder::new()
            .layer(RewriteUriLayer::new(|uri: &Uri| {
                let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
                Ok::<_, Infallible>(
                    format!("http://example.com{path}").parse::<Uri>().unwrap(),
                )
            }))
            .service(capture_uri_service());

        let req = Request::builder().uri("/hello").body(()).unwrap();
        let response = svc.call(req).await.unwrap();
        assert_eq!(response.into_body(), "http://example.com/hello");
    }

    #[tokio::test]
    async fn test_rewrite_uri_error_propagates() {
        // Use String as a convenient non-Infallible error type for both service
        // and rewriter so that String: Into<String> is satisfied.
        let inner = service_fn(|_: Request<()>| async {
            Ok::<_, String>(Response::new("ok".to_string()))
        });

        let mut svc = RewriteUriService::new(
            inner,
            |_uri: &Uri| Err::<Uri, String>("rewrite failed".to_string()),
        );

        let req = Request::builder().uri("/original").body(()).unwrap();
        let result = svc.call(req).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "rewrite failed");
    }

    #[tokio::test]
    async fn test_rewrite_uri_struct_impl() {
        #[derive(Clone)]
        struct PrependBase {
            base: &'static str,
        }

        impl RewriteUri for PrependBase {
            type Error = Infallible;

            fn rewrite_uri(&mut self, uri: &Uri) -> Result<Uri, Self::Error> {
                let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
                Ok(format!("{}{path}", self.base).parse().unwrap())
            }
        }

        let mut svc = RewriteUriLayer::new(PrependBase {
            base: "http://backend.internal",
        })
        .layer(capture_uri_service());

        let req = Request::builder().uri("/api/users").body(()).unwrap();
        let response = svc.call(req).await.unwrap();
        assert_eq!(response.into_body(), "http://backend.internal/api/users");
    }
}
