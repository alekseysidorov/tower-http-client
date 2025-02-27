//! Extensions for the `http::request::Builder`.

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

/// Extension trait for the [`http::request::Builder`].
pub trait RequestBuilderExt: Sized {
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
    ) -> Result<http::Request<String>, SetBodyError<serde_urlencoded::ser::Error>>;
}

impl RequestBuilderExt for http::request::Builder {
    #[cfg(feature = "json")]
    #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    fn json<T: serde::Serialize + ?Sized>(
        mut self,
        value: &T,
    ) -> Result<http::Request<bytes::Bytes>, SetBodyError<serde_json::Error>> {
        use http::{header::CONTENT_TYPE, HeaderValue};

        if let Some(headers) = self.headers_mut() {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        };

        let bytes = bytes::Bytes::from(serde_json::to_vec(value).map_err(SetBodyError::Encode)?);
        self.body(bytes).map_err(SetBodyError::Body)
    }

    #[cfg(feature = "form")]
    #[cfg_attr(docsrs, doc(cfg(feature = "form")))]
    fn form<T: serde::Serialize + ?Sized>(
        mut self,
        form: &T,
    ) -> Result<http::Request<String>, SetBodyError<serde_urlencoded::ser::Error>> {
        use http::{header::CONTENT_TYPE, HeaderValue};

        let string = serde_urlencoded::to_string(form).map_err(SetBodyError::Encode)?;
        if let Some(headers) = self.headers_mut() {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }
        self.body(string).map_err(SetBodyError::Body)
    }
}
