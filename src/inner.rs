use crate::vkapi::{Compression, Encoding};
use hyper::header::{ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use hyper::http::request::Builder;
use hyper::{Method, Request};

#[derive(Clone, Debug)]
pub(crate) struct VkApiInner {
    pub(crate) encoding: Compression,
    pub(crate) format: Encoding,
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
                    Compression::Zstd => "zstd",
                    #[cfg(feature = "compression_gzip")]
                    Compression::Gzip => "gzip",
                    Compression::None => "identity",
                },
            )
            .header(
                ACCEPT,
                match inner.format {
                    #[cfg(feature = "encode_msgpack")]
                    Encoding::Msgpack => "application/x-msgpack",
                    #[cfg(feature = "encode_json")]
                    Encoding::Json => "application/json",
                    Encoding::None => "text/*",
                },
            )
            .header(AUTHORIZATION, format!("Bearer {}", inner.access_token))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
    }
}
