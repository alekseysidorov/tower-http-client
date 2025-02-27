//! Extensions for Tower services that provide HTTP clients implementation.

pub use self::{
    body_reader::BodyReader,
    client_request::{ClientRequest, ClientRequestBuilder},
    into_uri::IntoUri,
    request_ext::RequestBuilderExt,
    response_ext::ResponseExt,
    service_ext::ServiceExt,
};

pub mod body_reader;
pub mod request_ext;

mod client_request;
mod into_uri;
mod response_ext;
mod service_ext;
