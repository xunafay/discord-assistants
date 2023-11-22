use std::collections::HashMap;

use async_openai::types::{
    AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, SubmitToolOutputsRunRequest,
    ToolsOutputs,
};
use serde::Serialize;
use serde_json::json;
use serenity::client::Context;

use crate::openai::OpenAI;

use super::AlvariumTool;

pub struct AssistantListTool;
impl AlvariumTool for AssistantListTool {
    type Arguments = ();
    fn definition() -> AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "".to_string(),
            function: ChatCompletionFunctions {
                name: "assistant_list".to_string(),
                description: Some("List all assistants".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        })
    }

    fn name() -> String {
        "assistant_list".to_owned()
    }

    async fn run(
        _args: Self::Arguments,
        context: &Context,
        tool: &async_openai::types::RunToolCallObject,
    ) -> async_openai::types::SubmitToolOutputsRunRequest {
        let data_read = context.data.read().await;
        let openai = data_read
            .get::<OpenAI>()
            .expect("Expected OpenAI in ShareMap");
        let assistants = openai
            .assistants()
            .await
            .iter()
            .map(|assistant| AssistantVm {
                id: assistant.id.clone(),
                name: assistant.name.clone(),
                description: assistant.description.clone(),
                metadata: assistant.metadata.clone(),
            })
            .collect::<Vec<AssistantVm>>();

        SubmitToolOutputsRunRequest {
            tool_outputs: vec![ToolsOutputs {
                tool_call_id: Some(tool.id.clone()),
                output: Some(
                    serde_json::to_string_pretty(&assistants)
                        .expect("Failed to serialize assistants"),
                ),
            }],
        }
    }
}

#[derive(Debug, Serialize)]
struct AssistantVm {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}
