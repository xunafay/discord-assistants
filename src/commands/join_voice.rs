use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{
        CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    prelude::Context,
};

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let channel = command
        .data
        .options
        .iter()
        .find(|option| option.name == "channel")
        .expect("No channel")
        .value
        .clone();

    match channel {
        CommandDataOptionValue::Channel(channel) => {
            let manager = songbird::get(ctx)
                .await
                .expect("Songbird Voice client placed in at initialisation.")
                .clone();

            manager
                .join(command.guild_id.expect(""), channel)
                .await
                .expect("Failed to join channel");

            let message = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Joined!"),
            );
            command
                .create_response(&ctx.http, message)
                .await
                .expect("Failed to create interaction response")
        }
        _ => {
            let message = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Invalid channel"),
            );
            command
                .create_response(&ctx.http, message)
                .await
                .expect("Failed to create interaction response");
            return;
        }
    };
}

pub fn register() -> CreateCommand {
    CreateCommand::new("voice")
        .description("Join a voice channel")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Channel, "channel", "The channel to join")
                .required(false),
        )
}
