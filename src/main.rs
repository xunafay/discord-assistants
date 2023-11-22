mod bot;
mod commands;
mod database;
mod openai;
mod thread;
mod tools;

use bot::Bot;
use std::env;
use tokio::{self};

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_module("cognicompany", log::LevelFilter::Debug)
        .init();

    // Load the Discord token from the environment variables
    let discord_token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create a new instance of the bot
    let bot = Bot::new();
    bot.start(&discord_token).await;

    // run until ctrl-c is pressed
    tokio::signal::ctrl_c().await.unwrap();
}
