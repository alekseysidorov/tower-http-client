//! Extensions for the `http::request::Builder`.

use private::Sealed;
use thiserror::Error;

/// Set body errors.
#[derive(Debug, Error)]
#[error(transparent)]
pub enum SetBodyError<S> {
    /// An error occurred while setting the body.
    Body(http::Error),
    /// An error occurred while encoding the body.
    Encode(S),
}

/// An error that can occur when setting a query string on a request.
#[cfg(feature = "query")]
#[cfg_attr(docsrs, doc(cfg(feature = "query")))]
#[derive(Debug, Error)]
#[error(transparent)]
pub enum SetQueryError {
    /// The resulting URI is invalid.
    InvalidUri(http::uri::InvalidUri),
    /// The resulting URI parts are invalid.
    InvalidUriParts(http::uri::InvalidUriParts),
    /// An error occurred while serializing the query parameters.
    Encode(serde_urlencoded::ser::Error),
}

/// Extension trait for the [`http::request::Builder`].
pub trait RequestBuilderExt: Sized + Sealed {
    /// Appends a typed header to this request.
    ///
    /// This function will append the provided header as a header to the
    /// internal [`http::HeaderMap`] being constructed.  Essentially this is
    /// equivalent to calling [`headers::HeaderMapExt::typed_insert`].
    #[must_use]
    #[cfg(feature = "typed-header")]
    #[cfg_attr(docsrs, doc(cfg(feature = "typed-header")))]
    fn typed_header<T>(self, header: T) -> Self
    where
        T: headers::Header;

    /// Sets a JSON body for this request.
    ///
    /// Additionally this method adds a `CONTENT_TYPE` header for JSON body.
    /// If you decide to override the request body, keep this in mind.
    ///
    /// # Errors
    ///
    /// If the given value's implementation of [`serde::Serialize`] decides to fail.
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    fn json<T: serde::Serialize + ?Sized>(
        self,
        value: &T,
    ) -> Result<http::Request<bytes::Bytes>, SetBodyError<serde_json::Error>>;

    /// Sets a form body for this request.
    ///
    /// Additionally this method adds a `CONTENT_TYPE` header for form body.
    /// If you decide to override the request body, keep this in mind.
    ///
    /// # Errors
    ///
    /// If the given value's implementation of [`serde::Serialize`] decides to fail.
    #[cfg(feature = "form")]
    #[cfg_attr(docsrs, doc(cfg(feature = "form")))]
    fn form<T: serde::Serialize + ?Sized>(
        self,
        form: &T,
    ) -> Result<http::Request<bytes::Bytes>, SetBodyError<serde_urlencoded::ser::Error>>;

    /// Sets the query string of the URL.
    ///
    /// Serializes the given value into a query string using [`serde_urlencoded`]
    /// and replaces the existing query string of the URL entirely. Any previously
    /// set query parameters are discarded.
    ///
    /// ```text
    /// // "existing=1" is lost
    /// .uri("http://example.com/path?existing=1")
    /// .query(&[("key", "value")])
    /// // => "http://example.com/path?key=value"
    /// ```
    ///
    /// Duplicate keys are preserved as-is:
    /// `.query(&[("foo", "a"), ("foo", "b")])` produces `"foo=a&foo=b"`.
    ///
    /// # Note
    ///
    /// This method does not support a single key-value tuple directly.
    /// Use a slice like `.query(&[("key", "val")])` instead.
    /// Structs and maps that serialize into key-value pairs are also supported.
    ///
    /// # Errors
    ///
    /// Returns a [`SetQueryError`] if the provided value cannot be serialized
    /// into a query string, or if the resulting URI is invalid.
    #[cfg(feature = "query")]
    #[cfg_attr(docsrs, doc(cfg(feature = "query")))]
    fn query<T: serde::Serialize + ?Sized>(self, query: &T) -> Result<Self, SetQueryError>;
}

impl RequestBuilderExt for http::request::Builder {
    #[cfg(feature = "typed-header")]
    #[cfg_attr(docsrs, doc(cfg(feature = "typed-header")))]
    fn typed_header<T>(mut self, header: T) -> Self
    where
        T: headers::Header,
    {
        use headers::HeaderMapExt;

        if let Some(headers) = self.headers_mut() {
            headers.typed_insert(header);
        }
        self
    }

    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    fn json<T: serde::Serialize + ?Sized>(
        mut self,
        value: &T,
    ) -> Result<http::Request<bytes::Bytes>, SetBodyError<serde_json::Error>> {
        use http::{HeaderValue, header::CONTENT_TYPE};

        if let Some(headers) = self.headers_mut() {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }

        let bytes = bytes::Bytes::from(serde_json::to_vec(value).map_err(SetBodyError::Encode)?);
        self.body(bytes).map_err(SetBodyError::Body)
    }

