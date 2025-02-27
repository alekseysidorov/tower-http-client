use super::BodyReader;

/// Extension trait for the [`http::Response`].
pub trait ResponseExt<T>: Sized {
    /// Consumes the response and returns a body reader wrapper.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use tower_http_client::{ResponseExt as _, ServiceExt as _};
    /// use tower_reqwest::HttpClientService;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     // Create a new client
    ///     let mut client = HttpClientService::new(reqwest::Client::new());
    ///     // Execute request by using this service.
    ///     let response = client.get("http://ip.jsontest.com").send().await?;
    ///
    ///     let text = response.body_reader().utf8().await?;
    ///     println!("{text}");
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    fn body_reader(self) -> BodyReader<T>;
}

impl<T> ResponseExt<T> for http::Response<T> {
    fn body_reader(self) -> BodyReader<T> {
        BodyReader::new(self.into_body())
    }
}
