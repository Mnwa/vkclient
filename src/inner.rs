use crate::structs::Version;
use crate::vkapi::{Compression, Encoding};
use crate::{ResponseDeserialize, VkApiError, VkApiResult};
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::io::{BufReader, IoSliceMut, Read};

#[derive(Clone, Debug)]
pub struct VkApiInner {
    pub(crate) encoding: Compression,
    pub(crate) format: Encoding,
    pub(crate) access_token: String,
    pub(crate) version: Version,
    pub(crate) domain: String,
}

pub fn create_client() -> Client {
    Client::builder()
        .https_only(true)
        .use_rustls_tls()
        .build()
        .unwrap()
}

pub enum CompressReader<'a, R>
where
    R: 'static + Read,
{
    #[cfg(feature = "compression_zstd")]
    Zstd(zstd::Decoder<'a, BufReader<R>>),
    #[cfg(feature = "compression_gzip")]
    Gzip(Box<flate2::read::GzDecoder<BufReader<R>>>),
    Skip(BufReader<R>),
}

impl<'a, R> Read for CompressReader<'a, R>
where
    R: 'static + Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            CompressReader::Zstd(reader) => reader.read(buf),
            CompressReader::Gzip(reader) => reader.read(buf),
            CompressReader::Skip(reader) => reader.read(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        match self {
            CompressReader::Zstd(reader) => reader.read_exact(buf),
            CompressReader::Gzip(reader) => reader.read_exact(buf),
            CompressReader::Skip(reader) => reader.read_exact(buf),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        match self {
            CompressReader::Zstd(reader) => reader.read_to_end(buf),
            CompressReader::Gzip(reader) => reader.read_to_end(buf),
            CompressReader::Skip(reader) => reader.read_to_end(buf),
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        match self {
            CompressReader::Zstd(reader) => reader.read_to_string(buf),
            CompressReader::Gzip(reader) => reader.read_to_string(buf),
            CompressReader::Skip(reader) => reader.read_to_string(buf),
        }
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        match self {
            CompressReader::Zstd(reader) => reader.read_vectored(bufs),
            CompressReader::Gzip(reader) => reader.read_vectored(bufs),
            CompressReader::Skip(reader) => reader.read_vectored(bufs),
        }
    }
}

pub fn uncompress<B: Read + 'static>(
    encode: Option<HeaderValue>,
    body: B,
) -> VkApiResult<CompressReader<'static, B>> {
    match encode {
        #[cfg(feature = "compression_zstd")]
        Some(v) if v == "zstd" => Ok(CompressReader::Zstd(
            zstd::Decoder::new(body).map_err(VkApiError::IO)?,
        )),
        #[cfg(feature = "compression_gzip")]
        Some(v) if v == "gzip" => Ok(CompressReader::Gzip(Box::new(
            flate2::read::GzDecoder::new(BufReader::new(body)),
        ))),
        _ => Ok(CompressReader::Skip(BufReader::new(body))),
    }
}

pub fn decode<T: DeserializeOwned, B: Read>(
    format: &Option<HeaderValue>,
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
