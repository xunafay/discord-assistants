use async_openai::types::{
    AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, RunToolCallObject,
    SpeechModel, SubmitToolOutputsRunRequest, ToolsOutputs, Voice,
};
use log::debug;
use serde_json::json;

use crate::{database::blob::Minio, openai::OpenAI, thread::TtsToolArguments};

use super::AlvariumTool;

pub struct TtsTool;
impl AlvariumTool for TtsTool {
    type Arguments = TtsToolArguments;
    fn name() -> String {
        "tts".to_string()
    }

    fn definition() -> async_openai::types::AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "function".to_string(),
            function: ChatCompletionFunctions {
                name: "tts".to_string(),
                description: Some("Generate a voice from a prompt".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The text to synthesize"
                        },
                        "voice": {
                            "type": "string",
                            "enum": [
                                "alloy",
                                "echo",
                                "fable",
                                "nova",
                                "onyx",
                                "shimmer"
                            ],
                            "description": "The voice to use"
                        }
                    },
                    "required": ["content"],
                }),
            },
        })
    }

    fn description() -> String {
        match Self::definition() {
            AssistantTools::Function(AssistantToolsFunction { function, .. }) => {
                function.description.unwrap_or_default()
            }
            _ => "".to_owned(),
        }
    }

    async fn run(
        args: Self::Arguments,
        context: &serenity::prelude::Context,
        tool: &RunToolCallObject,
    ) -> ToolsOutputs {
        let voice = match args.voice {
            Some(text) => match text.as_str() {
                "alloy" => Voice::Alloy,
                "echo" => Voice::Echo,
                "fable" => Voice::Fable,
                "nova" => Voice::Nova,
                "onyx" => Voice::Onyx,
                "shimmer" => Voice::Shimmer,
                _ => Voice::Nova,
            },
            None => Voice::Nova,
        };

        let quality = SpeechModel::Tts1;

        debug!(
            "calling tts with options: content: {:?}, voice: {:?}, quality: {:?}",
            args.content, voice, quality
        );

        let data_read = context.data.read().await;
        let openai = data_read
            .get::<OpenAI>()
            .expect("Expected OpenAI in ShareMap");

        let result = openai
            .tts(&args.content, voice, quality)
            .await
            .expect("Failed to generate voice");

        let file_name = rand::random::<u64>().to_string();
        let file_location = format!("./voice/{}.mp3", file_name);

        result
            .save(&file_location)
            .await
            .expect("Failed to save voice");

        let url = Minio::new()
            .upload_mp3(&file_location)
            .await
            .expect("Failed to upload voice");

        ToolsOutputs {
            tool_call_id: Some(tool.id.clone()),
            output: Some(json!({"url": url}).to_string()),
        }
    }
}
