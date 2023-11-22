use async_openai::types::{
    AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, SubmitToolOutputsRunRequest, ToolsOutputs, RunToolCallObject,
};
use serde_json::json;
use serenity::client::Context;

use super::AlvariumTool;

pub struct AssistantCreateTool;
impl AlvariumTool for AssistantCreateTool {
    type Arguments = ();
    fn definition() -> AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "function".to_string(),
            function: ChatCompletionFunctions {
                name: "assistant_create".to_string(),
                description: Some("Create a new assistant".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "The name of the assistant"
                        },
                        "model": {
                            "type": "string",
                            "enum": [
                                "gpt-3.5-turbo-1106",
                                "gpt-4-1106-preview"
                            ],
                            "description": "The model to use for the assistant"
                        },
                        "instructions": {
                            "type": "string",
                            "description": "The instructions for the assistant"
                        }
                    },
                }),
            },
        })
    }

    fn name() -> String {
        "assistant_create".to_owned()
    }

    async fn run(
        _args: Self::Arguments,
        _context: &Context,
        tool: &RunToolCallObject,
    ) -> SubmitToolOutputsRunRequest {
        SubmitToolOutputsRunRequest {
            tool_outputs: vec![ToolsOutputs {
                tool_call_id: Some(tool.id.clone()),
                output: Some(json!({"error": "Tool not implemented"}).to_string()),
            }],
        }
    }
}
