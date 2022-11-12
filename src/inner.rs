use crate::structs::Version;
use crate::vkapi::{Compression, Encoding};
use crate::{ResponseDeserialize, VkApiError};
use hyper::client::HttpConnector;
use hyper::header::{HeaderValue, ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE};
use hyper::http::request::Builder;
use hyper::{Client, Method, Request};
use hyper_rustls::HttpsConnector;
use serde::de::DeserializeOwned;
use std::io::Read;

#[derive(Clone, Debug)]
pub(crate) struct VkApiInner {
    pub(crate) encoding: Compression,
    pub(crate) format: Encoding,
    pub(crate) access_token: String,
    pub(crate) version: Version,
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

pub(crate) fn create_client<B>() -> Client<HttpsConnector<HttpConnector>, B> {
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http2()
        .build();

    Client::builder().http2_only(true).build(https)
}

pub(crate) fn uncompress<B: Read + 'static>(
    encode: Option<&HeaderValue>,
    body: B,
) -> Result<Box<dyn Read>, VkApiError> {
    match encode {
        #[cfg(feature = "compression_zstd")]
        Some(v) if v == "zstd" => Ok(Box::new(zstd::Decoder::new(body).map_err(VkApiError::IO)?)),
        #[cfg(feature = "compression_gzip")]
        Some(v) if v == "gzip" => Ok(Box::new(flate2::read::GzDecoder::new(body))),
        _ => Ok(Box::new(body)),
    }
}

pub(crate) fn decode<T: DeserializeOwned, B: Read>(
    format: Option<&HeaderValue>,
    body: B,
) -> Result<T, VkApiError> {
    match format.and_then(|f| f.to_str().ok()) {
        #[cfg(feature = "encode_json")]
        Some(v) if v.starts_with("application/json") => serde_json::from_reader::<B, T>(body)
            .map_err(|e| VkApiError::ResponseDeserialize(ResponseDeserialize::Json(e))),
        #[cfg(feature = "encode_msgpack")]
        Some(v) if v.starts_with("application/x-msgpack") => {
            rmp_serde::decode::from_read::<B, T>(body)
                .map_err(|e| VkApiError::ResponseDeserialize(ResponseDeserialize::MsgPack(e)))
        }
        _ => Err(VkApiError::ResponseDeserialize(
            ResponseDeserialize::BadEncoding,
        )),
    }
}
