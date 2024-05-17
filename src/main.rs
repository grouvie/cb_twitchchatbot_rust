#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use dotenv::dotenv;

use cb_twitchchatbot_rust::chat_bot::ChatBot;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let nickname = std::env::var("NICKNAME").expect("NICKNAME env var not set");
    let oauth_token = std::env::var("OAUTH_TOKEN").expect("OAUTH_TOKEN env var not set");
    let channel = std::env::var("CHANNEL").expect("CHANNEL env var not set");
    let file_path = std::env::var("FILEPATH").expect("FILEPATH env var not set");

    let mut bot = ChatBot::new(nickname, oauth_token, channel, file_path);

    bot.run().await;
}
