use bytes::Bytes;
use headers::{HeaderMapExt, UserAgent};
use http_body_util::{combinators::BoxBody, BodyExt as _};
use serde::Deserialize;
use tower::{util::BoxCloneSyncService, BoxError, Service, ServiceBuilder};
use tower_http::ServiceBuilderExt;
use tower_http_client::{adapters::reqwest::into_reqwest_body, ResponseExt as _, ServiceExt};
use tower_reqwest::HttpClientLayer;

/// A body that can be cloned in order to be sent multiple times.
type CloneableBody = http_body_util::Full<Bytes>;
/// A type-erased HTTP client that is completely implementation-agnostic.
type HttpClient = BoxCloneSyncService<
    http::Request<CloneableBody>,
    http::Response<BoxBody<Bytes, BoxError>>,
    BoxError,
>;

/// Convert a `reqwest::Client` into an implementation-agnostic opaque client.
fn into_opaque_http_client(
    client: reqwest::Client,
) -> impl Service<
    http::Request<CloneableBody>,
    Response = http::Response<BoxBody<Bytes, BoxError>>,
    Error = BoxError,
    Future = impl Send,
> + Send
       + Clone {
    ServiceBuilder::new()
        .map_err(BoxError::from)
        .map_response_body(|body: reqwest::Body| body.map_err(BoxError::from).boxed())
        .map_request_body(|body: CloneableBody| into_reqwest_body(body))
        .layer(HttpClientLayer)
        .service(client)
}

#[derive(Debug, Deserialize)]
struct IpInfo {
    ip: String,
    country: String,
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    eprintln!("-> Creating an HTTP client with Tower layers...");
    // First of all, we create an opaque client using the reqwest client.
    let opaque_client = into_opaque_http_client(reqwest::Client::new());
    // Secondary we add some middleware to the client and then box it.
    let mut client: HttpClient = ServiceBuilder::new()
        .layer_fn(BoxCloneSyncService::new)
        .map_request(|mut request: http::Request<_>| {
            request
                .headers_mut()
                .typed_insert(UserAgent::from_static("tower-http-client"));
            request
        })
        .service(opaque_client);

    // Finally, we can use the boxed client to send requests.
    eprintln!("-> Getting IP information...");
    let response = client.get("http://api.myip.com").send().await?;
    let info = response.body_reader().json::<IpInfo>().await?;

    eprintln!("-> Got information:");
    eprintln!("   IP address: {}", info.ip);
    eprintln!("   Country: {}", info.country);

    Ok(())
}
