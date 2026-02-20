//! Middleware for setting headers on requests.
//!
//! This module borrows heavily from the `set-header` module in the `tower-http` crate.
//! The main difference is that this module is designed to work with the `reqwest` client,
//! while the [`set-header`] module is designed to work with the abstract `http` service.
//!
//! # Example
//!
//! Setting a header from a fixed value
//!
//! ```
#![doc = include_str!("../examples/set_header.rs")]
//! ```
//!
//! [`set-header`]: https://docs.rs/tower-http/latest/tower_http/set_header/index.html

use std::{
    fmt,
    task::{Context, Poll},
};

use http::{HeaderName, HeaderValue};
use tower_layer::Layer;
use tower_service::Service;

/// Trait for producing header values.
///
/// This trait is implemented for closures with the correct type signature. Typically users will
/// not have to implement this trait for their own types.
///
/// It is also implemented directly for [`HeaderValue`]. When a fixed header value should be added
/// to all responses, it can be supplied directly to the middleware.
pub trait MakeHeaderValue<T> {
    /// Try to create a header value from the request or response.
    fn make_header_value(&mut self, message: &T) -> Option<HeaderValue>;
}

impl<F, T> MakeHeaderValue<T> for F
where
    F: FnMut(&T) -> Option<HeaderValue>,
{
    fn make_header_value(&mut self, message: &T) -> Option<HeaderValue> {
        self(message)
    }
}

impl<T> MakeHeaderValue<T> for HeaderValue {
    fn make_header_value(&mut self, _message: &T) -> Option<HeaderValue> {
        Some(self.clone())
    }
}

impl<T> MakeHeaderValue<T> for Option<HeaderValue> {
    fn make_header_value(&mut self, _message: &T) -> Option<HeaderValue> {
        self.clone()
    }
}

#[derive(Debug, Clone, Copy)]
enum InsertHeaderMode {
    Override,
    Append,
    IfNotPresent,
}

impl InsertHeaderMode {
    fn apply<M>(self, header_name: &HeaderName, target: &mut reqwest::Request, make: &mut M)
    where
        M: MakeHeaderValue<reqwest::Request>,
    {
        match self {
            InsertHeaderMode::Override => {
                if let Some(value) = make.make_header_value(target) {
                    target.headers_mut().insert(header_name.clone(), value);
                }
            }
            InsertHeaderMode::IfNotPresent => {
                if !target.headers().contains_key(header_name)
                    && let Some(value) = make.make_header_value(target)
                {
                    target.headers_mut().insert(header_name.clone(), value);
                }
            }
            InsertHeaderMode::Append => {
                if let Some(value) = make.make_header_value(target) {
                    target.headers_mut().append(header_name.clone(), value);
                }
            }
        }
    }
}

/// Layer that applies [`SetRequestHeader`] which adds a request header.
///
/// # Example
///
/// Setting a header from a fixed value
///
/// ```
#[doc = include_str!("../examples/set_header.rs")]
/// ```
pub struct SetRequestHeaderLayer<M> {
    header_name: HeaderName,
    make: M,
    mode: InsertHeaderMode,
}

impl<M> fmt::Debug for SetRequestHeaderLayer<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetRequestHeaderLayer")
            .field("header_name", &self.header_name)
            .field("mode", &self.mode)
            .field("make", &std::any::type_name::<M>())
            .finish()
    }
}

impl<M> SetRequestHeaderLayer<M>
where
    M: MakeHeaderValue<reqwest::Request>,
{
    /// Create a new [`SetRequestHeaderLayer`].
    ///
    /// If a previous value exists for the same header, it is removed and replaced with the new
    /// header value.
    pub fn overriding(header_name: HeaderName, make: M) -> Self {
        Self::new(header_name, make, InsertHeaderMode::Override)
    }

    /// Create a new [`SetRequestHeaderLayer`].
    ///
    /// The new header is always added, preserving any existing values. If previous values exist,
    /// the header will have multiple values.
    pub fn appending(header_name: HeaderName, make: M) -> Self {
        Self::new(header_name, make, InsertHeaderMode::Append)
    }

    /// Create a new [`SetRequestHeaderLayer`].
    ///
    /// If a previous value exists for the header, the new value is not inserted.
    pub fn if_not_present(header_name: HeaderName, make: M) -> Self {
        Self::new(header_name, make, InsertHeaderMode::IfNotPresent)
    }

    fn new(header_name: HeaderName, make: M, mode: InsertHeaderMode) -> Self {
        Self {
            header_name,
            make,
            mode,
        }
    }
}

