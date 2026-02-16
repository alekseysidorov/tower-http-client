use std::time::Duration;

use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use http::{HeaderName, HeaderValue, header::USER_AGENT};
use http_body_util::{BodyExt as _, combinators::BoxBody};
use tokio::time::Instant;
use tower::{BoxError, ServiceBuilder, util::BoxCloneSyncService};
use tower_http::ServiceBuilderExt;
use tower_http_client::{ResponseExt as _, ServiceExt as _};
use tower_reqwest::HttpClientLayer;

/// A body that can be cloned in order to be sent multiple times.
type CloneableBody = http_body_util::Full<Bytes>;
/// A type-erased HTTP client that is completely implementation-agnostic.
type HttpClient = BoxCloneSyncService<
    http::Request<CloneableBody>,
    http::Response<BoxBody<Bytes, BoxError>>,
    BoxError,
>;

#[derive(Debug, Clone)]
struct AddHeader {
    name: HeaderName,
    value: HeaderValue,
}

#[async_trait::async_trait]
impl reqwest_middleware::Middleware for AddHeader {
    async fn handle(
        &self,
        mut req: reqwest::Request,
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        req.headers_mut().insert(&self.name, self.value.clone());
        next.run(req, extensions).await
    }
}

fn tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

async fn create_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
        .await
        .expect("Failed to bind socket");

    let local_addr = listener.local_addr().expect("Failed to get local address");
    let handle = tokio::spawn(async move {
        let router = axum::Router::new()
            .route("/hello", axum::routing::get(|| async { "Hello, World!" }))
            .route(
                "/json",
                axum::routing::get(|| async {
                    axum::extract::Json(serde_json::json!({
                        "message": "Hello, World!",
                        "timestamp": "2023-04-01T12:00:00Z",
                        "id": 123,
                        "status": "active"
                    }))
                }),
            );

        axum::serve(listener, router)
            .await
            .expect("Failed to start server");
    });

    (local_addr.to_string(), handle)
}

