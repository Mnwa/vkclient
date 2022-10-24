use crate::inner::VkApiInner;
use crate::structs::Version;
use crate::vkapi::{Compression, Encoding, VkApi};

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
                let encoding = Compression::Zstd;
            } else if #[cfg(features = "compression_gzip")] {
                let encoding = Compression::Gzip;
            } else {
                let encoding = Compression::None;
            }
        }
        cfg_if::cfg_if! {
            if #[cfg(features = "encode_msgpack")] {
                let format = Encoding::Msgpack;
            } else if #[cfg(features = "encode_json")] {
                let format = Encoding::Json;
            } else {
                let format = Encoding::None;
            }
        }

        Self {
            inner: VkApiInner {
                access_token,
                version: Version::default(),
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
    pub fn with_version(mut self, version: Version) -> Self {
        self.inner.version = version;
        self
    }

    /// Pass new API domain to builder. Default is api.vk.com
    pub fn with_domain(mut self, domain: String) -> Self {
        self.inner.domain = domain;
        self
    }

    /// Pass new compression to builder. Default is Compression::Zstd
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.inner.encoding = compression;
        self
    }

    /// Pass new encoding to builder. Default is Encoding::Msgpack
    pub fn with_encoding(mut self, encoding: Encoding) -> Self {
        self.inner.format = encoding;
        self
    }
}

impl From<VkApiBuilder> for VkApi {
    fn from(builder: VkApiBuilder) -> Self {
        VkApi::from_inner(builder.inner)
    }
}
