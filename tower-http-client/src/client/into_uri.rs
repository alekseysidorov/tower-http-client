use http::{Uri, uri};
use private::Sealed;

/// A helper trait to try to convert some types into `Uri`.
///
/// This trait is sealed and implemented only for the most suitable types.
///
/// Unlike the similar trait in the Reqwest, this one describes a type's representation
/// that implements [`TryInto<Uri>`]. This approach can pass third-party types  like [`url::Url`]
/// directly to the [`http::request::Builder::uri`] without any wrappers.
pub trait IntoUri: Sealed {
    /// Which kind of value should be converted to the Uri via [`TryInto<Uri>`]
    type TryInto;
    /// Converts this value into the input type for the [`TryInto<Uri>`] conversion.
    fn into_uri(self) -> Self::TryInto;
}

impl IntoUri for &Uri {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for Uri {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for String {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for &String {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for &str {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl<'a> IntoUri for &'a Vec<u8> {
    type TryInto = &'a [u8];

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for Vec<u8> {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for &[u8] {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for uri::Parts {
    type TryInto = Self;

    fn into_uri(self) -> Self::TryInto {
        self
    }
}

impl IntoUri for url::Url {
    type TryInto = String;

    fn into_uri(self) -> Self::TryInto {
        self.into()
    }
}

impl<'a> IntoUri for &'a url::Url {
    type TryInto = &'a str;

    fn into_uri(self) -> Self::TryInto {
        self.as_str()
    }
}

mod private {
    use http::{Uri, uri};
    use url::Url;

    pub trait Sealed {}

    impl Sealed for uri::Parts {}
    impl Sealed for Uri {}
    impl Sealed for &Uri {}

    impl Sealed for String {}
    impl Sealed for &String {}
    impl Sealed for &str {}

    impl Sealed for Vec<u8> {}
    impl Sealed for &Vec<u8> {}
    impl Sealed for &[u8] {}

    impl Sealed for Url {}
    impl Sealed for &Url {}
}

#[cfg(test)]
mod tests {
    use http::Uri;
    use url::Url;

    use super::IntoUri as _;

    #[test]
    fn test_url_to_uri_smoke() {
        let example =
            "abc://username:password@example.com:123/path/data?key=value&key2=value2#fragid1";

        let url = Url::parse(example).unwrap();
        let expected_uri = Uri::from_static(example);

        let actual_uri: Uri = url.into_uri().parse().expect("failed to convert url");
        assert_eq!(actual_uri, expected_uri);
    }
}
