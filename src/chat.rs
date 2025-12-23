use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Chatter {
    user_id: String,
    user_login: String,
    user_name: String,
}

#[derive(Deserialize, Debug)]
struct ChattersResponse {
    data: Vec<Chatter>,
}

pub async fn get_chatters(
    broadcaster_id: &str,
    moderator_id: &str,
    oauth_token: &str,
    client_id: &str,
) -> Result<Vec<Chatter>, reqwest::Error> {

    let url = format!(
        "https://api.twitch.tv/helix/chat/chatters?broadcaster_id={}&moderator_id={}",
        broadcaster_id, moderator_id
    );

    let client = Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(oauth_token)
        .header("Client-Id", client_id)
        .send()
        .await?
        .json::<ChattersResponse>()
        .await?;

    Ok(resp.data)
}