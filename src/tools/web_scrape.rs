use async_openai::types::{
    AssistantTools, AssistantToolsFunction, ChatCompletionFunctions, RunToolCallObject,
    ToolsOutputs,
};
use serde_json::json;
use serenity::client::Context;

use super::AlvariumTool;

pub struct WebScrapeTool;

impl AlvariumTool for WebScrapeTool {
    type Arguments = ();

    fn definition() -> AssistantTools {
        AssistantTools::Function(AssistantToolsFunction {
            r#type: "function".to_string(),
            function: ChatCompletionFunctions {
                name: "web_scrape".to_string(),
                description: Some(
                    "Scrape a web page and get its content in plain text".to_string(),
                ),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL of the web page to scrape"
                        }
                    },
                    "required": ["url"]
                }),
            },
        })
    }

    fn name() -> String {
        "web_scrape".to_owned()
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
        ToolsOutputs {
            tool_call_id: Some(tool.id.clone()),
            output: Some(json!({"error": "Tool not implemented"}).to_string()),
        }
    }
}
