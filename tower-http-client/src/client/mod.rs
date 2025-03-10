//! Utilities to turn Tower service into the useful HTTP client.

pub use self::{
    body_reader::{BodyReader, ResponseExt},
    client_request::{ClientRequest, ClientRequestBuilder},
    into_uri::IntoUri,
    request_ext::RequestBuilderExt,
    service_ext::ServiceExt,
};

pub mod request_ext;
pub use http_body_reader as body_reader;

mod client_request;
mod into_uri;
mod service_ext;
