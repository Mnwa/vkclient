use crate::inner::{create_client, decode, uncompress};
use crate::VkApiError;
use cfg_if::cfg_if;
use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use hyper::{Body, Client, Method, Request};
use hyper_rustls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

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
/// use vkclient::longpoll::VkLongPoll;
///
/// let longpoll_client = VkLongPoll::default();
/// ```
#[derive(Debug, Clone)]
pub struct VkLongPoll {
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl VkLongPoll {
    /// Returns an events stream from long poll server.
    ///
    /// ## Usage
    /// ```rust
    /// use vkclient::longpoll::{VkLongPoll, LongPollRequest};
    ///
    /// let longpoll_client = VkLongPoll::default();
    ///
    /// longpoll_client.subscribe(LongPollRequest {
    ///         key,
    ///         server,
    ///         ts,
    ///         wait: 25,
    ///         additional_params: (),
    ///     })
    ///     .take(1)
    ///     .for_each(|r| async move { println!("{:?}", r) });
    /// ```
    #[cfg(feature = "longpoll_stream")]
    pub fn subscribe<T: Serialize + Clone, I: DeserializeOwned>(
        &self,
        mut request: LongPollRequest<T>,
    ) -> impl futures_util::Stream<Item = Result<I, VkApiError>> {
        let client = self.client.clone();

        async_stream::stream! {
            loop {
                match Self::subscribe_once_with_client(&client, request.clone()).await {
                    Err(VkApiError::LongPoll(LongPollError { ts: Some(ts), .. })) => {
                        request.ts = ts;
                    },
                    Ok(LongPollSuccess{ ts, updates }) => {
                        request.ts = ts.clone();
                        for update in updates {
                            yield Ok(update);
                        }
                    },
                    Err(e) => {
                        yield Err(e);
                        break;
                    },
                };
            }
        }
    }

    /// Returns first events chunk from long poll server.
    ///
    /// ## Usage
    /// ```rust
    /// use vkclient::longpoll::{VkLongPoll, LongPollRequest};
    ///
    /// let longpoll_client = VkLongPoll::default();
    ///
    /// longpoll_client.subscribe_once(LongPollRequest {
    ///         key,
    ///         server,
    ///         ts,
    ///         wait: 25,
    ///         additional_params: (),
    ///     });
    /// ```
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
            format!("{}?act=a_check&{}", server, params)
        } else {
            format!("https://{}?act=a_check&{}", server, params)
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

        let body = hyper::body::to_bytes(body)
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

impl From<Client<HttpsConnector<HttpConnector>, Body>> for VkLongPoll {
    fn from(client: Client<HttpsConnector<HttpConnector>, Body>) -> Self {
        Self { client }
    }
}

impl Default for VkLongPoll {
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

/// Long poll events chunk. You should to replace ts on next request with this value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongPollSuccess<R> {
    ts: String,
    updates: Vec<R>,
}

/// Long poll error.
/// [Read more about possible errors](https://dev.vk.com/api/user-long-poll/getting-started#%D0%A4%D0%BE%D1%80%D0%BC%D0%B0%D1%82%20%D0%BE%D1%82%D0%B2%D0%B5%D1%82%D0%B0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongPollError {
    failed: usize,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_usize_or_string_option")]
    ts: Option<String>,
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

/// Long poll request structure.
/// * `server`, `key` and `ts` you should get from VK API.
/// * `wait` is the timeout in seconds for this long poll request. Recommended value: 25.
/// * `additional_params` is a custom struct, which will be inlined to request for passing external data, like a `mode`, `version`, etc. Put an empty tuple if you don't need it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongPollRequest<T> {
    pub server: String,
    pub key: String,
    #[serde(deserialize_with = "deserialize_usize_or_string")]
    pub ts: String,
    pub wait: usize,
    #[serde(flatten)]
    pub additional_params: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LongPollServer(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LongPollQueryParams<T> {
    key: String,
    #[serde(deserialize_with = "deserialize_usize_or_string")]
    ts: String,
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

struct DeserializeUsizeOrString;

impl<'de> serde::de::Visitor<'de> for DeserializeUsizeOrString {
    type Value = String;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer or a string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }
}

struct DeserializeUsizeOrStringOption;

impl<'de> serde::de::Visitor<'de> for DeserializeUsizeOrStringOption {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an integer or a string or a null")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Some(v.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Some(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Some(v))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }
}

fn deserialize_usize_or_string<'de, D>(
    deserializer: D,
) -> Result<String, <D as Deserializer<'de>>::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeUsizeOrString)
}

fn deserialize_usize_or_string_option<'de, D>(
    deserializer: D,
) -> Result<Option<String>, <D as Deserializer<'de>>::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeUsizeOrStringOption)
}

#[cfg(test)]
mod tests {
    use crate::longpoll::{deserialize_usize_or_string, deserialize_usize_or_string_option};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Ts {
        #[serde(deserialize_with = "deserialize_usize_or_string")]
        ts: String,
    }

    #[derive(Deserialize)]
    struct TsOpt {
        #[serde(default)]
        #[serde(deserialize_with = "deserialize_usize_or_string_option")]
        ts: Option<String>,
    }

    #[test]
    fn test_deserialize_ts_string() {
        let ts: Ts = serde_json::from_str(r#"{"ts": "123"}"#).unwrap();
        assert_eq!(ts.ts, "123".to_string())
    }

    #[test]
    fn test_deserialize_ts_usize() {
        let ts: Ts = serde_json::from_str(r#"{"ts": 123}"#).unwrap();
        assert_eq!(ts.ts, "123".to_string())
    }

    #[test]
    fn test_deserialize_ts_opt_string() {
        let ts: TsOpt = serde_json::from_str(r#"{"ts": "123"}"#).unwrap();
        assert_eq!(ts.ts, Some("123".to_string()))
    }

    #[test]
    fn test_deserialize_ts_opt_usize() {
        let ts: TsOpt = serde_json::from_str(r#"{"ts": 123}"#).unwrap();
        assert_eq!(ts.ts, Some("123".to_string()))
    }

    #[test]
    fn test_deserialize_ts_opt_none() {
        let ts: TsOpt = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(ts.ts, None)
    }
}
