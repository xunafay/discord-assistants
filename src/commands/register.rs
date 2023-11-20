
use serenity::{all::CommandInteraction, client::Context, builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateCommand}};

use crate::database::users::{User, UserStore};

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let data_read = ctx.data.read().await;
    let user_store = data_read
        .get::<UserStore>()
        .expect("Expected UserStore in TypeMap");
    let user_store = user_store.read().await;
    user_store
        .register_user(&User::new(
            command.user.id.get().to_string(),
            command.user.name.clone(),
            command
                .user
                .nick_in(&ctx.http, command.guild_id.expect("No guild"))
                .await,
            None,
        ))
        .expect("Failed to register user");

    let message = CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Registered!"));

    command
        .create_response(&ctx.http, message)
        .await
        .expect("Failed to create interaction response");
}

pub fn register() -> CreateCommand {
    CreateCommand::new("register").description("Register yourself")
}
