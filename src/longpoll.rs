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

// todo: DOCS
#[derive(Debug, Clone)]
pub struct LongPollClient {
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl LongPollClient {
    #[cfg(feature = "longpoll_stream")]
    pub fn subscribe<T: Serialize + Clone, I: DeserializeOwned>(
        &self,
        mut request: LongPollRequest<T>,
    ) -> impl futures_util::Stream<Item = Result<LongPollResponse<I>, VkApiError>> {
        let client = self.client.clone();
        async_stream::try_stream! {
            loop {
                match Self::subscribe_once_with_client(&client, request.clone()).await? {
                    LongPollResponse::Error { ts: Some(ts), .. } => {
                        request.ts = ts;
                    }
                    LongPollResponse::Success { ts, updates } => {
                        request.ts = ts;
                        yield LongPollResponse::Success { ts, updates }
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
    ) -> Result<LongPollResponse<I>, VkApiError> {
        Self::subscribe_once_with_client(&self.client, request).await
    }

    async fn subscribe_once_with_client<T: Serialize, I: DeserializeOwned>(
        client: &Client<HttpsConnector<HttpConnector>, Body>,
        request: LongPollRequest<T>,
    ) -> Result<LongPollResponse<I>, VkApiError> {
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

        Ok(resp)
    }
}

impl Default for LongPollClient {
    fn default() -> Self {
        Self {
            client: create_client(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LongPollResponse<R> {
    Success {
        ts: usize,
        updates: Vec<R>,
    },
    Error {
        failed: usize,
        #[serde(default)]
        ts: Option<usize>,
        #[serde(default)]
        min_version: Option<usize>,
        #[serde(default)]
        max_version: Option<usize>,
    },
}

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
    pub key: String,
    pub ts: usize,
    pub wait: usize,
    #[serde(flatten)]
    pub additional_params: T,
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
