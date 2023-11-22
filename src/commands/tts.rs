use async_openai::types::{CreateSpeechResponse, SpeechModel, Voice};
use serenity::{
    all::{CommandDataOptionValue, CommandInteraction, CommandOptionType},
    builder::{
        CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    },
    client::Context,
};

use crate::openai::OpenAI;

async fn generate_voice(
    ctx: &Context,
    prompt: &str,
    voice: Voice,
    quality: SpeechModel,
) -> Result<CreateSpeechResponse, String> {
    let data = ctx.data.read().await;
    let openai = data.get::<OpenAI>().expect("Expected OpenAI in TypeMap");
    let voice = openai.tts(&prompt, voice, quality).await;

    match voice {
        Ok(voice) => Ok(voice),
        Err(_) => Err(format!("Failed to generate voice, sorry")),
    }
}

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let options = &command.data.options;
    let prompt_value = options
        .iter()
        .find(|option| option.name == "text")
        .expect("No text")
        .value
        .clone();

    let model_value = options.iter().find(|option| option.name == "model");

    let model = if let Some(command_option) = model_value {
        if let CommandDataOptionValue::String(model) = command_option.value.clone() {
            match model.as_str() {
                "alloy" => Voice::Alloy,
                "echo" => Voice::Echo,
                "fable" => Voice::Fable,
                "nova" => Voice::Nova,
                "onyx" => Voice::Onyx,
                "shimmer" => Voice::Shimmer,
                _ => Voice::Nova,
            }
        } else {
            Voice::Nova
        }
    } else {
        Voice::Nova
    };

    let quality_value = options.iter().find(|option| option.name == "quality");

    let quality = if let Some(command_option) = quality_value {
        if let CommandDataOptionValue::String(quality) = command_option.value.clone() {
            match quality.as_str() {
                "standard" => SpeechModel::Tts1,
                "hd" => SpeechModel::Tts1Hd,
                _ => SpeechModel::Tts1,
            }
        } else {
            SpeechModel::Tts1
        }
    } else {
        SpeechModel::Tts1
    };

    if let CommandDataOptionValue::String(prompt) = prompt_value {
        command.defer(&ctx.http).await.expect("Failed to defer");
        let voice = generate_voice(&ctx, &prompt, model, quality).await;
        if let Err(_) = voice {
            let message = CreateInteractionResponseFollowup::new()
                .content("Failed to generate images, did you hit the safety filter?");
            command
                .create_followup(&ctx.http, message)
                .await
                .expect("Failed to respond");
            return;
        }

        let file_name = rand::random::<u64>().to_string();
        let file_location = format!("./voice/{}.mp3", file_name);
        voice
            .unwrap()
            .save(&file_location)
            .await
            .expect("Failed to save voice");

        let message = CreateInteractionResponseFollowup::new()
            .content(format!("Prompt: {}", prompt))
            .add_file(
                CreateAttachment::path(file_name)
                    .await
                    .expect("Failed to create attachment"),
            );

        command
            .create_followup(&ctx.http, message)
            .await
            .expect("Failed to respond");
    } else {
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Missing Prompt"),
        );

        command
            .create_response(&ctx.http, response)
            .await
            .expect("Failed to respond");
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("tts")
        .description("Generate speech from text")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "text",
                "The text to generate speech from",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "model", "The model to use")
                .add_string_choice("Alloy", "alloy")
                .add_string_choice("Echo", "echo")
                .add_string_choice("Fable", "fable")
                .add_string_choice("Nova", "nova")
                .add_string_choice("Onyx", "onyx")
                .add_string_choice("Shimmer", "shimmer")
                .required(false),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "quality", "The quality of the speech")
                .add_string_choice("Standard", "standard")
                .add_string_choice("HD", "hd")
                .required(false),
        )
}
