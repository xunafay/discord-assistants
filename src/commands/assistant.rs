use serenity::{
    all::{CommandInteraction, CommandOptionType},
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage,
    },
    client::Context,
};

use crate::{bot::SplitToVector, openai::OpenAI};

fn clip_instructions(instructions: String) -> String {
    let parts = instructions.split_to_vector(500);
    if parts.len() > 1 {
        let mut clipped = parts.first().unwrap().clone();
        clipped.push_str("...");
        clipped
    } else {
        instructions
    }
}

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let data = ctx.data.read().await;
    let openai = data.get::<OpenAI>().expect("Expected OpenAI in TypeMap");

    let options = &command.data.options;
    for subcommand in options.iter() {
        match subcommand.name.as_str() {
            "list" => {
                let query = [("limit", "10")];
                let assistants = openai
                    .client
                    .assistants()
                    .list(&query)
                    .await
                    .expect("Failed to list assistants");
                let embeds = assistants
                    .data
                    .iter()
                    .map(|assistant| {
                        CreateEmbed::new()
                            .field(
                                "Name",
                                assistant.name.clone().unwrap_or("None".to_string()),
                                true,
                            )
                            .field("Id", assistant.id.as_str(), true)
                            .field("Model", assistant.model.clone(), true)
                            .field(
                                "Instructions",
                                clip_instructions(
                                    assistant.instructions.clone().unwrap_or("None".to_string()),
                                ),
                                false,
                            )
                    })
                    .collect::<Vec<CreateEmbed>>();
                let message = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().add_embeds(embeds),
                );
                command
                    .create_response(&ctx.http, message)
                    .await
                    .expect("Failed to create interaction response");
                return;
            }
            _ => {
                let message = CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("Invalid subcommand"),
                );
                command
                    .create_response(&ctx.http, message)
                    .await
                    .expect("Failed to create interaction response");
                return;
            }
        }
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("assistant")
        .description("Manage your assistants")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "default",
                "Set your default assistant",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "assistant",
                    "The assistant to set as default",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List available assistants",
        ))
}
