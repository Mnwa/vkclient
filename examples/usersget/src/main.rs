use serde::{Deserialize, Serialize};
use std::time::Instant;
use vkclient::{Compression, Encoding, List, VkApi, VkApiResult};

fn main() {
    let access_token = std::env::var("SERVICE_TOKEN").unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let client: VkApi = vkclient::VkApiBuilder::new(access_token.clone())
            .with_encoding(Encoding::Json)
            .with_compression(Compression::Gzip)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("json+gzip {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::VkApiBuilder::new(access_token.clone())
            .with_encoding(Encoding::Msgpack)
            .with_compression(Compression::Zstd)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("msgpack+zstd {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::VkApiBuilder::new(access_token.clone())
            .with_encoding(Encoding::Json)
            .with_compression(Compression::Zstd)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("json+zstd {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::VkApiBuilder::new(access_token.clone())
            .with_encoding(Encoding::Msgpack)
            .with_compression(Compression::Gzip)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("msgpack+gzip {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::VkApiBuilder::new(access_token.clone())
            .with_encoding(Encoding::Json)
            .with_compression(Compression::None)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("json+none {} micros", i.elapsed().as_micros());

        let client: VkApi = vkclient::VkApiBuilder::new(access_token.clone())
            .with_encoding(Encoding::Msgpack)
            .with_compression(Compression::None)
            .into();

        let i = Instant::now();
        assert!(get_users_info(&client).await.is_ok());
        println!("msgpack+none {} micros", i.elapsed().as_micros());
    });
}
async fn get_users_info(client: &VkApi) -> VkApiResult<Vec<UsersGetResponse>> {
    client
        .send_request(
            "users.get",
            UsersGetRequest {
                user_ids: List(vec![1, 2, 3]),
                fields: List(vec!["id", "sex"]),
            },
        )
        .await
}

#[derive(Serialize, Debug)]
struct UsersGetRequest<'a> {
    user_ids: List<Vec<usize>>,
    fields: List<Vec<&'a str>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct UsersGetResponse {
    id: i64,
    first_name: String,
    last_name: String,
    #[serde(default)]
    sex: Option<usize>,
}
