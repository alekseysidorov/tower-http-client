use http::header::{HeaderValue, USER_AGENT};
use pretty_assertions::assert_eq;
use tower_layer::Layer as _;
use tower_reqwest::set_header::SetRequestHeaderLayer;
use tower_service::Service as _;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start a mock server.
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(move |request: &wiremock::Request| {
            eprintln!(
                "Received request with user agent {:?}",
                request.headers[USER_AGENT]
            );
            ResponseTemplate::new(200)
        })
        .mount(&mock_server)
        .await;
    // Create a new client with the `SetRequestHeaderLayer` layer.
    let user_agent_value = HeaderValue::from_static("My-Custom-User-Agent/1.0");
    let response = SetRequestHeaderLayer::overriding(USER_AGENT, user_agent_value)
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
