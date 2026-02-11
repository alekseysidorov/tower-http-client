//! Adapter for [`reqwest`] client.
//!
//! [`reqwest`]: https://crates.io/crates/reqwest

use std::{future::Future, task::Poll};

use pin_project::pin_project;
use tower_service::Service;

use crate::HttpClientService;

impl<S> Service<http::Request<reqwest::Body>> for HttpClientService<S>
where
    S: Service<reqwest::Request, Error = reqwest::Error>,
    http::Response<reqwest::Body>: From<S::Response>,
{
    type Response = http::Response<reqwest::Body>;
    type Error = S::Error;
    type Future = ExecuteRequestFuture<S>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<reqwest::Body>) -> Self::Future {
        let future = reqwest::Request::try_from(req).map(|reqw| self.0.call(reqw));
        ExecuteRequestFuture::new(future)
    }
}

/// Future that resolves to the response or failure to connect.
#[pin_project(project = ExecuteRequestFutureProj)]
#[derive(Debug)]
pub enum ExecuteRequestFuture<S>
where
    S: Service<reqwest::Request>,
{
    Future {
        #[pin]
        fut: S::Future,
    },
    Error {
        error: Option<S::Error>,
    },
}

impl<S> ExecuteRequestFuture<S>
where
    S: Service<reqwest::Request>,
{
    fn new(future: Result<S::Future, S::Error>) -> Self {
        match future {
            Ok(fut) => Self::Future { fut },
            Err(error) => Self::Error { error: Some(error) },
        }
    }
}

impl<S> Future for ExecuteRequestFuture<S>
where
    S: Service<reqwest::Request>,
    http::Response<reqwest::Body>: From<S::Response>,
{
    type Output = Result<http::Response<reqwest::Body>, S::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            ExecuteRequestFutureProj::Future { fut } => fut.poll(cx).map_ok(From::from),
            ExecuteRequestFutureProj::Error { error } => {
                let error = error.take().expect("Polled after ready");
                Poll::Ready(Err(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{FutureExt, TryFutureExt as _, future::BoxFuture};
    use http::{HeaderName, HeaderValue, header::USER_AGENT};
    use http_body_util::BodyExt;
    use pretty_assertions::assert_eq;
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use tower::{Layer, Service, ServiceBuilder, ServiceExt};
    use tower_http::{ServiceBuilderExt, request_id::MakeRequestUuid};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use crate::HttpClientLayer;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Info {
        student: String,
        answer: u32,
        request_id: Option<String>,
    }

    impl Info {
        async fn from_body(body: reqwest::Body) -> anyhow::Result<Self> {
            let body_bytes = body.collect().await?.to_bytes();
            let info: Info = serde_json::from_slice(&body_bytes)?;
            Ok(info)
        }
    }

    #[tokio::test]
    async fn test_http_client_layer() -> anyhow::Result<()> {
        // Start a background HTTP server on a random local port
        let mock_server = MockServer::start().await;
        // Get mock server base uri
        let mock_uri = mock_server.uri();

        // Arrange the behaviour of the MockServer adding a Mock:
        // when it receives a GET request on '/hello' it will respond with a 200.
        Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(|req: &wiremock::Request| {
                let request_id = req
                    .headers
                    .get(HeaderName::from_static("x-request-id"))
                    .map(|value| value.to_str().unwrap().to_owned());

                ResponseTemplate::new(200).set_body_json(Info {
                    student: "Vasya Pupkin".to_owned(),
                    answer: 42,
                    request_id,
                })
            })
            // Mounting the mock on the mock server - it's now effective!
            .mount(&mock_server)
            .await;
        // Create HTTP client
        let client = Client::new();

        // Execute request without layers
        let request = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(format!("{mock_uri}/hello"))
            // TODO Make in easy to create requests without body.
            .body(reqwest::Body::default())?;

        let response = ServiceBuilder::new()
            .layer(HttpClientLayer)
            .service(client.clone())
            .call(request)
            .await?;
        assert!(response.status().is_success());
        // Try to read body
        let info = Info::from_body(response.into_body()).await?;
        assert!(info.request_id.is_none());

        // TODO Find the way to avoid cloning the service.
        let service = ServiceBuilder::new()
            .override_response_header(USER_AGENT, HeaderValue::from_static("tower-reqwest"))
            .set_x_request_id(MakeRequestUuid)
            .map_err(|err: FailableServiceError<reqwest::Error>| anyhow::Error::from(err))
            .layer(FailableServiceLayer)
            .layer(HttpClientLayer)
            .service(client)
            .boxed_clone();
        // Execute request with a several layers from the tower-http
        let request = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(format!("{mock_uri}/hello"))
            // TODO Make in easy to create requests without body.
            .body(reqwest::Body::default())?;
        let response = service
            .clone()
            .call(request)
            .await
            .inspect_err(|_: &anyhow::Error| {})?;

        assert!(response.status().is_success());
        assert_eq!(
            response.headers().get(USER_AGENT).unwrap(),
            HeaderValue::from_static("tower-reqwest")
        );

        // Try to read body again.
        let info = Info::from_body(response.into_body()).await?;
        assert_eq!(info.student, "Vasya Pupkin");
        assert_eq!(info.answer, 42);
        assert!(info.request_id.is_some());

        Ok(())
    }

    #[derive(Debug, Clone)]
    struct FailableServiceLayer;

    impl<S> Layer<S> for FailableServiceLayer {
        type Service = FailableService<S>;

        fn layer(&self, inner: S) -> Self::Service {
            FailableService { inner }
        }
    }

    #[derive(Debug, Clone)]
    struct FailableService<S> {
        inner: S,
    }

    impl<S, B> Service<http::Request<B>> for FailableService<S>
    where
        S: Service<http::Request<B>>,
        S::Future: Send + 'static,
        S::Error: 'static,
    {
        type Response = S::Response;
        type Error = FailableServiceError<S::Error>;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            self.inner
                .poll_ready(cx)
                .map_err(FailableServiceError::Inner)
        }

        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            self.inner
                .call(req)
                .map_err(FailableServiceError::Inner)
                .boxed()
        }
    }

    #[derive(Debug, Clone, thiserror::Error)]
    #[error("i'm failed")]
    #[allow(unused)]
    enum FailableServiceError<E> {
        Inner(E),
        Other,
    }
}
