use crate::inner::VkApiInner;
use crate::{Encoding, Format, VkApi};

/// API Client builder struct.
/// Use `VkApi::from` or `into` to make `VkApi` struct.
#[derive(Clone, Debug)]
pub struct VkApiBuilder {
    inner: VkApiInner,
}

impl VkApiBuilder {
    /// Creates the builder from access key with default values.
    pub fn new(access_token: String) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(features = "compression_zstd")] {
                let encoding = Encoding::Zstd;
            } else if #[cfg(features = "compression_gzip")] {
                let encoding = Encoding::Gzip;
            } else {
                let encoding = Encoding::None;
            }
        }
        cfg_if::cfg_if! {
            if #[cfg(features = "encode_msgpack")] {
                let format = Format::Msgpack;
            } else if #[cfg(features = "encode_json")] {
                let format = Format::Json;
            } else {
                let format = Format::None;
            }
        }

        Self {
            inner: VkApiInner {
                access_token,
                version: "5.131".to_string(),
                domain: "api.vk.com".to_string(),
                format,
                encoding,
            },
        }
    }

    /// Pass new access token to builder
    pub fn with_access_token(mut self, access_token: String) -> Self {
        self.inner.access_token = access_token;
        self
    }

    /// Pass new version to builder. Default is 5.131
    pub fn with_version(mut self, version: String) -> Self {
        self.inner.version = version;
        self
    }

    /// Pass new API domain to builder. Default is api.vk.com
    pub fn with_domain(mut self, domain: String) -> Self {
        self.inner.domain = domain;
        self
    }

    /// Pass new encoding to builder. Default is Encoding::Zstd
    pub fn with_encoding(mut self, encoding: Encoding) -> Self {
        self.inner.encoding = encoding;
        self
    }

    /// Pass new format to builder. Default is Format::Msgpack
    pub fn with_format(mut self, format: Format) -> Self {
        self.inner.format = format;
        self
    }
}

impl From<VkApiBuilder> for VkApi {
    fn from(builder: VkApiBuilder) -> Self {
        VkApi::from_inner(builder.inner)
    }
}
