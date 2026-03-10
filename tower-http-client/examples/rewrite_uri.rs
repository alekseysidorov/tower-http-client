use bytes::Bytes;
use http::Uri;
use tower::{ServiceBuilder, ServiceExt as _};
use tower_http::ServiceBuilderExt as _;
use tower_http_client::{
    ServiceExt as _,
    rewrite_uri::{RewriteUri, RewriteUriLayer},
};
use tower_reqwest::HttpClientLayer;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

/// Rewrites every request to target a fixed base URI, preserving the
/// original path and query.  Useful for pointing a client at staging vs
/// production without changing call sites.
#[derive(Clone)]
struct BaseUri {
    scheme: http::uri::Scheme,
    authority: http::uri::Authority,
}

impl BaseUri {
    /// Create a new `BaseUri` rewriter from the parts (scheme and authority) of the given URI.
    fn from_uri(uri: Uri) -> Result<Self, std::io::Error> {
        let parts = uri.into_parts();
        Ok(Self {
            scheme: parts
                .scheme
                .ok_or_else(|| std::io::Error::other("missing scheme"))?,
            authority: parts
                .authority
                .ok_or_else(|| std::io::Error::other("missing authority"))?,
        })
    }
}

impl RewriteUri for BaseUri {
    type Error = http::Error;

    fn rewrite_uri(&mut self, uri: &Uri) -> Result<Uri, Self::Error> {
        let pq = uri.path_and_query().map_or("/", |pq| pq.as_str());
        http::Uri::builder()
            .scheme(self.scheme.clone())
            .authority(self.authority.clone())
            .path_and_query(pq)
            .build()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (_mock_server, mock_server_uri) = create_mock_server().await;

    eprintln!("-> Creating an HTTP client with RewriteUri layer...");
    let mock_uri: Uri = mock_server_uri.parse()?;

    // Build the base HTTP client first, then wrap it with RewriteUriLayer.
    let base_client = ServiceBuilder::new()
        .map_request_body(|body: http_body_util::Full<Bytes>| reqwest::Body::wrap(body))
        .layer(HttpClientLayer)
        .service(reqwest::Client::new());

    let mut client = ServiceBuilder::new()
        // Rewrite every request URI to target the mock server base URI.
        .layer(RewriteUriLayer::new(BaseUri::from_uri(mock_uri)?))
        .map_err(anyhow::Error::msg)
        .service(base_client)
        .boxed_clone();

    eprintln!("-> Sending request with a relative URI (path only)...");
    let response: http::Response<_> = client.get("/hello").send().await?;

    assert_eq!(response.status(), 200);
    eprintln!("-> Response status: {}", response.status());

    Ok(())
}

async fn create_mock_server() -> (MockServer, String) {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;
    let mock_server_uri = mock_server.uri();
    (mock_server, mock_server_uri)
}
