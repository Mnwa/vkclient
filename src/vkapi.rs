use crate::inner::{create_client, decode, uncompress, VkApiInner};
use crate::structs::Version;
use crate::wrapper::VkApiWrapper;
use bytes::Buf;
use cfg_if::cfg_if;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

/// # Base VK API client realisation.
/// This client supports zstd compression and msgpack format of VK API. It's works with http2 only connections.
///
/// ## Usage
/// ```rust
/// use vkclient::VkApi;
/// let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
/// ```
#[derive(Debug, Clone)]
pub struct VkApi {
    inner: Arc<VkApiInner>,
    client: Client,
}

impl VkApi {
    pub(crate) fn from_inner(inner: VkApiInner) -> Self {
        let client = create_client();

        Self {
            inner: Arc::new(inner),
            client,
        }
    }

    /// Send request to VK API. See list of [VK API methods](https://dev.vk.com/method).
    /// ```rust
    /// use vkclient::{VkApi, VkApiResult, List};
    /// use serde::{Deserialize, Serialize};
    ///
    /// async fn get_users_info(client: &VkApi) -> VkApiResult<Vec<UsersGetResponse>> {
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
    pub async fn send_request<T, B, M>(&self, method: M, body: B) -> VkApiResult<T>
    where
        T: DeserializeOwned,
        B: Serialize + Send,
        M: AsRef<str> + Send,
    {
        self.send_request_with_version(method, body, self.inner.version)
            .await
    }

    /// Send request to VK API struct that implement `VkApiWrapper` trait
    pub async fn send_request_with_wrapper<W>(&self, wrapper: W) -> VkApiResult<W::Response>
    where
        W: VkApiWrapper + Serialize + Send,
    {
        self.send_request_with_version(W::get_method_name(), wrapper, W::get_version())
            .await
    }

    /// Send request to VK API with specific version.
    pub async fn send_request_with_version<T, B, M>(
        &self,
        method: M,
        body: B,
        version: Version,
    ) -> VkApiResult<T>
    where
        T: DeserializeOwned,
        B: Serialize + Send,
        M: AsRef<str> + Send,
    {
        cfg_if! {
            if #[cfg(feature = "encode_msgpack")] {
                let url = if matches!(self.inner.format, Encoding::Msgpack) {
                    format!(
                        "https://{}/method/{}.msgpack",
                        self.inner.domain,
                        method.as_ref()
                    )
                } else {
                    format!("https://{}/method/{}", self.inner.domain, method.as_ref())
                };
            } else {
                let url = format!("https://{}/method/{}", self.inner.domain, method.as_ref());
            }
        }

        let request = self
            .client
            .post(url)
            .header(
                ACCEPT_ENCODING,
                match self.inner.encoding {
                    #[cfg(feature = "compression_zstd")]
                    Compression::Zstd => "zstd",
                    #[cfg(feature = "compression_gzip")]
                    Compression::Gzip => "gzip",
                    Compression::None => "identity",
                },
            )
            .header(
                ACCEPT,
                match self.inner.format {
                    #[cfg(feature = "encode_msgpack")]
                    Encoding::Msgpack => "application/x-msgpack",
                    #[cfg(feature = "encode_json")]
                    Encoding::Json => "application/json",
                    Encoding::None => "text/*",
                },
            )
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&VkApiBody {
                v: &version,
                access_token: self.inner.access_token.as_str(),
                body,
            });

        let response = request.send().await.map_err(VkApiError::Request)?;
        let headers = response.headers();

        let content_type = headers.get(CONTENT_TYPE).cloned();
        let content_encoding = headers.get(CONTENT_ENCODING).cloned();

        let body = response.bytes().await.map_err(VkApiError::Request)?;

        let resp =
            decode::<Response<T>, _>(&content_type, uncompress(content_encoding, body.reader())?)?;

        match resp {
            Response::Success { response } => Ok(response),
            Response::Error { error } => Err(VkApiError::Vk(error)),
        }
    }

    /// Returns `VkLongPoll` client with the same connection pool as the vk api client.
    #[cfg(feature = "longpoll")]
    pub fn longpoll(&self) -> crate::longpoll::VkLongPoll {
        crate::longpoll::VkLongPoll::from(self.client.clone())
    }

    /// Returns `VkUploader` client with the same connection pool as the vk api client.
    #[cfg(feature = "uploader")]
    pub fn uploader(&self) -> crate::upload::VkUploader {
        crate::upload::VkUploader::from(self.client.clone())
    }
}

/// Vk Api errors.
/// `VkApiError::Vk` - is an error of buisness logic, like expired token or incorrect request params
/// Other errors is about things around your request, like a serialization/deserialization or network errors.
#[derive(Debug)]
pub enum VkApiError {
    Request(reqwest::Error),
    RequestSerialize(serde_urlencoded::ser::Error),
    ResponseDeserialize(ResponseDeserialize),
    Vk(VkError),
    IO(std::io::Error),
    #[cfg(feature = "longpoll")]
    LongPoll(crate::longpoll::LongPollError),
}

impl Display for VkApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IO(e) => Display::fmt(e, f),
            Self::Request(e) => Display::fmt(e, f),
            Self::ResponseDeserialize(e) => Display::fmt(e, f),
            Self::Vk(e) => Display::fmt(e, f),
            Self::RequestSerialize(e) => Display::fmt(e, f),
            #[cfg(feature = "longpoll")]
            Self::LongPoll(e) => Display::fmt(e, f),
        }
    }
}

impl Error for VkApiError {}

/// Shorthand for ``Result<T, VkApiError>``
pub type VkApiResult<T> = Result<T, VkApiError>;

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
            Self::Json(e) => Display::fmt(e, f),
            #[cfg(feature = "encode_msgpack")]
            Self::MsgPack(e) => Display::fmt(e, f),
            Self::BadEncoding => {
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
struct VkApiBody<'a, T> {
    v: &'a Version,
    access_token: &'a str,
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
