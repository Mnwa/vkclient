use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vkclient::{longpoll::LongPollRequest, VkApi, VkApiWrapper};

fn main() {
    let access_token = std::env::var("SERVICE_TOKEN").unwrap();
    let group_id = std::env::var("GROUP_ID").unwrap().parse().unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();

        let BotLongPollResponse { key, server, ts } = client
            .send_request_with_wrapper(BotLongPollRequest { group_id })
            .await
            .unwrap();

        client
            .longpoll()
            .subscribe::<_, Value>(LongPollRequest {
                key,
                server,
                ts,
                wait: 25,
                additional_params: (),
            })
            .take(1)
            .for_each(|r| async move { println!("{:?}", r) })
            .await;
    });
}

#[derive(Serialize, Debug)]
struct BotLongPollRequest {
    group_id: usize,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BotLongPollResponse {
    key: String,
    server: String,
    ts: String,
}

impl VkApiWrapper for BotLongPollRequest {
    type Response = BotLongPollResponse;

    fn get_method_name() -> &'static str {
        "groups.getLongPollServer"
    }
}
