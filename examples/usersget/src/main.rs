use serde::{Deserialize, Serialize};
use std::time::Instant;
use vkclient::{Encoding, Format, VkApi, VkApiError};

fn main() {
    let access_token = std::env::var("SERVICE_TOKEN").unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token.clone())
            .with_format(Format::Json)
            .with_encoding(Encoding::Gzip)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("json+gzip {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token.clone())
            .with_format(Format::Msgpack)
            .with_encoding(Encoding::Zstd)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("msgpack+zstd {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token.clone())
            .with_format(Format::Json)
            .with_encoding(Encoding::Zstd)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("json+zstd {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token.clone())
            .with_format(Format::Msgpack)
            .with_encoding(Encoding::Gzip)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("msgpack+gzip {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token.clone())
            .with_format(Format::Json)
            .with_encoding(Encoding::None)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("json+none {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::builder::VkApiBuilder::new(access_token.clone())
            .with_format(Format::Msgpack)
            .with_encoding(Encoding::None)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("msgpack+none {} micros", i.elapsed().as_micros());
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