impl<S, M> Layer<S> for SetRequestHeaderLayer<M>
where
    M: Clone,
{
    type Service = SetRequestHeader<S, M>;

    fn layer(&self, inner: S) -> Self::Service {
        SetRequestHeader {
            inner,
            header_name: self.header_name.clone(),
            make: self.make.clone(),
            mode: self.mode,
        }
    }
}

impl<M> Clone for SetRequestHeaderLayer<M>
where
    M: Clone,
{
    fn clone(&self) -> Self {
        Self {
            make: self.make.clone(),
            header_name: self.header_name.clone(),
            mode: self.mode,
        }
    }
}

/// Middleware that sets a header on the request.
#[derive(Clone)]
pub struct SetRequestHeader<S, M> {
    inner: S,
    header_name: HeaderName,
    make: M,
    mode: InsertHeaderMode,
}

impl<S, M> SetRequestHeader<S, M> {
    /// Create a new [`SetRequestHeader`].
    ///
    /// If a previous value exists for the same header, it is removed and replaced with the new
    /// header value.
    pub fn overriding(inner: S, header_name: HeaderName, make: M) -> Self {
        Self::new(inner, header_name, make, InsertHeaderMode::Override)
    }

    /// Create a new [`SetRequestHeader`].
    ///
    /// The new header is always added, preserving any existing values. If previous values exist,
    /// the header will have multiple values.
    pub fn appending(inner: S, header_name: HeaderName, make: M) -> Self {
        Self::new(inner, header_name, make, InsertHeaderMode::Append)
    }

    /// Create a new [`SetRequestHeader`].
    ///
    /// If a previous value exists for the header, the new value is not inserted.
    pub fn if_not_present(inner: S, header_name: HeaderName, make: M) -> Self {
        Self::new(inner, header_name, make, InsertHeaderMode::IfNotPresent)
    }

    fn new(inner: S, header_name: HeaderName, make: M, mode: InsertHeaderMode) -> Self {
        Self {
            inner,
            header_name,
            make,
            mode,
        }
    }
}

impl<S, M> fmt::Debug for SetRequestHeader<S, M>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetRequestHeader")
            .field("inner", &self.inner)
            .field("header_name", &self.header_name)
            .field("mode", &self.mode)
            .field("make", &std::any::type_name::<M>())
            .finish()
    }
}

impl<S, M> Service<reqwest::Request> for SetRequestHeader<S, M>
where
    S: Service<reqwest::Request, Response = reqwest::Response>,
    M: MakeHeaderValue<reqwest::Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: reqwest::Request) -> Self::Future {
        self.mode.apply(&self.header_name, &mut req, &mut self.make);
        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {

    use http::{HeaderName, HeaderValue};
    use tower_layer::Layer;
    use tower_service::Service;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use crate::set_header::SetRequestHeaderLayer;

    #[tokio::test]
    async fn test_set_headers() -> anyhow::Result<()> {
        let mock_server = MockServer::start().await;
        let mock_uri = mock_server.uri();

        let header_name = HeaderName::from_static("x-test-header");
        let header_value = HeaderValue::from_static("test-value");

        Mock::given(method("GET"))
            .and(path("/test"))
            .and(wiremock::matchers::header(&header_name, &header_value))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let uri = format!("{mock_uri}/test");
        let request = reqwest::Request::new(reqwest::Method::GET, uri.parse()?);

        let client = reqwest::Client::new();
        // Check that the header is not set by default.
        let response = client.execute(request.try_clone().unwrap()).await?;
        assert_eq!(response.status(), 404);
        // Check that the header will be set by the layer.
        let response = SetRequestHeaderLayer::overriding(header_name, header_value)
            .layer(client)
            .call(request)
            .await?;
        assert_eq!(response.status(), 200);

        Ok(())
    }
}
