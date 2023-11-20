use async_openai::{types::{RunToolCallObject, SubmitToolOutputsRunRequest, ImageModel, ImageQuality, ImageStyle, Image, ToolsOutputs}, error::OpenAIError};
use log::{debug, error};

use crate::openai::{OpenAI, ImageToolArguments};

pub async fn image_tool(
    args: String,
    openai: &OpenAI,
    tool: &RunToolCallObject,
) -> SubmitToolOutputsRunRequest {
    let args: ImageToolArguments =
        serde_json::from_str(&args).expect("Failed to deserialize arguments");

    let model = match args.model {
        Some(text) => match text.as_str() {
            "dall-e-3" => ImageModel::DallE3,
            "dall-e-2" => ImageModel::DallE2,
            _ => ImageModel::DallE3,
        },
        None => ImageModel::DallE3,
    };

    let quality = match args.quality {
        Some(text) => match text.as_str() {
            "hd" => ImageQuality::HD,
            "standard" => ImageQuality::Standard,
            _ => ImageQuality::Standard,
        },
        None => ImageQuality::Standard,
    };

    let style = match args.style {
        Some(text) => match text.as_str() {
            "natural" => ImageStyle::Natural,
            "vivid" => ImageStyle::Vivid,
            _ => ImageStyle::Natural,
        },
        None => ImageStyle::Natural,
    };

    debug!("calling generate image with options: prompt: {:?}, model: {:?}, quality: {:?}, style: {:?}", args.prompt, model, quality, style);

    let images = openai
        .generate_image(&args.prompt, Some(model), quality, style)
        .await;

    match images {
        Ok(images) => {
            let image_urls: Vec<String> = images
                .data
                .iter()
                .map(|image| match &**image {
                    Image::Url {
                        url,
                        revised_prompt: _,
                    } => Some(url.clone()),
                    _ => None,
                })
                .filter_map(|x| x)
                .collect();

            SubmitToolOutputsRunRequest {
                tool_outputs: vec![ToolsOutputs {
                    tool_call_id: Some(tool.id.clone()),
                    output: Some(image_urls.join("\n")),
                }],
            }
        }
        Err(err) => {
            let message = match err {
                OpenAIError::ApiError(error) => error.message,
                _ => "Failed to generate image, sorry".to_string(),
            };

            error!("Failed to generate image, sorry");
            SubmitToolOutputsRunRequest {
                tool_outputs: vec![ToolsOutputs {
                    tool_call_id: Some(tool.id.clone()),
                    output: Some(message),
                }],
            }
        }
    }
}

