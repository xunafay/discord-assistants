use std::io::Cursor;

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
use serde_json::json;
use serenity::client::Context;

use crate::{
    database::tasks::{
        complete_task, create_task, get_tasks, CompleteTaskToolArgumetens, CreateTaskToolArguments,
        ListTaskToolArguments,
    },
    openai::OpenAI,
    tools::{image::ImageTool, transcribe::TranscribeTool, tts::TtsTool, AlvariumTool},
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
    default_assistant: String,
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
            default_assistant: "asst_12pff2aZ3RLAVAPpAnusFgwV".to_owned(),
        }
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

    pub async fn run(
        &self,
        openai: &OpenAI,
        ctx: &Context,
        assistant: &str,
    ) -> Result<Vec<MessageContent>, String> {
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
                    let tool = required_action
                        .submit_tool_outputs
                        .tool_calls
                        .first()
                        .expect("Failed to get tool call");

                    let args = tool.function.arguments.clone();
                    debug!("Tool: {:?}", tool.function.name);
                    match tool.function.name.as_str() {
                        "user_info" => {
                            let args = serde_json::from_str::<MentionToolArguments>(&args)
                                .expect("Failed to deserialize arguments");

                            let read_data = ctx.data.read().await;
                            let user_store = read_data
                                .get::<crate::database::users::UserStore>()
                                .expect("Expected UserStore in TypeMap");
                            let user_store = user_store.read().await;
                            let output = match user_store.get_user(&args.id) {
                                Some(user) => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some(format!("User: {}", user.get_name())),
                                    }],
                                },
                                None => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some("User not found".to_string()),
                                    }],
                                },
                            };
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "task_create" => {
                            let args = serde_json::from_str::<CreateTaskToolArguments>(&args)
                                .expect("Failed to deserialize arguments");
                            debug!("args: {:?}", args);

                            let output = match create_task(
                                &args.user_id,
                                &args.title,
                                args.description,
                                args.due_date,
                                args.estimated_time,
                            ) {
                                Ok(_) => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some("Task created".to_string()),
                                    }],
                                },
                                Err(err) => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some(format!("Failed to create task: {}", err)),
                                    }],
                                },
                            };
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "task_complete" => {
                            let args = serde_json::from_str::<CompleteTaskToolArgumetens>(&args)
                                .expect("Failed to deserialize arguments");

                            let output = match complete_task(&args.id) {
                                Ok(_) => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some("Task completed".to_string()),
                                    }],
                                },
                                Err(err) => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some(format!("Failed to complete task: {}", err)),
                                    }],
                                },
                            };
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "task_list" => {
                            let args = serde_json::from_str::<ListTaskToolArguments>(&args)
                                .expect("Failed to deserialize arguments");

                            debug!("args: {:?}", args);

                            let output = match get_tasks(&args.user_id) {
                                Ok(tasks) => {
                                    debug!("tasks: {:?}", tasks);

                                    SubmitToolOutputsRunRequest {
                                        tool_outputs: vec![ToolsOutputs {
                                            tool_call_id: Some(tool.id.clone()),
                                            output: Some(format!(
                                                "{:?}",
                                                serde_json::to_string(&tasks)
                                                    .expect("Failed to convert tasks to value")
                                            )),
                                        }],
                                    }
                                }
                                Err(err) => SubmitToolOutputsRunRequest {
                                    tool_outputs: vec![ToolsOutputs {
                                        tool_call_id: Some(tool.id.clone()),
                                        output: Some(format!("Failed to get tasks: {}", err)),
                                    }],
                                },
                            };

                            self.reply_tool_output(&run.id, output).await;
                        }
                        "mention" => {
                            let args = serde_json::from_str::<MentionToolArguments>(&args)
                                .expect("Failed to deserialize arguments");
                            let output = SubmitToolOutputsRunRequest {
                                tool_outputs: vec![ToolsOutputs {
                                    tool_call_id: Some(tool.id.clone()),
                                    output: Some(
                                        serde_json::to_string(&MentionToolResponse {
                                            mention_format: format!("<@!{}>", args.id),
                                        })
                                        .expect("Failed to serialize response"),
                                    ),
                                }],
                            };
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "tts" => {
                            let args = serde_json::from_str::<TtsToolArguments>(&args)
                                .expect("Failed to deserialize arguments");
                            let output = TtsTool::run(args, &ctx, tool).await;
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "transcribe" => {
                            let args = serde_json::from_str::<TranscribeToolArguments>(&args)
                                .expect("Failed to deserialize arguments");
                            let output = TranscribeTool::run(args, &ctx, tool).await;
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "image" => {
                            let args: ImageToolArguments = serde_json::from_str(&args)
                                .expect("Failed to deserialize arguments");
                            let output = ImageTool::run(args, ctx, tool).await;
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "current_date_time" => {
                            let output = SubmitToolOutputsRunRequest {
                                tool_outputs: vec![ToolsOutputs {
                                    tool_call_id: Some(tool.id.clone()),
                                    output: Some(chrono::Local::now().to_string()),
                                }],
                            };
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "web_to_text" => {
                            let args: TranscribeToolArguments = serde_json::from_str(&args)
                                .expect("Failed to deserialize arguments");
                            let client = reqwest::Client::new();
                            let request = client
                                .get(args.url)
                                .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
                                .header(reqwest::header::DNT, 1)
                                .build()
                                .expect("Failed to build request");
                            let response = client.execute(request).await;
                            match response {
                                Ok(response) => {
                                    let html = response.text().await.expect("Failed to get text");
                                    let text_cursor = Cursor::new(html);
                                    let text = html2text::from_read(text_cursor, 9999);
                                    debug!("url: {:?}", text);
                                    let output = SubmitToolOutputsRunRequest {
                                        tool_outputs: vec![ToolsOutputs {
                                            tool_call_id: Some(tool.id.clone()),
                                            output: Some(text),
                                        }],
                                    };
                                    self.reply_tool_output(&run.id, output).await;
                                }
                                Err(err) => {
                                    let output = SubmitToolOutputsRunRequest {
                                        tool_outputs: vec![ToolsOutputs {
                                            tool_call_id: Some(tool.id.clone()),
                                            output: Some(format!("Failed to get url: {:?}", err)),
                                        }],
                                    };
                                    self.reply_tool_output(&run.id, output).await;
                                }
                            }
                        }
                        _ => {
                            let output = SubmitToolOutputsRunRequest {
                                tool_outputs: vec![ToolsOutputs {
                                    tool_call_id: Some(tool.id.clone()),
                                    output: Some(
                                        json!({"error": "tool not implemented or enabled"})
                                            .to_string(),
                                    ),
                                }],
                            };

                            self.reply_tool_output(&run.id, output).await;
                        }
                    }
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
