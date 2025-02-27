//! Useful utilities for constructing HTTP requests.

use std::{any::Any, future::Future, marker::PhantomData};

use http::{Extensions, HeaderMap, HeaderName, HeaderValue, Method, Uri, Version};
use tower_service::Service;

use super::{IntoUri, ServiceExt as _};

type EmptyBody = ();

const EMPTY_BODY: EmptyBody = ();

/// An [`http::Request`] builder.
///
/// Generally, this builder copies the behavior of the [`http::request::Builder`],
/// but unlike it, this builder contains a reference to the client and is able to send a
/// constructed request. Also, this builder borrows most useful methods from the [`reqwest`] one.
///
/// [`reqwest`]: https://docs.rs/reqwest/latest/reqwest/struct.RequestBuilder.html
#[derive(Debug)]
pub struct ClientRequestBuilder<'a, S, Err, RespBody> {
    service: &'a mut S,
    builder: http::request::Builder,
    _phantom: PhantomData<(Err, RespBody)>,
}

impl<'a, S, Err, RespBody> ClientRequestBuilder<'a, S, Err, RespBody> {
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
    ) -> Result<ClientRequest<'a, S, Err, NewReqBody, RespBody>, http::Error> {
        Ok(ClientRequest {
            service: self.service,
            request: self.builder.body(body.into())?,
            _phantom: PhantomData,
        })
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
        self,
        value: &T,
    ) -> Result<
        ClientRequest<'a, S, Err, bytes::Bytes, RespBody>,
        super::request_ext::SetBodyError<serde_json::Error>,
    > {
        use super::RequestBuilderExt as _;

        Ok(ClientRequest {
            service: self.service,
            request: self.builder.json(value)?,
            _phantom: PhantomData,
        })
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
        self,
        form: &T,
    ) -> Result<
        ClientRequest<'a, S, Err, String, RespBody>,
        super::request_ext::SetBodyError<serde_urlencoded::ser::Error>,
    > {
        use super::RequestBuilderExt as _;

        Ok(ClientRequest {
            service: self.service,
            request: self.builder.form(form)?,
            _phantom: PhantomData,
        })
    }

    /// Consumes this builder and returns a constructed request without a body.
    ///
    /// # Errors
    ///
    /// If erroneous data was passed during the query building process.
    #[allow(clippy::missing_panics_doc)]
    pub fn build(self) -> ClientRequest<'a, S, Err, EmptyBody, RespBody> {
        ClientRequest {
            service: self.service,
            request: self
                .builder
                .body(EMPTY_BODY)
                .expect("failed to build request without a body"),
            _phantom: PhantomData,
        }
    }
}

/// An [`http::Request`] wrapper with a reference to a client.
///
/// This struct is used to send constructed HTTP request by using a client.
#[derive(Debug)]
pub struct ClientRequest<'a, S, Err, ReqBody, RespBody> {
    service: &'a mut S,
    request: http::Request<ReqBody>,
    _phantom: PhantomData<(Err, RespBody)>,
}

impl<'a, S, Err, RespBody> ClientRequest<'a, S, Err, (), RespBody> {
    /// Creates a client request builder.
    pub fn builder(service: &'a mut S) -> ClientRequestBuilder<'a, S, Err, RespBody> {
        ClientRequestBuilder {
            service,
            builder: http::Request::builder(),
            _phantom: PhantomData,
        }
    }
}

/// Workaround for impl trait lifetimes capturing rules:
/// https://github.com/rust-lang/rust/issues/34511#issuecomment-373423999
#[doc(hidden)]
pub trait Captures<U> {}

impl<T: ?Sized, U> Captures<U> for T {}

impl<'a, S, Err, RespBody> ClientRequestBuilder<'a, S, Err, RespBody> {
    /// Sends the request to the target URI.
    #[allow(clippy::missing_panics_doc)]
    pub fn send<ReqBody>(
        self,
    ) -> impl Future<Output = Result<http::Response<RespBody>, Err>> + Captures<&'a ()>
    where
        S: Service<http::Request<ReqBody>, Response = http::Response<RespBody>, Error = Err>,
        S::Future: Send + 'static,
        S::Error: 'static,
        ReqBody: Default,
    {
        let request = self
            .builder
            .body(ReqBody::default())
            .expect("failed to build request without a body");
        self.service.execute(request)
    }
}

impl<'a, S, Err, R, RespBody> ClientRequest<'a, S, Err, R, RespBody> {
    /// Sends the request to the target URI.
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
        Ok(self.service.execute(self.request))
    }
}

#[cfg(test)]
mod tests {
    use http::Method;
    use reqwest::Client;
    use tower::ServiceBuilder;
    use tower_reqwest::HttpClientLayer;

    use crate::ServiceExt as _;

    // Check that client request builder uses proper methods.
    #[test]
    fn test_service_ext_request_builder_methods() {
        let mut fake_client = ServiceBuilder::new()
            .layer(HttpClientLayer)
            .service(Client::new());

        assert_eq!(
            fake_client.get("http://localhost").build().request.method(),
            Method::GET
        );
        assert_eq!(
            fake_client
                .post("http://localhost")
                .build()
                .request
                .method(),
            Method::POST
        );
        assert_eq!(
            fake_client.put("http://localhost").build().request.method(),
            Method::PUT
        );
        assert_eq!(
            fake_client
                .patch("http://localhost")
                .build()
                .request
                .method(),
            Method::PATCH
        );
        assert_eq!(
            fake_client
                .delete("http://localhost")
                .build()
                .request
                .method(),
            Method::DELETE
        );
        assert_eq!(
            fake_client
                .head("http://localhost")
                .build()
                .request
                .method(),
            Method::HEAD
        );
    }
}
