use http::header::{HeaderValue, USER_AGENT};
use pretty_assertions::assert_eq;
use tower_layer::Layer as _;
use tower_reqwest::set_header::SetRequestHeaderLayer;
use tower_service::Service as _;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();

    // Create a mock server that will respond to a GET request on `/test`.
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

    let user_agent_value = HeaderValue::from_static("My-Custom-User-Agent/1.0");
    let response = SetRequestHeaderLayer::overriding(USER_AGENT, user_agent_value)
        .layer(reqwest::Client::new())
        .call(reqwest::Request::new(
            reqwest::Method::GET,
            format!("{mock_uri}/test").parse()?,
        ))
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}
