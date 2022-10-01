# VK API Client

This is a base pure rust implementation of VK API client.

See the [library documentation](https://docs.rs/vkclient) or [VK API documentation](https://dev.vk.com/reference) for more.

## Usage
```rust
use vkclient::VkApi;

fn main() {
    let client = VkApi::from_access_token("XXX");
    
    ..
}
```