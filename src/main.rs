use twitch_irc::{
    login::{TokenStorage, UserAccessToken, RefreshingLoginCredentials},
    ClientConfig,
    SecureTCPTransport,
    TwitchIRCClient,
    message::ServerMessage,
};
use std::{
    fs, 
    env, 
    io::{self as stdio, Write},
    sync::Arc,
};
use tokio::{
    sync::broadcast,
    io::{self, AsyncBufReadExt, BufReader},
};
use async_trait::async_trait;
use dotenv::dotenv;
use dlog::{*, enums::OutputTarget};

#[derive(Debug)]
struct FileTokenStorage {
    path: String,
}

#[async_trait]
impl TokenStorage for FileTokenStorage {

    type LoadError = std::io::Error;
    type UpdateError = std::io::Error;

    async fn load_token(&mut self) -> Result<UserAccessToken, Self::LoadError> {
        let data = fs::read_to_string(&self.path)?;
        let token: UserAccessToken = serde_json::from_str(&data)?;
        Ok(token)
    }

    async fn update_token(&mut self, token: &UserAccessToken) -> Result<(), Self::UpdateError> {
        let data = serde_json::to_string(token)?;
        fs::write(&self.path, data)?;
        Ok(())
    }
}


#[tokio::main]
pub async fn main() {

    let mut log = Logger::init("twitch_bot", None, OutputTarget::Terminal).unwrap();

    dotenv().ok();
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID not set");
    let client_secret = env::var("CLIENT_SECRET").expect("CLIENT_SECRET not set");
    let twitch_channel = env::var("TWITCH_CHANNEL").expect("TWITCH_CHANNEL not set");

    let storage = FileTokenStorage { path: "token.json".to_string() };

    let credentials = RefreshingLoginCredentials::init(client_id, client_secret, storage);
    let config = ClientConfig::new_simple(credentials);
    let (mut incoming_messages, client) = TwitchIRCClient::<SecureTCPTransport, RefreshingLoginCredentials<FileTokenStorage>>::new(config);
    let client = Arc::new(client);
    let twitch_channel = Arc::new(twitch_channel);


    let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

    let client_for_chat = client.clone();
    let channel_for_chat = twitch_channel.clone();
    let join_handle = tokio::spawn({
        let mut shutdown_rx = shutdown_tx.subscribe();
        async move {
            loop {
                tokio::select! {
                    maybe_msg = incoming_messages.recv() => {
                        match maybe_msg {
                            Some(message) => match message {
                                ServerMessage::Privmsg(msg) => {
                                    if msg.message_text == "!hello" {
                                        let _ = client_for_chat.say(channel_for_chat.to_string(), format!("Hello {}", msg.sender.name)).await;
                                    }
                                    print!("\r\x1b[2k");
                                    log.info(format!("[{}]: {}", msg.sender.name, msg.message_text));
                                    print!("> ");
                                    stdio::stdout().flush().unwrap();
                                },
                                ServerMessage::Whisper(msg) => {
                                    log.info(format!("(w) {}: {}", msg.sender.name, msg.message_text));
                                },
                                _ => {}
                            },
                            None => break,
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        }
    });

    client.join(twitch_channel.to_string()).unwrap();
    

    let client_for_stdin = client.clone();
    let shutdown_tx2 = shutdown_tx.clone();
    let channel_for_stdin = twitch_channel.clone();
    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        loop {
            print!("> ");
            stdio::stdout().flush().unwrap();

            match reader.next_line().await {
                Ok(Some(line)) if !line.trim().is_empty() => {
                    if line.trim() == "!quit" {
                        let _ = client_for_stdin.part(channel_for_stdin.to_string());
                        let _ = shutdown_tx2.send(());
                        break;
                    } else {
                        let _ = client_for_stdin.say(channel_for_stdin.to_string(), line).await;
                    }
                }
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => break,
            }
        }
    });

    join_handle.await.unwrap();
}