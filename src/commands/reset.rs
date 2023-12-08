use serenity::{
    all::CommandInteraction,
    builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage},
    client::Context,
};

use crate::database::channels::reset_channel_thread;

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let message = match reset_channel_thread(command.channel_id.get()).await {
        Ok(()) => CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Thread has been reset!"),
        ),
        Err(message) => CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(message),
        ),
    };

    command
        .create_response(&ctx.http, message)
        .await
        .expect("Failed to create interaction response");
}

pub fn register() -> CreateCommand {
    CreateCommand::new("reset").description("Reset the chat history")
}
