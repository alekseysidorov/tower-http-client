//! Useful utilities for constructing HTTP requests.

use std::{any::Any, future::Future, marker::PhantomData};

use http::{Extensions, HeaderMap, HeaderName, HeaderValue, Method, Uri, Version};
use tower_service::Service;

use super::{IntoUri, ServiceExt as _};

/// An [`http::Request`] builder.
///
/// Generally, this builder copies the behavior of the [`http::request::Builder`],
/// but unlike it, this builder contains a reference to the client and is able to send a
/// constructed request. Also, this builder borrows most useful methods from the [`reqwest`] one.
///
/// [`reqwest`]: https://docs.rs/reqwest/latest/reqwest/struct.RequestBuilder.html
#[derive(Debug)]
pub struct ClientRequest<'a, S, Err, ReqBody, RespBody> {
    service: &'a mut S,
    builder: http::request::Builder,
    body: ReqBody,
    _phantom: PhantomData<(Err, RespBody)>,
}

impl<'a, S, Err, ReqBody, RespBody> ClientRequest<'a, S, Err, ReqBody, RespBody>
where
    ReqBody: Default,
{
    /// Creates a client request builder.
    pub fn builder(service: &'a mut S) -> Self {
        Self {
            service,
            builder: http::Request::builder(),
            body: ReqBody::default(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, S, Err, ReqBody, RespBody> ClientRequest<'a, S, Err, ReqBody, RespBody> {
    /// Sets the HTTP method for this request.
    ///
    /// By default this is `GET`.
    #[must_use]
    pub fn method<T>(mut self, method: T) -> Self
    where
        Method: TryFrom<T>,
        <Method as TryFrom<T>>::Error: Into<http::Error>,
    {
        self.builder = self.builder.method(method);
        self
    }

    /// Sets the URI for this request
    ///
    /// By default this is `/`.
    #[must_use]
    pub fn uri<U: IntoUri>(mut self, uri: U) -> Self
    where
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.builder = self.builder.uri(uri.into_uri());
        self
    }

    /// Set the HTTP version for this request.
    ///
    /// By default this is HTTP/1.1.
    #[must_use]
    pub fn version(mut self, version: Version) -> Self {
        self.builder = self.builder.version(version);
        self
    }

    /// Appends a header to this request.
    ///
    /// This function will append the provided key/value as a header to the
    /// internal [`HeaderMap`] being constructed.  Essentially this is
    /// equivalent to calling [`HeaderMap::append`].
    #[must_use]
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        HeaderValue: TryFrom<V>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.builder = self.builder.header(key, value);
        self
    }

    /// Returns a mutable reference to headers of this request builder.
    ///
    /// If builder contains error returns `None`.
    pub fn headers_mut(&mut self) -> Option<&mut HeaderMap<HeaderValue>> {
        self.builder.headers_mut()
    }

    /// Adds an extension to this builder.
    #[must_use]
    pub fn extension<T>(mut self, extension: T) -> Self
    where
        T: Clone + Any + Send + Sync + 'static,
    {
        self.builder = self.builder.extension(extension);
        self
    }

    /// Returns a mutable reference to the extensions of this request builder.
    ///
    /// If builder contains error returns `None`.
    #[must_use]
    pub fn extensions_mut(&mut self) -> Option<&mut Extensions> {
        self.builder.extensions_mut()
    }

    /// Sets a body for this request.
    ///
    /// Unlike the [`http::request::Builder`] this function doesn't consume builder.
    /// This allows to override the request body.
    pub fn body<NewReqBody>(
        self,
        body: impl Into<NewReqBody>,
    ) -> ClientRequest<'a, S, Err, NewReqBody, RespBody> {
        ClientRequest {
            service: self.service,
            builder: self.builder,
            body: body.into(),
            _phantom: PhantomData,
        }
    }

    /// Sets a JSON body for this request.
    ///
    /// Additionally this method adds a `CONTENT_TYPE` header for JSON body.
    /// If you decide to override the request body, keep this in mind.
    ///
    /// # Errors
    ///
    /// If the given value's implementation of [`serde::Serialize`] decides to fail.
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json<T: serde::Serialize + ?Sized>(
        mut self,
        value: &T,
    ) -> Result<ClientRequest<'a, S, Err, bytes::Bytes, RespBody>, serde_json::Error> {
        use http::header::CONTENT_TYPE;

        let bytes = bytes::Bytes::from(serde_json::to_vec(value)?);
        if let Some(headers) = self.headers_mut() {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        Ok(self.body(bytes))
    }

    /// Sets a form body for this request.
    ///
    /// Additionally this method adds a `CONTENT_TYPE` header for form body.
    /// If you decide to override the request body, keep this in mind.
    ///
    /// # Errors
    ///
    /// If the given value's implementation of [`serde::Serialize`] decides to fail.
    #[cfg(feature = "form")]
    #[cfg_attr(docsrs, doc(cfg(feature = "form")))]
    pub fn form<T: serde::Serialize + ?Sized>(
        mut self,
        form: &T,
    ) -> Result<ClientRequest<'a, S, Err, bytes::Bytes, RespBody>, serde_urlencoded::ser::Error>
    {
        use http::header::CONTENT_TYPE;

        let string = serde_urlencoded::to_string(form)?;
        if let Some(headers) = self.headers_mut() {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }
        Ok(self.body(string))
    }

    /// Consumes this builder and returns a constructed request.
    ///
    /// # Errors
    ///
    /// If erroneous data was passed during the query building process.
    pub fn build(self) -> Result<http::Request<ReqBody>, http::Error> {
        self.builder.body(self.body)
    }
}

/// Workaround for impl trait lifetimes capturing rules:
/// https://github.com/rust-lang/rust/issues/34511#issuecomment-373423999
#[doc(hidden)]
pub trait Captures<U> {}

impl<T: ?Sized, U> Captures<U> for T {}

impl<'a, S, Err, R, RespBody> ClientRequest<'a, S, Err, R, RespBody> {
    /// Constructs the request and sends it to the target URI.
    pub fn send<ReqBody>(
        self,
    ) -> Result<
        impl Future<Output = Result<http::Response<RespBody>, Err>> + Captures<&'a ()>,
        http::Error,
    >
    where
        S: Service<http::Request<ReqBody>, Response = http::Response<RespBody>, Error = Err>,
        S::Future: Send + 'static,
        S::Error: 'static,
        ReqBody: From<R>,
    {
        let request = self.builder.body(self.body)?;
        Ok(self.service.execute(request))
    }
}
