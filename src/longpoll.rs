use crate::inner::{create_client, decode, uncompress};
use crate::VkApiError;
use cfg_if::cfg_if;
use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use hyper::{Body, Client, Method, Request};
use hyper_rustls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[cfg(feature = "longpoll_stream")]
use futures_util::{Stream, StreamExt, TryStreamExt};

/// # Client for long poll subscriptions
/// Use it to subscribe on some VK events, like
/// the [UserLong Poll API](https://dev.vk.com/api/user-long-poll/getting-started)
/// or the [Bots Long Poll API](https://dev.vk.com/api/bots-long-poll/getting-started).
///
/// ## Usage:
/// ```rust
/// use vkclient::VkApi;
/// let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
///
/// let longpoll_client = client.longpoll();
/// ```
///
/// ```rust
/// use vkclient::LongPollClient;
///
/// let longpoll_client = LongPollClient::default();
/// ```
#[derive(Debug, Clone)]
pub struct LongPollClient {
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl LongPollClient {
    /// Returns an events stream from long poll server.
    #[cfg(feature = "longpoll_stream")]
    pub fn subscribe<T: Serialize + Clone, I: DeserializeOwned>(
        &self,
        request: LongPollRequest<T>,
    ) -> impl Stream<Item = Result<I, VkApiError>> {
        self.subscribe_inner::<T, I>(request)
            .map_ok(|r| futures_util::stream::iter(r.updates).map(Ok))
            .try_flatten()
    }

    #[cfg(feature = "longpoll_stream")]
    fn subscribe_inner<T: Serialize + Clone, I: DeserializeOwned>(
        &self,
        mut request: LongPollRequest<T>,
    ) -> impl Stream<Item = Result<LongPollSuccess<I>, VkApiError>> {
        let client = self.client.clone();
        async_stream::stream! {
            loop {
                match Self::subscribe_once_with_client(&client, request.clone()).await {
                    Err(VkApiError::LongPoll(LongPollError { ts: Some(ts), .. })) => {
                        request.ts = ts;
                    },
                    Ok(LongPollSuccess{ ts, updates }) => {
                        request.ts = ts;
                        yield Ok(LongPollSuccess { ts, updates })
                    },
                    r => {
                        yield r;
                        break;
                    },
                };
            }
        }
    }

    pub async fn subscribe_once<T: Serialize, I: DeserializeOwned>(
        &self,
        request: LongPollRequest<T>,
    ) -> Result<LongPollSuccess<I>, VkApiError> {
        Self::subscribe_once_with_client(&self.client, request).await
    }

    async fn subscribe_once_with_client<T: Serialize, I: DeserializeOwned>(
        client: &Client<HttpsConnector<HttpConnector>, Body>,
        request: LongPollRequest<T>,
    ) -> Result<LongPollSuccess<I>, VkApiError> {
        let LongPollInnerRequest(LongPollServer(server), params) =
            LongPollInnerRequest::from(request);

        let params = serde_urlencoded::to_string(params).map_err(VkApiError::RequestSerialize)?;

        let url = if server.starts_with("http") {
            format!("{}?{}", server, params)
        } else {
            format!("https://{}?{}", server, params)
        };

        cfg_if! {
            if #[cfg(features = "compression_gzip")] {
                let encoding = "gzip";
            } else {
                let encoding =  "identity";
            }
        }

        cfg_if! {
            if #[cfg(features = "encode_json")] {
                let serialisation = "application/json";
            } else {
                let serialisation =  "text/*";
            }
        }

        let request = Request::builder()
            .method(Method::GET)
            .header(ACCEPT_ENCODING, encoding)
            .header(ACCEPT, serialisation)
            .uri(url)
            .body(Body::empty())
            .map_err(VkApiError::Http)?;

        let response = client.request(request).await.map_err(VkApiError::Request)?;

        let (parts, body) = response.into_parts();

        let body = hyper::body::aggregate(body)
            .await
            .map_err(VkApiError::Request)?;

        let resp = decode::<LongPollResponse<I>, _>(
            parts.headers.get(CONTENT_TYPE),
            uncompress(parts.headers.get(CONTENT_ENCODING), body.reader())?,
        )?;

        match resp {
            LongPollResponse::Success(r) => Ok(r),
            LongPollResponse::Error(e) => Err(VkApiError::LongPoll(e)),
        }
    }
}

impl From<Client<HttpsConnector<HttpConnector>, Body>> for LongPollClient {
    fn from(client: Client<HttpsConnector<HttpConnector>, Body>) -> Self {
        Self { client }
    }
}

impl Default for LongPollClient {
    fn default() -> Self {
        Self::from(create_client())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum LongPollResponse<R> {
    Success(LongPollSuccess<R>),
    Error(LongPollError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongPollSuccess<R> {
    ts: usize,
    updates: Vec<R>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongPollError {
    failed: usize,
    #[serde(default)]
    ts: Option<usize>,
    #[serde(default)]
    min_version: Option<usize>,
    #[serde(default)]
    max_version: Option<usize>,
}

impl Display for LongPollError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "long poll error occured, code: {}", self.failed,)
    }
}

impl Error for LongPollError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongPollRequest<T> {
    pub server: String,
    pub key: String,
    pub ts: usize,
    pub wait: usize,
    #[serde(flatten)]
    pub additional_params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LongPollServer(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LongPollQueryParams<T> {
    key: String,
    ts: usize,
    wait: usize,
    #[serde(flatten)]
    additional_params: T,
}

struct LongPollInnerRequest<T>(LongPollServer, LongPollQueryParams<T>);

impl<T> From<LongPollRequest<T>> for LongPollInnerRequest<T> {
    fn from(
        LongPollRequest {
            server,
            key,
            ts,
            wait,
            additional_params,
        }: LongPollRequest<T>,
    ) -> Self {
        LongPollInnerRequest(
            LongPollServer(server),
            LongPollQueryParams {
                key,
                ts,
                wait,
                additional_params,
            },
        )
    }
}
