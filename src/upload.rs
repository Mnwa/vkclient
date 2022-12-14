use crate::inner::{create_client, uncompress};
use crate::VkApiError;
use cfg_if::cfg_if;
use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING};
use hyper::{Client, Request};
pub use hyper_multipart_rfc7578::client::multipart::*;
use hyper_rustls::HttpsConnector;
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
/// [Read more about uploads](https://dev.vk.com/api/upload).
#[derive(Clone, Debug)]
pub struct VkUploader {
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl VkUploader {
    /// Upload any form to given url.
    /// Supports gzip encoding for responses.
    /// Returns String, which must be passed to VK save file API.
    pub async fn upload<U: AsRef<str>>(
        &self,
        url: U,
        form: Form<'static>,
    ) -> Result<String, VkApiError> {
        cfg_if! {
            if #[cfg(feature = "compression_gzip")] {
                let encoding ="gzip";
            } else {
                let encoding ="identity";
            }
        }

        let req_builder = Request::post(url.as_ref())
            .header(ACCEPT_ENCODING, encoding)
            .header(ACCEPT, "application/json");

        let req = form.set_body::<Body>(req_builder).unwrap();

        let (parts, body) = self
            .client
            .request(req)
            .await
            .map_err(VkApiError::Request)?
            .into_parts();

        let body = hyper::body::aggregate(body)
            .await
            .map_err(VkApiError::Request)?;

        let mut body = uncompress(parts.headers.get(CONTENT_ENCODING), body.reader())?;

        let mut response = String::new();

        body.read_to_string(&mut response).map_err(VkApiError::IO)?;

        Ok(response)
    }
}

impl Default for VkUploader {
    fn default() -> Self {
        Self {
            client: create_client(),
        }
    }
}
