mod prelude;
mod chat;

use chat::*;
use prelude::*;

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

pub struct TwitchBot {
    client: Arc<TwitchIRCClient<SecureTCPTransport, RefreshingLoginCredentials<FileTokenStorage>>>,
    incoming_messages: tokio::sync::mpsc::UnboundedReceiver<ServerMessage>,
    channel: Arc<String>,
    log: Logger,
    shutdown_tx: broadcast::Sender<()>,
}

impl TwitchBot {

    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {

        let mut log = Logger::init("twitch_bot", None, OutputTarget::Terminal).unwrap();

        log.info("Loading .env file.");
        dotenv().ok();
        let client_id = env::var("CLIENT_ID").expect("CLIENT_ID not set");
        let client_secret = env::var("CLIENT_SECRET").expect("CLIENT_SECRET not set");
        let twitch_channel = env::var("TWITCH_CHANNEL").expect("TWITCH_CHANNEL not set");

        log.info("Checking token.json.");
        let storage = FileTokenStorage { path: "token.json".to_string() };

        log.info("Checking Credentials.");
        let credentials = RefreshingLoginCredentials::init(client_id, client_secret, storage);
        let config = ClientConfig::new_simple(credentials);
        log.info("Setting client/incoming_messages.");
        let (mut incoming_messages, client) = TwitchIRCClient::<SecureTCPTransport, RefreshingLoginCredentials<FileTokenStorage>>::new(config);

        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        log.info("Initiating twitchbot.");
        Ok(TwitchBot {
            client: Arc::new(client),
            incoming_messages,
            channel: Arc::new(twitch_channel),
            log,
            shutdown_tx,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.log.info("Listening for chat.");
        let join_handle = self.handle_chat();

        self.log.info(format!("Joining channel: {}", &self.channel));
        self.join_channel();
        self.log.info("Listening for terminal input.");
        self.handle_input();

        join_handle.await.unwrap();
        Ok(())
    }

    fn join_channel(&self) {
        self.client.join(self.channel.to_string()).unwrap();
    }

    fn handle_chat(&mut self) -> tokio::task::JoinHandle<()> {
        let client = self.client.clone();
        let channel = self.channel.clone();
        let shutdown_tx = self.shutdown_tx.clone();

        let (_dummy_sender, dummy_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut incoming_messages = std::mem::replace(&mut self.incoming_messages, dummy_receiver);

        tokio::spawn({
            let mut shutdown_rx = shutdown_tx.subscribe();
            async move {
                loop {
                    tokio::select! {
                        maybe_msg = incoming_messages.recv() => {
                            match maybe_msg {

                                // Core Twitch information flow/control
                                Some(message) => match message {

                                    // Regular chat control
                                    ServerMessage::Privmsg(msg) => {
                                        if msg.message_text == "!hello" {
                                            let _ = client.say(channel.to_string(), format!("Hello {}", msg.sender.name)).await;
                                        }
                                        format_messages(format!("[{}]: {}", msg.sender.name, msg.message_text))
                                    },

                                    // Whisper interactions.
                                    ServerMessage::Whisper(msg) => {
                                        format_messages(format!("(w) {}: {}", msg.sender.name, msg.message_text))
                                    },

                                    ServerMessage::Join(msg) => {
                                        if msg.user_login != "daegonica_software" {
                                            format_messages(format!("({}) joined chat.", msg.user_login))
                                        } else {
                                            let _ = client.say(channel.to_string(), format!("twitch_bot activated.")).await;
                                        }
                                    },
                                    ServerMessage::Part(msg) => {
                                        format_messages(format!("({}) left chat.", msg.user_login))
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
        })
    }

    fn handle_input(&mut self) {

        let client = self.client.clone();
        let channel = self.channel.clone();
        let shutdown_tx2 = self.shutdown_tx.clone();

        tokio::spawn(async move {
            let stdin = io::stdin();
            let mut reader = BufReader::new(stdin).lines();
            loop {
                print!("> ");
                stdio::stdout().flush().unwrap();

                match reader.next_line().await {
                    Ok(Some(line)) if !line.trim().is_empty() => {
                        match parse_command(&line) {
                            TCommand::Quit => {
                                let _ = client.say(channel.to_string(), "I'm out of here!".to_string()).await;
                                let _ = client.part(channel.to_string());
                                let _ = shutdown_tx2.send(());
                                break;
                            }
                            TCommand::Hello => {
                                let _ = client.say(channel.to_string(), "Hello there!".to_string()).await;
                            }
                            TCommand::Unknown(cmd) => {
                                let _ = client.say(channel.to_string(), cmd).await;
                            }
                        }
                    }
                    Ok(Some(_)) => continue,
                    Ok(None) | Err(_) => break,
                }
            }
        });
    }
}

enum TCommand {
    Quit,
    Hello,
    Unknown(String),
}

fn parse_command(line: &str) -> TCommand {
    match line.trim() {
        "!quit" => TCommand::Quit,
        "!hello" => TCommand::Hello,
        other => TCommand::Unknown(other.to_string()),
    }
}

fn format_messages(msg: String) {
    print!("\r\x1b[2k");
    print!("{}", msg);
    print!("\n> ");
    stdio::stdout().flush().unwrap();
}