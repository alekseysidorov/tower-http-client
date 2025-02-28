use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tower::{ServiceBuilder, ServiceExt as _};
use tower_http::ServiceBuilderExt as _;
use tower_http_client::{ResponseExt as _, ServiceExt as _};
use tower_reqwest::{into_reqwest_body, HttpClientLayer};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SomeInfo {
    name: String,
    age: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start a mock server.
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/test"))
        .respond_with(move |request: &wiremock::Request| {
            let info: SomeInfo = serde_urlencoded::from_bytes(request.body.as_ref()).unwrap();

            eprintln!("Received request with info {info:?}",);
            ResponseTemplate::new(200)
                .set_body_json(format!("I am {} and {} years old", info.name, info.age))
        })
        .mount(&mock_server)
        .await;
    let mock_server_uri = mock_server.uri();

    eprintln!("-> Creating an HTTP client with Tower layers...");
    let mut client = ServiceBuilder::new()
        // Set the request body type.
        .map_request_body(|body: http_body_util::Full<Bytes>| into_reqwest_body(body))
        .layer(HttpClientLayer)
        .service(reqwest::Client::new())
        .map_err(anyhow::Error::msg)
        .boxed_clone();

    let response = client
        .post(format!("{mock_server_uri}/test"))
        .form(&SomeInfo {
            name: "John".to_string(),
            age: 30,
        })?
        .send()
        .await?;

    // Check that the request was successful.
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.body_reader().json::<String>().await?,
        "I am John and 30 years old"
    );

    Ok(())
}
