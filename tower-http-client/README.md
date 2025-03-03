# tower-http-client

[![tests](https://github.com/alekseysidorov/tower-http-client/actions/workflows/ci.yml/badge.svg)](https://github.com/alekseysidorov/tower-http-client/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tower-http-client.svg)](https://crates.io/crates/tower-http-client)
[![Documentation](https://docs.rs/tower-http-client/badge.svg)](https://docs.rs/tower-http-client)
[![MIT/Apache-2 licensed](https://img.shields.io/crates/l/tower-http-client)](./LICENSE)

<!-- ANCHOR: description -->

This library provides middlewares and various utilities for HTTP-clients.

Thus, it extends the [`tower_http`] functionality for creating HTTP clients
using [`tower`] middlewares.

At the moment, the de facto standard client library is [`reqwest`], which is
poorly compatible with the [`tower`] services, but thanks to the
[`tower_reqwest`] crate, it can be used with the any [`tower_http`] layers.

The first goal of the project is to create a more flexible and extensible
alternative for [`reqwest_middleware`].

## Examples

Simple client usage with layers from the [`tower_http`].

<!-- ANCHOR: example -->

```rust
use http::{header::USER_AGENT, HeaderValue};
use tower::{ServiceBuilder, ServiceExt};
use tower_http::ServiceBuilderExt;
use tower_http_client::{ServiceExt as _, ResponseExt as _};
use tower_reqwest::HttpClientLayer;

/// Implementation agnostic HTTP client.
type HttpClient = tower::util::BoxCloneService<
    http::Request<reqwest::Body>,
    http::Response<reqwest::Body>,
    anyhow::Error,
>;

/// Creates HTTP client with Tower layers on top of the given client.
fn make_client(client: reqwest::Client) -> HttpClient {
    ServiceBuilder::new()
        // Add some layers.
        .override_request_header(USER_AGENT, HeaderValue::from_static("tower-http-client"))
        // Make client compatible with the `tower-http` layers.
        .layer(HttpClientLayer)
        .service(client)
        .map_err(anyhow::Error::from)
        .boxed_clone()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a new client.
    let mut client = make_client(reqwest::Client::new());
    // Execute request by using this service.
    let response = client
        .get("http://ip.jsontest.com")
        .send()
        .await?;

    let text = response.body_reader().utf8().await?;
    println!("{text}");

    Ok(())
}
```

<!-- ANCHOR_END: example -->

[`tower_reqwest`]: https://docs.rs/tower-reqwest
[`reqwest_middleware`]: https://docs.rs/reqwest-middleware
[`reqwest`]: https://github.com/seanmonstar/reqwest
[`tower_http`]: https://github.com/tower-rs/tower-http
[`tower`]: https://github.com/tower-rs/tower

<!-- ANCHOR_END: description -->
