use async_openai::{
    config::OpenAIConfig,
    types::{
        CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, MessageContent,
        RunStatus, SubmitToolOutputsRunRequest, ToolsOutputs,
    },
    Client,
};
use log::debug;
use serde::{Deserialize, Serialize};
use serenity::client::Context;

use crate::tools::{
    assistant_list::AssistantListTool, available_tools, datetime::DateTimeTool, image::ImageTool,
    transcribe::TranscribeTool, tts::TtsTool, AlvariumTool, Tools,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeToolArguments {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionToolArguments {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionToolResponse {
    pub mention_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageToolArguments {
    pub prompt: String,
    pub model: Option<String>,
    pub quality: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsToolArguments {
    pub content: String,
    pub voice: Option<String>,
}

pub struct OpenAIThread {
    thread_id: String,
    client: Client<OpenAIConfig>,
}

impl OpenAIThread {
    pub async fn new() -> Self {
        let client: Client<OpenAIConfig> = Client::new();

        let thread_request = CreateThreadRequestArgs::default()
            .build()
            .expect("Failed to build request");
        let thread = client
            .threads()
            .create(thread_request)
            .await
            .expect("Failed to create thread");

        OpenAIThread {
            thread_id: thread.id,
            client,
        }
    }

    pub fn from_existing(thread_id: &str) -> Self {
        let client: Client<OpenAIConfig> = Client::new();

        OpenAIThread {
            thread_id: thread_id.to_owned(),
            client,
        }
    }

    pub fn id(&self) -> &str {
        &self.thread_id
    }

    pub async fn add_message(&self, message: String) {
        debug!("Adding message: {}", message);
        let message = CreateMessageRequestArgs::default()
            .role("user")
            .content(message)
            .build()
            .expect("Failed to build request");

        self.client
            .threads()
            .messages(&self.thread_id)
            .create(message)
            .await
            .expect("Failed to create message");
    }

    pub async fn run(&self, ctx: &Context, assistant: &str) -> Result<Vec<MessageContent>, String> {
        debug!("Running thread {}", self.thread_id);
        let run_request = CreateRunRequestArgs::default()
            .assistant_id(assistant)
            .build()
            .expect("Failed to build request");

        let run = self
            .client
            .threads()
            .runs(&self.thread_id)
            .create(run_request)
            .await
            .expect("Failed to create run");

        loop {
            let run = self
                .client
                .threads()
                .runs(&self.thread_id)
                .retrieve(&run.id)
                .await
                .expect("Failed to retrieve run");

            match run.status {
                RunStatus::Cancelled => return Err("Run was cancelled".to_string()),
                RunStatus::Cancelling => debug!("Run is cancelling"),
                RunStatus::Failed => return Err("Run failed".to_string()),
                RunStatus::Completed => return Ok(self.get_latest_message().await),
                RunStatus::Expired => return Err("Run expired".to_string()),
                RunStatus::InProgress => debug!("Run is in progress"),
                RunStatus::Queued => debug!("Run is queued"),
                RunStatus::RequiresAction => {
                    let required_action =
                        run.required_action.expect("Failed to get required action");

                    let mut outputs: Vec<ToolsOutputs> = vec![];
                    // TODO should convert to asynchrounously run tools
                    for tool_request in required_action.submit_tool_outputs.tool_calls {
                        let args = tool_request.function.arguments.clone();
                        debug!("Tool: {:?}", tool_request.function.name);

                        let tools = available_tools();
                        let tool = tools
                            .iter()
                            .find(|tool| tool.name() == tool_request.function.name.as_str())
                            .expect("Failed to find tool");

                        match tool {
                            Tools::AssistantList => {
                                let output = AssistantListTool::run((), ctx, &tool_request).await;
                                outputs.push(output);
                            }
                            Tools::DateTime => {
                                let output = DateTimeTool::run((), ctx, &tool_request).await;
                                outputs.push(output);
                            }
                            Tools::Tts => {
                                let args = serde_json::from_str::<TtsToolArguments>(&args)
                                    .expect("Failed to deserialize arguments");
                                let output = TtsTool::run(args, ctx, &tool_request).await;
                                outputs.push(output);
                            }
                            Tools::Transcribe => {
                                let args = serde_json::from_str::<TranscribeToolArguments>(&args)
                                    .expect("Failed to deserialize arguments");
                                let output = TranscribeTool::run(args, ctx, &tool_request).await;
                                outputs.push(output);
                            }
                            Tools::Image => {
                                let args = serde_json::from_str::<ImageToolArguments>(&args)
                                    .expect("Failed to deserialize arguments");
                                let output = ImageTool::run(args, ctx, &tool_request).await;
                                outputs.push(output);
                            }
                        }
                    }

                    self.reply_tool_output(
                        &run.id,
                        SubmitToolOutputsRunRequest {
                            tool_outputs: outputs,
                        },
                    )
                    .await;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    async fn reply_tool_output(&self, run_id: &str, output: SubmitToolOutputsRunRequest) {
        self.client
            .threads()
            .runs(&self.thread_id)
            .submit_tool_outputs(run_id, output)
            .await
            .expect("Failed to submit tool outputs");
    }

    async fn get_latest_message(&self) -> Vec<MessageContent> {
        let query = [("limit", "1")];
        let response = self
            .client
            .threads()
            .messages(&self.thread_id)
            .list(&query)
            .await
            .expect("Failed to list messages");

        let message_id = response
            .data
            .get(0)
            .expect("Failed to get message id")
            .id
            .clone();

        let message = self
            .client
            .threads()
            .messages(&self.thread_id)
            .retrieve(&message_id)
            .await
            .expect("Failed to retrieve message");

        message.content
    }

    #[allow(dead_code)]
    pub fn get_messages(&self) -> Vec<String> {
        todo!()
    }
}
