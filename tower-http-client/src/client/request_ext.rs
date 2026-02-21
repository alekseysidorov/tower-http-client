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

/// Extension trait for the [`http::request::Builder`].
pub trait RequestBuilderExt: Sized + Sealed {
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
}

impl RequestBuilderExt for http::request::Builder {
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
}

mod private {
    pub trait Sealed {}

    impl Sealed for http::request::Builder {}
}
