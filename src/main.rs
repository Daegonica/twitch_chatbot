use twitch_bot::TwitchBot;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bot = TwitchBot::new().await?;

    bot.run().await
}