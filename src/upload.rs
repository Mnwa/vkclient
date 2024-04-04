use crate::inner::{create_client, uncompress};
use crate::VkApiError;
use bytes::Buf;
use cfg_if::cfg_if;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING};
pub use reqwest::multipart::Form;
use reqwest::Client;
use std::io::Read;

/// # Upload files to VK Uploader Servers
/// Firstly you need to get any uploader server from VK API.
/// As example [photos.getUploadServer](https://dev.vk.com/method/photos.getUploadServer).
/// Then you can use this struct to upload files.
/// Example:
/// ```rust
/// use vkclient::upload::{Form, VkUploader};
/// let uploader = VkUploader::default();
///
/// let url = "...";
/// let form = Form::new();
///
/// async {
///     let response: String = uploader.upload(url, form).await.expect("uploading error");
/// }
/// ```
///
/// ```rust
/// use vkclient::VkApi;
/// let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
///
/// let uploader = client.uploader();
/// ```
///
/// [Read more about uploads](https://dev.vk.com/api/upload).
#[derive(Clone, Debug)]
pub struct VkUploader {
    client: Client,
}

impl VkUploader {
    /// Upload any form to given url.
    /// Supports gzip encoding for responses.
    /// Returns String, which must be passed to VK save file API.
    pub async fn upload<U: AsRef<str>>(&self, url: U, form: Form) -> Result<String, VkApiError> {
        cfg_if! {
            if #[cfg(feature = "compression_gzip")] {
                let encoding ="gzip";
            } else {
                let encoding ="identity";
            }
        }

        let req = self
            .client
            .post(url.as_ref())
            .header(ACCEPT_ENCODING, encoding)
            .header(ACCEPT, "application/json")
            .multipart(form);

        let response = req.send().await.map_err(VkApiError::Request)?;
        let headers = response.headers();

        let content_encoding = headers.get(CONTENT_ENCODING).cloned();

        let body = response.bytes().await.map_err(VkApiError::Request)?;

        let mut body = uncompress(content_encoding, body.reader())?;

        let mut response = String::new();

        body.read_to_string(&mut response).map_err(VkApiError::IO)?;

        Ok(response)
    }
}

impl From<Client> for VkUploader {
    fn from(client: Client) -> Self {
        Self { client }
    }
}

impl Default for VkUploader {
    fn default() -> Self {
        Self {
            client: create_client(),
        }
    }
}
