//! Convenient wrapper for reading [`Body`] content.

use std::string::FromUtf8Error;

use bytes::{Buf, Bytes};
use http_body::Body;
use http_body_util::BodyExt;
use thiserror::Error;

/// Convenient wrapper for reading [`Body`] content.
///
/// It is useful in the most common response body reading cases.
#[derive(Debug, Clone)]
pub struct BodyReader<B>(B);

/// Read body errors.
#[derive(Debug, Error)]
#[error(transparent)]
pub enum BodyReaderError<E, D> {
    /// An error occurred while reading the body.
    Read(E),
    /// An error occurred while decoding the body content.
    Decode(D),
}

impl<B> BodyReader<B> {
    /// Creates a new reader instance for the given body.
    pub const fn new(body: B) -> Self {
        Self(body)
    }

    /// Reads the full response body as [`Bytes`].
    ///
    /// # Example
    ///
    /// ```
    /// use http_body_util::Full;
    /// use tower_http_client::client::BodyReader;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let body = Full::new("Hello world".as_bytes());
    ///     let content = BodyReader::new(body).bytes().await?;
    ///
    ///     assert_eq!(content, "Hello world");
    ///     Ok(())
    /// }
    /// ```
    pub async fn bytes(self) -> Result<Bytes, B::Error>
    where
        B: Body,
        B::Data: Buf,
    {
        let body_bytes = self.0.collect().await?.to_bytes();
        Ok(body_bytes)
    }

    /// Reads the full response text.
    ///
    /// # Note
    ///
    /// The method will only attempt to decode the response as `UTF-8`, regardless of the
    /// `Content-Type` header.
    ///
    /// # Example
    ///
    /// ```
    /// use http_body_util::Full;
    /// use tower_http_client::client::BodyReader;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let body = Full::new("Hello world".as_bytes());
    ///     let content = BodyReader::new(body).utf8().await?;
    ///
    ///     assert_eq!(content, "Hello world");
    ///     Ok(())
    /// }
    /// ```
    pub async fn utf8(self) -> Result<String, BodyReaderError<B::Error, FromUtf8Error>>
    where
        B: Body,
        B::Data: Buf,
    {
        let bytes = self.bytes().await.map_err(BodyReaderError::Read)?;
        String::from_utf8(bytes.into()).map_err(BodyReaderError::Decode)
    }

    /// Deserializes the response body as JSON.
    ///
    /// # Examples
    ///
    /// ```
    /// use http_body_util::Full;
    /// use serde_json::{json, Value};
    /// use tower_http_client::client::BodyReader;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let data = serde_json::to_vec(&json!({ "id": 1234 })).unwrap();
    ///     let body = Full::new(data.as_ref());
    ///     let content: Value = BodyReader::new(body).json().await?;
    ///
    ///     assert_eq!(content["id"], 1234);
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub async fn json<T>(self) -> Result<T, BodyReaderError<B::Error, serde_json::Error>>
    where
        T: serde::de::DeserializeOwned,
        B: Body,
        B::Data: Buf,
    {
        let bytes = self.bytes().await.map_err(BodyReaderError::Read)?;
        serde_json::from_slice(&bytes).map_err(BodyReaderError::Decode)
    }
}
