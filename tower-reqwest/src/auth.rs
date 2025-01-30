//! Middlweare for adding [`Authorization`] header to requests.
//!
//! This module borrows heavily from the `auth` module in the `tower-http` crate.
//!
//! [`Authorization`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Authorization

use std::task::{Context, Poll};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use http::header::{HeaderValue, InvalidHeaderValue};
use tower_layer::Layer;
use tower_service::Service;

/// Layer which adds authorization to all requests using the [`Authorization`] header.
///
/// [`Authorization`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Authorization
#[derive(Debug, Clone)]
pub struct AddAuthorizationLayer {
    value: HeaderValue,
}

/// Middleware that adds authorization all requests using the [`Authorization`] header.
///
/// [`Authorization`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Authorization
#[derive(Debug, Clone)]
pub struct AddAuthorizationService<S> {
    inner: S,
    value: HeaderValue,
}

impl AddAuthorizationLayer {
    /// Authorize requests using a username and password pair.
    ///
    /// The `Authorization` header will be set to `Basic {credentials}` where `credentials` is
    /// `base64_encode("{username}:{password}")`.
    ///
    /// Since the username and password is sent in clear text it is recommended to use HTTPS/TLS
    /// with this method. However use of HTTPS/TLS is not enforced by this middleware.
    pub fn basic(
        username: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<Self, InvalidHeaderValue> {
        let encoded = BASE64.encode([username.as_ref(), ":", password.as_ref()].concat());
        Ok(Self {
            value: ["Basic ", encoded.as_ref()].concat().parse()?,
        })
    }

    /// Authorize requests using a "bearer token". Commonly used for OAuth 2.
    ///
    /// The `Authorization` header will be set to `Bearer {token}`.
    pub fn bearer(token: impl AsRef<str>) -> Result<Self, InvalidHeaderValue> {
        Ok(Self {
            value: ["Bearer ", token.as_ref()].concat().parse()?,
        })
    }

    /// Mark the header as [sensitive].
    ///
    /// This can for example be used to hide the header value from logs.
    ///
    /// [sensitive]: https://docs.rs/http/latest/http/header/struct.HeaderValue.html#method.set_sensitive
    #[must_use]
    pub fn set_sensitive(mut self, sensitive: bool) -> Self {
        self.value.set_sensitive(sensitive);
        self
    }
}

impl<S> Layer<S> for AddAuthorizationLayer {
    type Service = AddAuthorizationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AddAuthorizationService {
            inner,
            value: self.value.clone(),
        }
    }
}

impl<S> AddAuthorizationService<S> {
    /// Authorize requests using a username and password pair.
    ///
    /// The `Authorization` header will be set to `Basic {credentials}` where `credentials` is
    /// `base64_encode("{username}:{password}")`.
    ///
    /// Since the username and password is sent in clear text it is recommended to use HTTPS/TLS
    /// with this method. However use of HTTPS/TLS is not enforced by this middleware.
    pub fn basic(
        inner: S,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<Self, InvalidHeaderValue> {
        AddAuthorizationLayer::basic(username, password).map(|layer| layer.layer(inner))
    }

    /// Authorize requests using a "bearer token". Commonly used for OAuth 2.
    ///
    /// The `Authorization` header will be set to `Bearer {token}`.
    pub fn bearer(inner: S, token: impl AsRef<str>) -> Result<Self, InvalidHeaderValue> {
        AddAuthorizationLayer::bearer(token).map(|layer| layer.layer(inner))
    }

    /// Mark the header as [sensitive].
    ///
    /// This can for example be used to hide the header value from logs.
    ///
    /// [sensitive]: https://docs.rs/http/latest/http/header/struct.HeaderValue.html#method.set_sensitive
    #[must_use]
    pub fn set_sensitive(mut self, sensitive: bool) -> Self {
        self.value.set_sensitive(sensitive);
        self
    }
}

impl<S> Service<reqwest::Request> for AddAuthorizationService<S>
where
    S: Service<reqwest::Request, Response = reqwest::Response>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: reqwest::Request) -> Self::Future {
        req.headers_mut()
            .insert(http::header::AUTHORIZATION, self.value.clone());
        self.inner.call(req)
    }
}