    #[cfg(feature = "form")]
    #[cfg_attr(docsrs, doc(cfg(feature = "form")))]
    fn form<T: serde::Serialize + ?Sized>(
        mut self,
        form: &T,
    ) -> Result<http::Request<bytes::Bytes>, SetBodyError<serde_urlencoded::ser::Error>> {
        use http::{HeaderValue, header::CONTENT_TYPE};

        let string = serde_urlencoded::to_string(form).map_err(SetBodyError::Encode)?;
        if let Some(headers) = self.headers_mut() {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }

        self.body(bytes::Bytes::from(string))
            .map_err(SetBodyError::Body)
    }

    #[cfg(feature = "query")]
    #[cfg_attr(docsrs, doc(cfg(feature = "query")))]
    fn query<T: serde::Serialize + ?Sized>(self, query: &T) -> Result<Self, SetQueryError> {
        use http::uri::PathAndQuery;

        let mut parts = self.uri_ref().cloned().unwrap_or_default().into_parts();
        let new_path_and_query = {
            // If the URI doesn't have a path, we need to set it to "/" so that the query string can be appended correctly.
            let path = parts
                .path_and_query
                .as_ref()
                .map_or_else(|| "/", |pq| pq.path());

            let query_string = serde_urlencoded::to_string(query).map_err(SetQueryError::Encode)?;
            let pq_str = [path, "?", &query_string].concat();
            PathAndQuery::try_from(pq_str).map_err(SetQueryError::InvalidUri)?
        };

        parts.path_and_query = Some(new_path_and_query);
        let uri = http::Uri::from_parts(parts).map_err(SetQueryError::InvalidUriParts)?;

        Ok(self.uri(uri))
    }
}

mod private {
    pub trait Sealed {}

    impl Sealed for http::request::Builder {}
}

#[cfg(all(test, feature = "query"))]
mod query_tests {
    use pretty_assertions::assert_eq;
    use tower_http::BoxError;

    use super::*;

    #[test]
    fn test_query_basic() -> Result<(), BoxError> {
        let request = http::Request::builder()
            .uri("http://example.com/path")
            .query(&[("key", "value")])?
            .body(())?;

        assert_eq!(request.uri().query(), Some("key=value"));
        Ok(())
    }

    #[test]
    fn test_query_without_uri() -> Result<(), BoxError> {
        let request = http::Request::builder()
            .query(&[("key", "value")])?
            .body(())?;

        assert_eq!(request.uri().query(), Some("key=value"));
        Ok(())
    }

    #[test]
    fn test_query_overwrites_existing() -> Result<(), BoxError> {
        let request = http::Request::builder()
            .uri("http://example.com/path?existing=1")
            .query(&[("key", "value")])?
            .body(())?;

        // "existing=1" must be gone
        assert_eq!(request.uri().query(), Some("key=value"));
        Ok(())
    }

    #[test]
    fn test_query_last_call_wins() -> Result<(), BoxError> {
        let request = http::Request::builder()
            .uri("http://example.com/path")
            .query(&[("first", "1")])?
            .query(&[("second", "2")])?
            .body(())?;

        // Only the last call survives
        assert_eq!(request.uri().query(), Some("second=2"));
        Ok(())
    }

    #[test]
    fn test_query_duplicate_keys() -> Result<(), BoxError> {
        let request = http::Request::builder()
            .uri("http://example.com/path")
            .query(&[("foo", "a"), ("foo", "b")])?
            .body(())?;

        assert_eq!(request.uri().query(), Some("foo=a&foo=b"));
        Ok(())
    }

    #[test]
    fn test_query_struct() -> Result<(), BoxError> {
        #[derive(serde::Serialize)]
        struct Params {
            page: u32,
            limit: u32,
        }

        let request = http::Request::builder()
            .uri("http://example.com/path")
            .query(&Params { page: 2, limit: 10 })?
            .body(())?;

        assert_eq!(request.uri().query(), Some("page=2&limit=10"));
        Ok(())
    }

    #[test]
    fn test_query_special_characters_are_encoded() -> Result<(), BoxError> {
        let request = http::Request::builder()
            .uri("http://example.com/path")
            .query(&[("key", "hello world")])?
            .body(())?;

        assert_eq!(request.uri().query(), Some("key=hello+world"));
        Ok(())
    }

    #[test]
    fn test_query_encode_error() {
        // Scalars (e.g. integers) are not supported by serde_urlencoded
        let error = http::Request::builder().query(&42).unwrap_err();

        assert!(matches!(error, SetQueryError::Encode(_)));
    }
}
