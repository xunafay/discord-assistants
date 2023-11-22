use async_openai::{
    error::OpenAIError,
    types::{
        AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, Image, ImageModel,
        ImageQuality, ImageStyle, RunToolCallObject, SubmitToolOutputsRunRequest, ToolsOutputs,
    },
};
use log::{debug, error};
use serde_json::json;
use serenity::client::Context;

use crate::{database::blob::Minio, openai::OpenAI, thread::ImageToolArguments};

use super::AlvariumTool;

pub struct ImageTool;
impl AlvariumTool for ImageTool {
    type Arguments = ImageToolArguments;
    fn definition() -> AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "function".to_string(),
            function: ChatCompletionFunctions {
                name: "image".to_string(),
                description: Some("Generate an image from a prompt".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "prompt": {
                            "type": "string",
                            "description": "The prompt for the image generation model"
                        },
                        "model": {
                            "type": "string",
                            "enum": [
                                "dall-e-3",
                                "dall-e-2"
                            ],
                            "description": "The model to use for image generation"
                        },
                        "style": {
                            "type": "string",
                            "enum": [
                                "natural",
                                "vivid"
                            ],
                            "description": "The style to use for image generation, only applies to dall-e-3"
                        },
                        "quality": {
                            "type": "string",
                            "enum": [
                                "standard",
                                "hd"
                            ],
                            "description": "The quality of the image, only applies to dall-e-3"
                        }
                    },
                    "required": [
                        "prompt"
                    ]
                }),
            },
        })
    }

    fn name() -> String {
        "image".to_owned()
    }

    async fn run(
        args: Self::Arguments,
        context: &Context,
        tool: &RunToolCallObject,
    ) -> SubmitToolOutputsRunRequest {
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

        let images = {
            let data = context.data.read().await;
            let openai = data.get::<OpenAI>().expect("Expected OpenAI in TypeMap");
            openai
                .generate_image(&args.prompt, Some(model), quality, style)
                .await
        };

        let images = images
            .expect("Failed to generate image")
            .save("./images")
            .await;

        match images {
            Ok(images) => {
                let mut image_urls: Vec<String> = vec![];
                let minio = Minio::new();
                for image in images {
                    let res = minio
                        .upload_image(image.to_str().unwrap())
                        .await
                        .expect("Failed to upload image");
                    image_urls.push(res);
                    tokio::fs::remove_file(image)
                        .await
                        .expect("Failed to remove image");
                }

                SubmitToolOutputsRunRequest {
                    tool_outputs: vec![ToolsOutputs {
                        tool_call_id: Some(tool.id.clone()),
                        output: Some(json!({"urls": image_urls}).to_string()),
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
                        output: Some(json!({"error": message}).to_string()),
                    }],
                }
            }
        }
    }
}
