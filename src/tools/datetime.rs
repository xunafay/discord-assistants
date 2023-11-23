use async_openai::types::{
    AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, RunToolCallObject,
    SubmitToolOutputsRunRequest, ToolsOutputs,
};
use serde_json::json;
use serenity::client::Context;

use super::AlvariumTool;

pub struct DateTimeTool;
impl AlvariumTool for DateTimeTool {
    type Arguments = ();
    fn definition() -> AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "function".to_string(),
            function: ChatCompletionFunctions {
                name: "datetime".to_string(),
                description: Some("Get the current date and time".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        })
    }

    fn name() -> String {
        "datetime".to_owned()
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
        _args: Self::Arguments,
        _context: &Context,
        tool: &RunToolCallObject,
    ) -> ToolsOutputs {
        let now = chrono::Local::now(); // support time zones in the future
        let day = now.format("%A").to_string();
        ToolsOutputs {
            tool_call_id: Some(tool.id.clone()),
            output: Some(json!({ "datetime": now.to_string(), "day": day }).to_string()),
        }
    }
}
