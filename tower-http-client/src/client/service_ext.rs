use std::future::Future;

use http::{Method, Uri};
use tower_service::Service;

use super::{client_request::ClientRequestBuilder, ClientRequest, IntoUri};

/// An extension trait for Tower HTTP services with the typical client methods.
///
/// Essentially, this trait adds methods similar to those in [`reqwest::Client`] one.
///
/// # Examples
///
/// Creating a client and reading the response body using this trait.
///
#[doc = include_utils::include_md!("README.md:example")]
///
/// [`reqwest::Client`]: https://docs.rs/reqwest/latest/reqwest/struct.Client.html
pub trait ServiceExt<ReqBody, RespBody, Err>: Sized {
    /// Executes an HTTP request.
    fn execute<R>(
        &mut self,
        request: http::Request<R>,
    ) -> impl Future<Output = Result<http::Response<RespBody>, Err>>
    where
        ReqBody: From<R>;

    /// Starts building a request with the given method and URI.
    fn request<U>(
        &mut self,
        method: Method,
        uri: U,
    ) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        ClientRequest::builder(self).method(method).uri(uri)
    }

    /// Convenience method to make a `GET` request to a given URL.
    fn get<U>(&mut self, uri: U) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.request(Method::GET, uri)
    }

    /// Convenience method to make a `PUT` request to a given URL.
    fn put<U>(&mut self, uri: U) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.request(Method::PUT, uri)
    }

    /// Convenience method to make a `POST` request to a given URL.
    fn post<U>(&mut self, uri: U) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.request(Method::POST, uri)
    }

    /// Convenience method to make a `PATCH` request to a given URL.
    fn patch<U>(&mut self, uri: U) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.request(Method::PATCH, uri)
    }

    /// Convenience method to make a `DELETE` request to a given URL.
    fn delete<U>(&mut self, uri: U) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.request(Method::DELETE, uri)
    }

    /// Convenience method to make a `HEAD` request to a given URL.
    fn head<U>(&mut self, uri: U) -> ClientRequestBuilder<'_, Self, Err, RespBody>
    where
        U: IntoUri,
        Uri: TryFrom<U::TryInto>,
        <Uri as TryFrom<U::TryInto>>::Error: Into<http::Error>,
    {
        self.request(Method::HEAD, uri)
    }
}

impl<S, ReqBody, RespBody, Err> ServiceExt<ReqBody, RespBody, Err> for S
where
    S: Service<http::Request<ReqBody>, Response = http::Response<RespBody>, Error = Err>,
    S::Future: Send + 'static,
    S::Error: 'static,
{
    async fn execute<R>(
        &mut self,
        request: http::Request<R>,
    ) -> Result<http::Response<RespBody>, Err>
    where
        ReqBody: From<R>,
    {
        // Wait until service will be ready to executing requests. It's important for buffered services.
        futures_util::future::poll_fn(|ctx| self.poll_ready(ctx)).await?;
        // And then execute the given request.
        self.call(request.map(ReqBody::from)).await
    }
}
