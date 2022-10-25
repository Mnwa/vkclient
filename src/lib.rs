//! # Base VK API client realisation.
//! This client supports zstd compression and msgpack format of VK API. It's works with http2 only connections.
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
//! * compression_zstd - enabled by default. Adds zstd compression support;
//! * compression_gzip - enabled by default. Adds gzip compression support;
//! * encode_json - enabled by default. Adds json encoding support;
//! * encode_msgpack - enabled by default. Adds msgpack encoding support;

mod builder;
mod inner;
mod structs;
mod vkapi;

#[cfg(feature = "uploader")]
pub mod upload;

pub use builder::VkApiBuilder;
pub use structs::*;
pub use vkapi::*;
