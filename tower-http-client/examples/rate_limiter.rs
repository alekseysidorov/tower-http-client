use std::time::Duration;

use http::{Request, Response};
use reqwest::Body;
use tower::{ServiceBuilder, ServiceExt as _};
use tower_http_client::ServiceExt as _;
use tower_reqwest::HttpClientLayer;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

type HttpClient = tower::util::BoxCloneService<Request<Body>, Response<Body>, anyhow::Error>;

#[derive(Clone)]
struct State {
    host: String,
    client: HttpClient,
}

impl State {
    async fn get_hello(&mut self) -> anyhow::Result<()> {
        let response = self
            .client
            .get(format!("{}/hello", self.host))
            .send()
            .await?;

        anyhow::ensure!(response.status().is_success(), "response failed");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("-> Spawning a mock http server...");

    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    // Arrange the behaviour of the MockServer adding a Mock:
    // when it receives a GET request on '/hello' it will respond with a 200.
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        // Mounting the mock on the mock server - it's now effective!
        .mount(&mock_server)
        .await;

    eprintln!("-> Creating an HTTP client with Tower layers...");

    let state = State {
        host: mock_uri,
        client: ServiceBuilder::new()
            // Add some layers.
            .buffer(64)
            .rate_limit(2, Duration::from_secs(1))
            .concurrency_limit(5)
            // Make client compatible with the `tower-http` layers.
            .layer(HttpClientLayer)
            .service(reqwest::Client::new())
            .map_err(anyhow::Error::msg)
            .boxed_clone(),
    };

    eprintln!("-> Sending concurrent requests...");

    let tasks = (0..5).map({
        |i| {
            let state = state.clone();
            tokio::spawn(async move {
                let mut state = state.clone();
                for j in 0..5 {
                    state.get_hello().await?;
                    eprintln!("[task {i}]: Request #{j} completed successfully!");
                }

                anyhow::Ok(())
            })
        }
    });

    let results = futures_util::future::join_all(tasks).await;
    for result in results {
        result??;
    }

    Ok(())
}
