use crate::VkApiError;
use cfg_if::cfg_if;
use hyper::body::Buf;
use hyper::client::HttpConnector;
use hyper::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING};
use hyper::{Client, Request};
pub use hyper_multipart_rfc7578::client::multipart::*;
use hyper_rustls::HttpsConnector;
use std::io::{self, Read};

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

        let body = hyper::body::to_bytes(body)
            .await
            .map_err(VkApiError::Request)?;

        #[cfg(feature = "compression_gzip")]
        if matches!(parts.headers.get(CONTENT_ENCODING), Some(e) if e == "gzip") {
            let mut response = String::new();

            let mut reader = flate2::read::GzDecoder::new(body.reader());
            reader
                .read_to_string(&mut response)
                .map_err(VkApiError::IO)?;

            return Ok(response);
        }

        std::str::from_utf8(body.as_ref())
            .map(|b| b.to_string())
            .map_err(|e| VkApiError::IO(io::Error::new(io::ErrorKind::InvalidData, e)))
    }
}

impl Default for VkUploader {
    fn default() -> Self {
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http2()
            .build();

        let client = Client::builder().http2_only(true).build(https);

        Self { client }
    }
}
