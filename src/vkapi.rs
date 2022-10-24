use crate::inner::VkApiInner;
use crate::structs::Version;
use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper::header::{CONTENT_ENCODING, CONTENT_TYPE};
use hyper::http::request::Builder;
use hyper::http::HeaderValue;
use hyper::{Body, Client};
use hyper_rustls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::Read;

/// # Base VK API client realisation.
/// This client supports zstd compression and msgpack format of VK API. It's works with http2 only connections.
///
/// ## Usage
/// ```rust
/// use vkclient::VkApi;
/// let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
/// ```
///
/// ## Features
/// * compression_zstd - enabled by default. Adds zstd compression support;
/// * compression_gzip - enabled by default. Adds gzip compression support;
/// * encode_json - enabled by default. Adds json encoding support;
/// * encode_msgpack - enabled by default. Adds msgpack encoding support;
#[derive(Debug, Clone)]
pub struct VkApi {
    inner: VkApiInner,
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl VkApi {
    pub(crate) fn from_inner(inner: VkApiInner) -> Self {
        let client = create_client();

        Self { inner, client }
    }

    /// Send request to VK API. See list of [VK API methods](https://dev.vk.com/method).
    /// ```rust
    /// use vkclient::{VkApi, VkApiError, List};
    /// use serde::{Deserialize, Serialize};
    ///
    /// async fn get_users_info(client: &VkApi) -> Result<Vec<UsersGetResponse>, VkApiError> {
    ///     client.send_request("users.get", UsersGetRequest {
    ///         user_ids: List(vec![1,2]),
    ///         fields: List(vec!["id", "sex"]),
    ///    }).await
    /// }
    ///
    /// #[derive(Serialize)]
    /// struct UsersGetRequest<'a> {
    ///     user_ids: List<Vec<usize>>,
    ///     fields: List<Vec<&'a str>>,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct UsersGetResponse {
    ///     id: i64,
    ///     first_name: String,
    ///     last_name: String,
    ///     sex: u8,
    /// }
    ///
    ///
    /// ```
    pub async fn send_request<T, B, M>(&self, method: M, body: B) -> Result<T, VkApiError>
    where
        T: DeserializeOwned,
        B: Serialize,
        M: AsRef<str>,
    {
        #[cfg(feature = "encode_msgpack")]
        let url = if matches!(self.inner.format, Encoding::Msgpack) {
            format!(
                "https://{}/method/{}.msgpack",
                self.inner.domain,
                method.as_ref()
            )
        } else {
            format!("https://{}/method/{}", self.inner.domain, method.as_ref())
        };

        #[cfg(not(feature = "encode_msgpack"))]
        let url = format!("https://{}/method/{}", self.inner.domain, method.as_ref());

        let body = serde_urlencoded::to_string(VkApiBody {
            v: self.inner.version,
            body,
        })
        .map_err(VkApiError::RequestSerialize)?;

        let request = Builder::from(&self.inner)
            .uri(url)
            .body(hyper::Body::from(body))
            .map_err(VkApiError::Http)?;

        let response = self
            .client
            .request(request)
            .await
            .map_err(VkApiError::Request)?;

        let (parts, body) = response.into_parts();

        let body = hyper::body::aggregate(body)
            .await
            .map_err(VkApiError::Request)?;

        let resp = decode::<Response<T>, _>(
            parts.headers.get(CONTENT_TYPE),
            uncompress(parts.headers.get(CONTENT_ENCODING), body.reader())?,
        )?;

        match resp {
            Response::Success { response } => Ok(response),
            Response::Error { error } => Err(VkApiError::Vk(error)),
        }
    }
}

/// Vk Api errors.
/// VkApiError::Vk - is an error of buisness logic, like expired token or incorrect request params
/// Other errors is about things around your request, like a serialization/deserialization or network errors.
#[derive(Debug)]
pub enum VkApiError {
    Http(hyper::http::Error),
    Request(hyper::Error),
    RequestSerialize(serde_urlencoded::ser::Error),
    ResponseDeserialize(ResponseDeserialize),
    Vk(VkError),
    IO(std::io::Error),
}

impl Display for VkApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VkApiError::IO(e) => Display::fmt(e, f),
            VkApiError::Http(e) => Display::fmt(e, f),
            VkApiError::Request(e) => Display::fmt(e, f),
            VkApiError::ResponseDeserialize(e) => Display::fmt(e, f),
            VkApiError::Vk(e) => Display::fmt(e, f),
            VkApiError::RequestSerialize(e) => Display::fmt(e, f),
        }
    }
}

impl Error for VkApiError {}

#[derive(Debug)]
pub enum ResponseDeserialize {
    #[cfg(feature = "encode_json")]
    Json(serde_json::Error),
    #[cfg(feature = "encode_msgpack")]
    MsgPack(rmp_serde::decode::Error),
    BadEncoding,
}

impl Display for ResponseDeserialize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "encode_json")]
            ResponseDeserialize::Json(e) => Display::fmt(e, f),
            #[cfg(feature = "encode_msgpack")]
            ResponseDeserialize::MsgPack(e) => Display::fmt(e, f),
            ResponseDeserialize::BadEncoding => {
                write!(f, "vk api bad encoding or compression returned")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum Response<T> {
    Success { response: T },
    Error { error: VkError },
}

/// VK Backend business logic errors.
/// [More info about codes](https://dev.vk.com/reference/errors).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VkError {
    error_code: i16,
    error_msg: String,
}

impl Display for VkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "vk api error occurred. Code: {}, message: {}",
            self.error_code, self.error_msg
        )
    }
}

#[derive(Debug, Clone, Serialize)]
struct VkApiBody<T> {
    v: Version,
    #[serde(flatten)]
    body: T,
}

impl Error for VkError {}

#[derive(Clone, Copy, Debug)]
pub enum Compression {
    #[cfg(feature = "compression_zstd")]
    Zstd,
    #[cfg(feature = "compression_gzip")]
    Gzip,
    None,
}

#[derive(Clone, Copy, Debug)]
pub enum Encoding {
    #[cfg(feature = "encode_msgpack")]
    Msgpack,
    #[cfg(feature = "encode_json")]
    Json,
    None,
}

fn create_client() -> Client<HttpsConnector<HttpConnector>, Body> {
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http2()
        .build();

    Client::builder().http2_only(true).build(https)
}

fn uncompress<B: Read + 'static>(
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

fn decode<T: DeserializeOwned, B: Read>(
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
