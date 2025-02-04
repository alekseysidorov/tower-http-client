# tower-reqwest

[![tests](https://github.com/alekseysidorov/tower-http-client/actions/workflows/ci.yml/badge.svg)](https://github.com/alekseysidorov/tower-http-client/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tower-reqwest.svg)](https://crates.io/crates/tower-reqwest)
[![Documentation](https://docs.rs/tower-reqwest/badge.svg)](https://docs.rs/tower-reqwest)
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/tower-reqwest)](./LICENSE)

<!-- ANCHOR: description -->

This library provides adapters to use [reqwest] client with the [tower_http]
layers.

## Example

<!-- ANCHOR: example -->

```rust
use http::{header::USER_AGENT, HeaderValue};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::{Service, ServiceBuilder};
use tower_http::ServiceBuilderExt;
use tower_reqwest::HttpClientLayer;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    // Create a mock server that will respond to a GET request on `/test`.
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "value": "Hello world" })))
        .mount(&mock_server)
        .await;

    let mut client = ServiceBuilder::new()
        // Add some layers.
        .override_request_header(USER_AGENT, HeaderValue::from_static("tower-reqwest"))
        // Make client compatible with the `tower-http` layers.
        .layer(HttpClientLayer)
        .service(reqwest::Client::new());
    // Execute request by using this service.
    let response = client
        .call(
            http::request::Builder::new()
                .method(http::Method::GET)
                .uri(format!("{mock_uri}/hello"))
                .body(reqwest::Body::default())?,
        )
        .await?;

    let bytes = response.into_body().collect().await?.to_bytes();
    let value: Value = serde_json::from_slice(&bytes)?;
    println!("{value:#?}");

    Ok(())
}
```

<!-- ANCHOR_END: example -->

[reqwest]: https://github.com/seanmonstar/reqwest
[tower_http]: https://github.com/tower-rs/tower-http

<!-- ANCHOR_END: description -->
