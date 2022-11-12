# VK API Client

This is a base pure rust implementation of VK API client.
The client supports zstd compression and msgpack format of VK API. It's works with http2 only connections.

## Supported features
* API requests
* Longpoll
* Upload files

See the [library documentation](https://docs.rs/vkclient) or [VK API documentation](https://dev.vk.com/reference) for more.

## Usage
```rust
use vkclient::VkApi;

fn main() {
    let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
    ..
}
```

```rust
use vkclient::{VkApi, VkApiError, List};
use serde::{Deserialize, Serialize};

async fn get_users_info(client: &VkApi) -> Result<Vec<UsersGetResponse>, VkApiError> {
    client.send_request("users.get", UsersGetRequest {
        user_ids: List(vec![1,2]),
        fields: List(vec!["id", "sex"]),
   }).await
}

#[derive(Serialize)]
struct UsersGetRequest<'a> {
    user_ids: List<Vec<usize>>,
    fields: List<Vec<&'a str>>,
}

#[derive(Deserialize)]
struct UsersGetResponse {
    id: i64,
    first_name: String,
    last_name: String,
    sex: u8,
}
```