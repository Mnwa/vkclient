use crate::Version;
use serde::de::DeserializeOwned;

/// # API method description
/// Wrapper for strong typed API method description.
///
/// Example
/// ```
/// use vkclient::{List, Version, VkApi, VkApiWrapper};
/// use serde::{Serialize, Deserialize};
///
/// let client: VkApi = vkclient::VkApiBuilder::new(access_token).into();
///
/// async {
///     let users = client.send_request_with_wrapper(UsersGetRequest {
///         user_ids: List(vec![1,2]),
///         fields: List(vec!["id", "sex"]),
///     }).await.expect("vk api error");
/// }
///
/// #[derive(Serialize, Debug)]
/// struct UsersGetRequest<'a> {
///     user_ids: List<Vec<usize>>,
///     fields: List<Vec<&'a str>>,
/// }
///
/// #[derive(Deserialize, Debug)]
/// struct UsersGetResponse {
///     id: i64,
///     first_name: String,
///     last_name: String,
///     sex: usize,
/// }
///
/// impl <'a>VkApiWrapper for UsersGetRequest<'a> {
///     type Response = UsersGetResponse;
///
///     fn get_method_name() -> &'static str {
///         "users.get"
///     }
///
///     fn get_version() -> Version {
///         Version(5, 131)
///     }
/// }
/// ```
pub trait VkApiWrapper {
    type Response: DeserializeOwned;

    /// Method name
    fn get_method_name() -> &'static str;

    /// API version that required for this method
    fn get_version() -> Version {
        Version::default()
    }
}
