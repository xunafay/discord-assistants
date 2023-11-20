mod bot;
mod openai;
mod commands;
mod tools;
mod database;

use bot::Bot;
use std::env;
use tokio;

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_module("lovelace", log::LevelFilter::Debug)
        .init();

    // Load the Discord token from the environment variables
    let discord_token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create a new instance of the bot
    let bot = Bot::new();
    bot.start(&discord_token).await;

    // run until ctrl-c is pressed
    tokio::signal::ctrl_c().await.unwrap();
}
