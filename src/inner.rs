use crate::{Encoding, Format, Request};
use hyper::header::{ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use hyper::http::request::Builder;
use hyper::Method;

#[derive(Clone, Debug)]
pub(crate) struct VkApiInner {
    pub(crate) encoding: Encoding,
    pub(crate) format: Format,
    pub(crate) access_token: String,
    pub(crate) version: String,
    pub(crate) domain: String,
}

impl From<&'_ VkApiInner> for Builder {
    fn from(inner: &VkApiInner) -> Self {
        Request::builder()
            .method(Method::POST)
            .header(
                ACCEPT_ENCODING,
                match inner.encoding {
                    #[cfg(feature = "compression_zstd")]
                    Encoding::Zstd => "zstd",
                    #[cfg(feature = "compression_gzip")]
                    Encoding::Gzip => "gzip",
                    Encoding::None => "identity",
                },
            )
            .header(
                ACCEPT,
                match inner.format {
                    #[cfg(feature = "encode_msgpack")]
                    Format::Msgpack => "application/x-msgpack",
                    #[cfg(feature = "encode_json")]
                    Format::Json => "application/json",
                    Format::None => "text/*",
                },
            )
            .header(AUTHORIZATION, format!("Bearer {}", inner.access_token))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
    }
}
