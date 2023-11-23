use async_openai::types::{
    AssistantTools, RunToolCallObject, SubmitToolOutputsRunRequest, ToolsOutputs, AssistantToolsFunction, ChatCompletionFunctions,
};
use log::debug;
use serde_json::json;

use crate::{openai::OpenAI, thread::TranscribeToolArguments};

use super::AlvariumTool;

pub struct TranscribeTool;
impl AlvariumTool for TranscribeTool {
    type Arguments = TranscribeToolArguments;
    fn name() -> String {
        "transcribe".to_string()
    }

    fn definition() -> AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "function".to_string(),
            function: ChatCompletionFunctions {
                name: "transcribe".to_string(),
                description: Some("Transcribe a video to text".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL of the audio file to transcribe"
                        }
                    },
                    "required": ["url"],
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
    ) -> SubmitToolOutputsRunRequest {
        let data_read = context.data.read().await;
        let openai = data_read
            .get::<OpenAI>()
            .expect("Expected OpenAI in ShareMap");

        let transcript = openai.stt(&args.url).expect("Failed to transcribe");
        debug!("Transcript: {}", transcript);
        SubmitToolOutputsRunRequest {
            tool_outputs: vec![ToolsOutputs {
                tool_call_id: Some(tool.id.clone()),
                output: Some(json!({"transcript": transcript}).to_string()),
            }],
        }
    }
}
