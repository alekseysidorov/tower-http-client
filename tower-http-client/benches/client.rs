use criterion::{criterion_group, criterion_main, Criterion};
use http::{header::USER_AGENT, HeaderName, HeaderValue};
use tokio::time::Instant;
use tower::{util::BoxCloneSyncService, ServiceBuilder};
use tower_http_client::ServiceExt as _;
use tower_reqwest::HttpClientLayer;

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
        let router =
            axum::Router::new().route("/hello", axum::routing::get(|| async { "Hello, World!" }));

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

fn bench(criterion: &mut Criterion) {
    benchmark_baseline(criterion);
    benchmark_single_middleware(criterion);
    benchmark_multiple_middlewares(criterion, 10);
    benchmark_multiple_middlewares(criterion, 100);
}

criterion_group!(benches, bench);
criterion_main!(benches);
