use async_openai::error::OpenAIError;
use async_openai::types::{ImageModel, ImageQuality, ImageStyle, ImagesResponse};
use serenity::all::{CommandDataOptionValue, CommandInteraction, CommandOptionType};
use serenity::builder::{
    CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};
use serenity::client::Context;

use crate::openai::OpenAI;

async fn generate_image(
    ctx: &Context,
    prompt: &str,
    model: Option<ImageModel>,
    quality: ImageQuality,
    style: ImageStyle,
) -> Result<ImagesResponse, OpenAIError> {
    let data = ctx.data.read().await;
    let openai = data.get::<OpenAI>().expect("Expected OpenAI in TypeMap");
    openai.generate_image(&prompt, model, quality, style).await
}

pub async fn run(ctx: &Context, command: &CommandInteraction) {
    let options = &command.data.options;
    let prompt_value = options
        .iter()
        .find(|option| option.name == "prompt")
        .expect("No prompt")
        .value
        .clone();

    let model_value = options.iter().find(|option| option.name == "model");

    let model = if let Some(command_option) = model_value {
        if let CommandDataOptionValue::String(model) = command_option.value.clone() {
            match model.as_str() {
                "dall-e-3" => ImageModel::DallE3,
                "dall-e-2" => ImageModel::DallE2,
                _ => ImageModel::DallE3,
            }
        } else {
            ImageModel::DallE3
        }
    } else {
        ImageModel::DallE3
    };

    let style_value = options.iter().find(|option| option.name == "style");

    let style = if let Some(command_option) = style_value {
        if let CommandDataOptionValue::String(style) = command_option.value.clone() {
            match style.as_str() {
                "natural" => ImageStyle::Natural,
                "vivid" => ImageStyle::Vivid,
                _ => ImageStyle::Natural,
            }
        } else {
            ImageStyle::Natural
        }
    } else {
        ImageStyle::Natural
    };

    let quality_value = options.iter().find(|option| option.name == "quality");

    let quality = if let Some(command_option) = quality_value {
        if let CommandDataOptionValue::String(quality) = command_option.value.clone() {
            match quality.as_str() {
                "standard" => ImageQuality::Standard,
                "hd" => ImageQuality::HD,
                _ => ImageQuality::Standard,
            }
        } else {
            ImageQuality::Standard
        }
    } else {
        ImageQuality::Standard
    };

    if let CommandDataOptionValue::String(prompt) = prompt_value {
        command.defer(&ctx.http).await.expect("Failed to defer");
        let images = generate_image(&ctx, &prompt, Some(model), quality, style).await;
        if let Err(error) = images {
            let error_message = match error {
                OpenAIError::ApiError(error) => error.message,
                _ => "Failed to generate image, sorry".to_string(),
            };
            let message = CreateInteractionResponseFollowup::new().content(error_message);

            command
                .create_followup(&ctx.http, message)
                .await
                .expect("Failed to respond");
            return;
        }

        let images = images
            .expect("Failed to generate images")
            .save("./images")
            .await
            .expect("Failed to save image")
            .iter()
            .map(|image| image.display().to_string())
            .collect::<Vec<String>>();

        let mut attachments = vec![];
        for image in images {
            let attachment = CreateAttachment::path(&image)
                .await
                .expect("Failed to create attachment");
            attachments.push(attachment);
        }
        let followup = CreateInteractionResponseFollowup::new()
            .content(format!("Prompt: {}", prompt))
            .add_files(attachments);

        command
            .create_followup(&ctx.http, followup)
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
    CreateCommand::new("image")
        .description("Generate an image")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "prompt", "Describe the image")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "model", "The model to use")
                .add_string_choice("DALL·E 3", "dall-e-3")
                .add_string_choice("DALL·E 2", "dall-e-2")
                .kind(CommandOptionType::String)
                .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "style",
                "The style to use (only applies to DALL·E 3)",
            )
            .add_string_choice("Natural", "natural")
            .add_string_choice("Vivid", "vivid")
            .kind(CommandOptionType::String)
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "quality",
                "The quality of the image",
            )
            .add_string_choice("Standard", "standard")
            .add_string_choice("HD", "hd")
            .kind(CommandOptionType::String)
            .required(false),
        )
}
