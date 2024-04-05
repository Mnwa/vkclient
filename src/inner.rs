use crate::structs::Version;
use crate::vkapi::{Compression, Encoding};
use crate::{ResponseDeserialize, VkApiError, VkApiResult};
use reqwest::header::HeaderValue;
use reqwest::Client;
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

pub(crate) fn create_client() -> Client {
    Client::builder()
        .https_only(true)
        .use_rustls_tls()
        .build()
        .unwrap()
}

pub(crate) fn uncompress<B: Read + 'static>(
    encode: Option<HeaderValue>,
    body: B,
) -> VkApiResult<Box<dyn Read>> {
    match encode {
        #[cfg(feature = "compression_zstd")]
        Some(v) if v == "zstd" => Ok(Box::new(zstd::Decoder::new(body).map_err(VkApiError::IO)?)),
        #[cfg(feature = "compression_gzip")]
        Some(v) if v == "gzip" => Ok(Box::new(flate2::read::GzDecoder::new(body))),
        _ => Ok(Box::new(body)),
    }
}

pub(crate) fn decode<T: DeserializeOwned, B: Read>(
    format: Option<HeaderValue>,
    body: B,
) -> VkApiResult<T> {
    match format.as_ref().and_then(|f| f.to_str().ok()) {
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
