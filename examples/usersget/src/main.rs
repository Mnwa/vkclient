use serde::{Deserialize, Serialize};
use vkclient::{Encoding, Format, VkApi, VkApiError};

fn main() {
    let access_token = std::env::var("SERVICE_TOKEN").unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token)
            .with_format(Format::Msgpack)
            .with_encoding(Encoding::Zstd)
            .into();

        println!("{:#?}", get_users_info(&client).await)
    });
}
async fn get_users_info(client: &VkApi) -> Result<Vec<UsersGetResponse>, VkApiError> {
    client
        .send_request(
            "users.get",
            UsersGetRequest {
                user_ids: "1,2",
                fields: "id,sex",
            },
        )
        .await
}

#[derive(Serialize, Debug)]
struct UsersGetRequest<'a> {
    user_ids: &'a str,
    fields: &'a str,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct UsersGetResponse {
    id: i64,
    first_name: String,
    last_name: String,
    sex: usize,
}
