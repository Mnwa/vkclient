//! # Base VK API client realisation.
//! This client supports zstd compression and msgpack format of VK API. It's works with http2 only connections.
//!
//! ## VK API Clients list:
//! * [API client](crate::VkApi)
//! * [Uploader client](crate::upload::VkUploader)
//! * [Long Poll Client](crate::longpoll::VkLongPoll)
//!
//! ## Usage
//! ```rust
//! use vkclient::{List, VkApi, VkApiError};
//! let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
//!
//! async {
//!     let users = get_users_info(&client).await.unwrap();
//! }
//!
//! async fn get_users_info(client: &VkApi) -> Result<Vec<UsersGetResponse>, VkApiError> {
//!     client.send_request("users.get", UsersGetRequest {
//!         user_ids: List(vec![1,2]),
//!         fields: List(vec!["id", "sex"]),
//!    }).await
//! }
//!
//! #[derive(Serialize)]
//! struct UsersGetRequest<'a> {
//!     user_ids: List<Vec<usize>>,
//!     fields: List<Vec<&'a str>>,
//! }
//!
//! #[derive(Deserialize)]
//! struct UsersGetResponse {
//!     id: i64,
//!     first_name: String,
//!     last_name: String,
//!     sex: u8,
//! }
//! ```
//!
//! ## Features
//! * [compression_zstd](crate::Compression) - enabled by default. Adds zstd compression support;
//! * [compression_gzip](crate::Compression) - enabled by default. Adds gzip compression support;
//! * [encode_json](crate::Encoding) - enabled by default. Adds json encoding support;
//! * [encode_msgpack](crate::Encoding) - enabled by default. Adds msgpack encoding support;
//! * [uploader](crate::upload::VkUploader) - enabled by default. Adds file uploads support.
//! * [longpoll](crate::longpoll::VkLongPoll) - enabled by default. Adds longpoll support.
//! * [longpoll_stream](crate::longpoll::VkLongPoll::subscribe) - enabled by default. Adds converter long poll queries to futures stream.

mod builder;
mod inner;
mod structs;
mod vkapi;

#[cfg(feature = "longpoll")]
pub mod longpoll;
#[cfg(feature = "uploader")]
pub mod upload;
mod wrapper;

pub use builder::VkApiBuilder;
pub use structs::*;
pub use vkapi::*;
pub use wrapper::VkApiWrapper;
