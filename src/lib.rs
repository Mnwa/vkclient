use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Base VK API client realisation.
#[derive(Debug, Clone)]
pub struct VkApi {
    domain: String,
    access_token: String,
    version: String,
}

impl VkApi {
    /// Create client with service_token and default values for domain and version
    /// See [VK Access Tokens](https://dev.vk.com/api/access-token/getting-started) documentation.
    /// ```rust
    /// use vkclient::VkApi;
    /// let client = VkApi::from_access_token("XXX");
    /// ```
    pub fn from_access_token<S: Into<String>>(service_token: S) -> Self {
        Self::new("api.vk.com", "5.131", service_token)
    }

    /// Create client with all needed meta data
    /// ```rust
    /// use vkclient::VkApi;
    /// let client = VkApi::new("api.vk.com", "5.131", "XXX");
    /// ```
    pub fn new<D: Into<String>, V: Into<String>, A: Into<String>>(
        domain: D,
        version: V,
        access_token: A,
    ) -> Self {
        Self {
            domain: domain.into(),
            access_token: access_token.into(),
            version: version.into(),
        }
    }

    /// Send request to VK API. See list of [VK API methods](https://dev.vk.com/method).
    /// ```rust
    /// use vkclient::{VkApi, VkApiError};
    /// use serde::{Deserialize, Serialize};
    ///
    /// async fn get_users_info(client: &VkApi) -> Result<Vec<UsersGetResponse>, VkApiError> {
    ///     client.send_request("users.get", UsersGetRequest {
    ///         user_ids: vec![1],
    ///         fields: vec![],
    ///    }).await
    /// }
    ///
    /// #[derive(Serialize)]
    /// struct UsersGetRequest {
    ///     user_ids: Vec<i64>,
    ///     fields: Vec<String>,
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct UsersGetResponse {
    ///     id: i64,
    ///     first_name: String,
    ///     last_name: String,
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
        let url = format!("https://{}/method/{}", self.domain, method.as_ref());

        let resp = awc::Client::new()
            .post(url)
            .send_form(&VkApiBody {
                v: &self.version,
                access_token: &self.access_token,
                body,
            })
            .await
            .map_err(VkApiError::SendRequest)?
            .json::<Response<T>>()
            .await
            .map_err(VkApiError::RequestDeserialize)?;

        match resp {
            Response::Success { response } => Ok(response),
            Response::Error { error } => Err(VkApiError::Vk(error)),
        }
    }
}

/// Vk Api Error description.
#[derive(Debug)]
pub enum VkApiError {
    SendRequest(awc::error::SendRequestError),
    RequestDeserialize(awc::error::JsonPayloadError),
    Vk(VkError),
}

impl Display for VkApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VkApiError::SendRequest(e) => e.fmt(f),
            VkApiError::RequestDeserialize(e) => e.fmt(f),
            VkApiError::Vk(e) => e.fmt(f),
        }
    }
}

impl Error for VkApiError {}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VkApiBody<'a, T> {
    access_token: &'a str,
    v: &'a str,
    #[serde(flatten)]
    body: T,
}

impl Error for VkError {}
