use http::header::AUTHORIZATION;
use pretty_assertions::assert_eq;
use tower_layer::Layer as _;
use tower_reqwest::auth::AddAuthorizationLayer;
use tower_service::Service as _;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start a mock server.
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(move |request: &wiremock::Request| {
            eprintln!(
                "Received request with authorization header: {:?}",
                request.headers[AUTHORIZATION]
            );
            ResponseTemplate::new(200)
        })
        .mount(&mock_server)
        .await;
    // Create a new client with the `AddAuthorizationLayer` layer.
    let response = AddAuthorizationLayer::bearer("abacaba")?
        .layer(reqwest::Client::new())
        // Send a request to the mock server.
        .call(reqwest::Request::new(
            reqwest::Method::GET,
            format!("{}/test", mock_server.uri()).parse()?,
        ))
        .await?;
    // Check that the request was successful.
    assert_eq!(response.status(), 200);

    Ok(())
}
