use async_openai::{
    error::OpenAIError,
    types::{
        AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, Image, ImageModel,
        ImageQuality, ImageStyle, RunToolCallObject, SubmitToolOutputsRunRequest, ToolsOutputs,
    },
};
use log::{error, debug};
use serde_json::json;

use crate::{openai::{ImageToolArguments, OpenAI}, database::image::Minio};

use super::AlvariumTool;

pub struct ImageTool;
impl ImageTool {
    pub fn new() -> Self {
        Self
    }
}

impl AlvariumTool for ImageTool {
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

    fn name(&self) -> String {
        "image".to_owned()
    }

    async fn run(
        &mut self,
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

        let images = images
            .expect("Failed to generate image")
            .save("./images")
            .await;

        match images {
            Ok(images) => {
                let mut image_urls: Vec<String> = vec![];
                let minio = Minio::new().await;
                for image in images {
                    let res = minio
                        .upload_image(image.to_str().unwrap())
                        .await
                        .expect("Failed to upload image");
                    image_urls.push(res);
                    tokio::fs::remove_file(image).await.expect("Failed to remove image");
                }

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
}
