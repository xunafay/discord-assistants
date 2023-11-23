use std::time::Duration;

use async_openai::types::AssistantTools;
use log::debug;
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteractionDataKind,
    },
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
        CreateSelectMenuOption,
    },
    client::Context,
    utils::CreateQuickModal,
};

use crate::{bot::SplitToVector, openai::OpenAI, tools::available_tools};

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
    for option in options.iter() {
        match option.name.as_str() {
            "image" => {
                let m = command
                    .quick_modal(
                        &ctx,
                        CreateQuickModal::new("Set assistant profile image")
                            .short_field("Id")
                            .short_field("Url"),
                    )
                    .await
                    .expect("Failed to send message");

                match m {
                    Some(res) => {
                        let (id, url) = (&res.inputs[0], &res.inputs[1]);
                        debug!("Id: {}, Url: {}", id, url);
                        let assistants = openai.assistants().await;
                        let assistant = assistants
                            .iter()
                            .find(|assistant| assistant.id == id.clone());

                        match assistant {
                            Some(assistant) => {
                                debug!("trying to aquire read lock");
                                let data_read = ctx.data.read().await;
                                let openai = data_read
                                    .get::<OpenAI>()
                                    .expect("Expected OpenAI in ShareMap");
                                debug!("aquired read lock");
                                openai
                                    .set_assistant_image(assistant, url)
                                    .await
                                    .expect("Failed to set assistant image");

                                let message = CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!(
                                        "Successfully changed {} pfp to {}",
                                        assistant.name.clone().unwrap_or_default(),
                                        url
                                    )),
                                );
                                res.interaction
                                    .create_response(&ctx.http, message)
                                    .await
                                    .expect("Failed to create interaction response");
                            }
                            None => {
                                let message = CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new()
                                        .content("Invalid assistant id"),
                                );
                                command
                                    .create_response(&ctx.http, message)
                                    .await
                                    .expect("Failed to create interaction response");
                                return;
                            }
                        }
                    }
                    None => {
                        let message = CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Assistant creation cancelled"),
                        );
                        command
                            .create_response(&ctx.http, message)
                            .await
                            .expect("Failed to create interaction response");
                        return;
                    }
                }
            }
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
            "tools" => {
                command.defer(&ctx.http).await.expect("Failed to defer");
                let tool = options.first().expect("No subcommand").value.clone();
                let assistant_id = match tool {
                    CommandDataOptionValue::SubCommand(subcommand) => {
                        subcommand.first().expect("No assistant id").value.clone()
                    }
                    _ => {
                        let message = CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("Invalid assistant id"),
                        );
                        command
                            .create_response(&ctx.http, message)
                            .await
                            .expect("Failed to create interaction response");
                        return;
                    }
                };
                let assistant = match assistant_id {
                    CommandDataOptionValue::String(assistant_id) => {
                        debug!("Assistant id: {}", assistant_id);
                        let assistants = openai.assistants().await;
                        let assistant = assistants
                            .iter()
                            .find(|assistant| assistant.id == assistant_id)
                            .expect("Invalid assistant id")
                            .clone();
                        assistant
                    }
                    _ => {
                        let message = CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("Invalid assistant id"),
                        );
                        command
                            .create_response(&ctx.http, message)
                            .await
                            .expect("Failed to create interaction response");
                        return;
                    }
                };

                let m = command
                    .channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().content("Select tools").select_menu(
                            CreateSelectMenu::new(
                                "tool_select",
                                CreateSelectMenuKind::String {
                                    options: available_tools()
                                        .iter()
                                        .map(|tool| {
                                            CreateSelectMenuOption::new(tool.name(), tool.name())
                                                .description(tool.description())
                                        })
                                        .collect::<Vec<CreateSelectMenuOption>>(),
                                },
                            )
                            .min_values(1)
                            .max_values(available_tools().len() as u8),
                        ),
                    )
                    .await
                    .expect("Failed to send message");

                let interaction = match m
                    .await_component_interaction(&ctx.shard)
                    .timeout(Duration::from_secs(60 * 3))
                    .await
                {
                    Some(interaction) => interaction,
                    None => {
                        let message = CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("Timed out"),
                        );
                        command
                            .create_response(&ctx.http, message)
                            .await
                            .expect("Failed to create interaction response");
                        return;
                    }
                };

                let tools = match &interaction.data.kind {
                    ComponentInteractionDataKind::StringSelect { values } => values,
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
                };

                let data_read = ctx.data.read().await;
                let openai = data_read
                    .get::<OpenAI>()
                    .expect("Expected OpenAI in TypeMap");

                let tool_definitions = tools
                    .iter()
                    .map(|tool| {
                        let available_tools = available_tools();
                        let tool = available_tools
                            .iter()
                            .find(|available_tool| &available_tool.name() == tool)
                            .expect("Invalid tool");
                        tool.definition()
                    })
                    .collect::<Vec<AssistantTools>>();

                openai
                    .set_assistant_tools(&assistant, tool_definitions)
                    .await
                    .expect("Failed to set assistant tools");

                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Successfully set tools"),
                        ),
                    )
                    .await
                    .expect("Failed to create interaction response");

            }
            "create" => {
                let m = command
                    .quick_modal(
                        &ctx,
                        CreateQuickModal::new("Assistant details")
                            .short_field("Name")
                            .short_field("Description")
                            .paragraph_field("Instructions"),
                    )
                    .await
                    .expect("Failed to send message");

                match m {
                    Some(res) => {
                        let (name, description, instructions) =
                            (&res.inputs[0], &res.inputs[1], &res.inputs[2]);

                        let message = CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Assistant created! Say hi to {}", name)),
                        );
                        res.interaction
                            .create_response(&ctx.http, message)
                            .await
                            .expect("Failed to create interaction response");
                    }
                    None => {
                        let message = CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Assistant creation cancelled"),
                        );
                        command
                            .create_response(&ctx.http, message)
                            .await
                            .expect("Failed to create interaction response");
                        return;
                    }
                }
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
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "tools",
                "Set assistant tools",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "assistant",
                    "The assistant to set the image for",
                )
                .required(true),
            ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List available assistants",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "create",
            "Create a new assistant",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "image",
            "Set assistant profile image",
        ))
}
