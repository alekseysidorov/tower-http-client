use std::{
    ops::ControlFlow,
    sync::atomic::{AtomicI32, Ordering::SeqCst},
    time::SystemTime,
};

use bytes::Bytes;
use retry_policies::{RetryDecision, policies::ExponentialBackoff};
use tower::{ServiceBuilder, ServiceExt as _};
use tower_http::ServiceBuilderExt as _;
use tower_http_client::client::ServiceExt as _;
use tower_reqwest::HttpClientLayer;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[derive(Debug, Clone)]
pub struct RetrySequence<P> {
    policy: P,
    start_time: Option<SystemTime>,
    n_past_retries: u32,
}

impl<P> RetrySequence<P> {
    pub fn new(policy: P) -> Self {
        Self {
            policy,
            start_time: None,
            n_past_retries: 0,
        }
    }

    pub fn next_attempt(&mut self) -> ControlFlow<(), (SystemTime, &mut Self)>
    where
        P: retry_policies::RetryPolicy,
    {
        let start_time = self.start_time();
        match self.policy.should_retry(start_time, self.n_past_retries) {
            RetryDecision::Retry { execute_after } => {
                self.start_time = Some(execute_after);
                self.n_past_retries += 1;
                ControlFlow::Continue((execute_after, self))
            }
            RetryDecision::DoNotRetry => ControlFlow::Break(()),
        }
    }

    fn start_time(&self) -> SystemTime {
        self.start_time.unwrap_or_else(SystemTime::now)
    }
}

#[derive(Debug, Clone)]
pub struct SimpleRetry(RetrySequence<ExponentialBackoff>);

impl SimpleRetry {
    #[must_use]
    pub fn new(policy: ExponentialBackoff) -> Self {
        Self(RetrySequence::new(policy))
    }
}

impl<ReqBody: Clone, RespBody, E>
    tower::retry::Policy<http::Request<ReqBody>, http::Response<RespBody>, E> for SimpleRetry
{
    type Future = tokio::time::Sleep;

    fn retry(
        &mut self,
        _req: &mut http::Request<ReqBody>,
        result: &mut Result<http::Response<RespBody>, E>,
    ) -> Option<Self::Future> {
        match result {
            Ok(resp) if !resp.status().is_server_error() => {
                // Treat almost all `Response`s as success,
                // so don't retry...
                None
            }

            _other => match self.0.next_attempt() {
                ControlFlow::Continue((retry_at, next_attempt)) => {
                    let n_past_retries = next_attempt.n_past_retries;
                    let sleep_duration = retry_at
                        .duration_since(SystemTime::now())
                        .unwrap_or_default();

                    eprintln!(
                        "Making attempt #{n_past_retries} sleeping for {:.3}secs",
                        sleep_duration.as_secs_f32()
                    );

                    Some(tokio::time::sleep(sleep_duration))
                }
                // Used all our attempts, no retry...
                ControlFlow::Break(()) => None,
            },
        }
    }

    fn clone_request(&mut self, req: &http::Request<ReqBody>) -> Option<http::Request<ReqBody>> {
        Some(req.clone())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("-> Spawning a mock http server...");

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    let times = AtomicI32::new(3);
    // Arrange the behaviour of the MockServer adding a Mock:
    // when it receives a GET request on '/hello' it will respond with a 200.
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(move |_req: &wiremock::Request| {
            let old = times.fetch_sub(1, SeqCst);
            if old < 1 {
                ResponseTemplate::new(200)
            } else {
                ResponseTemplate::new(500)
            }
        })
        .mount(&mock_server)
        .await;

    eprintln!("-> Creating an HTTP client with Tower layers...");
    let mut client = ServiceBuilder::new()
        // Make client compatible with the `tower-http` layers.
        .retry(SimpleRetry::new(
            ExponentialBackoff::builder().build_with_max_retries(10),
        ))
        // Set the request body type.
        .map_request_body(|body: http_body_util::Full<Bytes>| reqwest::Body::wrap(body))
        .layer(HttpClientLayer)
        .service(reqwest::Client::new())
        .map_err(anyhow::Error::msg)
        .boxed_clone();

    let response = client.get(format!("{mock_uri}/hello")).send().await?;
    anyhow::ensure!(response.status().is_success(), "response failed");

    Ok(())
}