fn bench_with_server<I, C, P, R>(c: &mut Criterion, name: &str, init_client: I, payload: P)
where
    I: FnOnce() -> C + Copy,
    P: Fn(String, C) -> R + Copy,
    R: futures_util::Future<Output = ()> + Send,
    C: Clone,
{
    c.bench_function(name, move |bencher| {
        bencher
            .to_async(tokio_runtime())
            .iter_custom(|iters| async move {
                let client = init_client();
                let (server_addr, server_handle) = create_server().await;

                // Check that the server is ready.
                let max_attempts = 10;
                for attempt in 0..=max_attempts {
                    match reqwest::get(format!("http://{server_addr}/hello")).await {
                        Ok(_) => break,

                        Err(err) if attempt == max_attempts => {
                            panic!("Server failed to start: {err}")
                        }
                        Err(_) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }

                let start = Instant::now();
                for _ in 0..iters {
                    payload(server_addr.clone(), client.clone()).await;
                }
                let elapsed = start.elapsed();

                drop(server_handle);
                elapsed
            });
    });
}

fn benchmark_baseline(criterion: &mut Criterion) {
    bench_with_server(
        criterion,
        "reqwest/baseline",
        reqwest::Client::new,
        |addr, client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
    bench_with_server(
        criterion,
        "tower-http-client/baseline",
        || {
            ServiceBuilder::new()
                .layer(HttpClientLayer)
                .service(reqwest::Client::new())
        },
        |addr, mut client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
    bench_with_server(
        criterion,
        "tower-http-client/boxed",
        || {
            ServiceBuilder::new()
                .layer_fn(BoxCloneSyncService::new)
                .layer(HttpClientLayer)
                .service(reqwest::Client::new())
        },
        |addr, mut client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
}

fn benchmark_single_middleware(criterion: &mut Criterion) {
    bench_with_server(
        criterion,
        "tower-http-client/set-header",
        || {
            ServiceBuilder::new()
                .map_request(|mut request: http::Request<reqwest::Body>| {
                    let header_value = HeaderValue::from_static("criterion");
                    request.headers_mut().insert(USER_AGENT, header_value);
                    request
                })
                .layer(HttpClientLayer)
                .service(reqwest::Client::new())
        },
        |addr, mut client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
    bench_with_server(
        criterion,
        "reqwest-middleware/set-header",
        || {
            reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
                .with(AddHeader {
                    name: USER_AGENT,
                    value: HeaderValue::from_static("criterion"),
                })
                .build()
        },
        |addr, client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
}

fn benchmark_multiple_middlewares(criterion: &mut Criterion, count: usize) {
    bench_with_server(
        criterion,
        &format!("reqwest/set-header/{count}"),
        reqwest::Client::new,
        |addr, client| async move {
            let mut request: reqwest::RequestBuilder = client.get(format!("http://{addr}/hello"));
            for i in 0..count {
                request = request.header(
                    format!("X-Header-{i}"),
                    HeaderValue::from_static("criterion"),
                );
            }
            request.send().await.expect("Failed to send request");
        },
    );
    bench_with_server(
        criterion,
        &format!("tower-http-client/set-header/{count}"),
        || {
            let mut service = ServiceBuilder::new()
                .layer_fn(BoxCloneSyncService::new)
                .layer(HttpClientLayer)
                .service(reqwest::Client::new());

            for i in 0..count {
                let header_name: HeaderName = format!("X-Header-{i}").parse().unwrap();
                service = ServiceBuilder::new()
                    .layer_fn(BoxCloneSyncService::new)
                    .map_request(move |mut request: http::Request<reqwest::Body>| {
                        let header_value = HeaderValue::from_static("criterion");
                        request
                            .headers_mut()
                            .insert(header_name.clone(), header_value);
                        request
                    })
                    .service(service);
            }

            service
        },
        |addr, mut client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
    bench_with_server(
        criterion,
        &format!("reqwest-middleware/set-header/{count}"),
        || {
            let mut builder = reqwest_middleware::ClientBuilder::new(reqwest::Client::new());

            for i in 0..count {
                builder = builder.with(AddHeader {
                    name: format!("X-Header-{i}").parse().unwrap(),
                    value: HeaderValue::from_static("criterion"),
                });
            }
            builder.build()
        },
        |addr, client| async move {
            client
                .get(format!("http://{addr}/hello"))
                .send()
                .await
                .expect("Failed to send request");
        },
    );
}

fn benchmark_json(criterion: &mut Criterion) {
    bench_with_server(
        criterion,
        "reqwest/json",
        reqwest::Client::new,
        |addr, client| async move {
            let response: reqwest::Response = client
                .get(format!("http://{addr}/json"))
                .send()
                .await
                .expect("Failed to send request");
            let _value: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        },
    );
    bench_with_server(
        criterion,
        "tower-http-client/json",
        || -> HttpClient {
            ServiceBuilder::new()
                .layer_fn(BoxCloneSyncService::new)
                .map_err(BoxError::from)
                .map_response_body(|body: reqwest::Body| body.map_err(BoxError::from).boxed())
                .map_request_body(|body: CloneableBody| reqwest::Body::wrap(body))
                .layer(HttpClientLayer)
                .service(reqwest::Client::new())
        },
        |addr, mut client| async move {
            let response = client
                .get(format!("http://{addr}/json"))
                .send()
                .await
                .expect("Failed to send request");
            let _value: serde_json::Value = response
                .body_reader()
                .json()
                .await
                .expect("Failed to parse JSON");
        },
    );
}

fn bench(criterion: &mut Criterion) {
    benchmark_baseline(criterion);
    benchmark_json(criterion);
    benchmark_single_middleware(criterion);
    benchmark_multiple_middlewares(criterion, 10);
    benchmark_multiple_middlewares(criterion, 100);
}

criterion_group!(benches, bench);
criterion_main!(benches);
