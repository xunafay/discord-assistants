use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        CreateImageRequestArgs, CreateMessageRequestArgs, CreateRunRequestArgs,
        CreateSpeechRequestArgs, CreateSpeechResponse, CreateThreadRequestArgs,
        CreateTranscriptionRequestArgs, CreateTranscriptionResponse, ImageModel, ImageQuality,
        ImageSize, ImageStyle, ImagesResponse, MessageContent, ResponseFormat, RunStatus,
        SpeechModel, SubmitToolOutputsRunRequest, ToolsOutputs, Voice,
    },
    Client,
};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use serenity::{all::Webhook, client::Context};
use std::io::Cursor;
use std::{
    collections::HashMap,
    process::{Command, Stdio},
};

use crate::{
    database::tasks::{
        complete_task, create_task, get_tasks, CompleteTaskToolArgumetens, CreateTaskToolArguments,
        ListTaskToolArguments,
    },
    tools::{transcribe::transcribe_tool, tts::tts_tool, image::ImageTool, AlvariumTool},
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
                            let output = tts_tool(&args, openai, tool).await;
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "transcribe" => {
                            let output = transcribe_tool(&args, openai, tool);
                            self.reply_tool_output(&run.id, output).await;
                        }
                        "image" => {
                            let output = ImageTool::new().run(args, openai, tool).await;
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
                                    output: Some("tool not implemented".to_string()),
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

#[derive(Debug)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub webhook: String,
    pub voice: Voice,
    pub author_id: String,
}

pub struct AssistantStore {
    channels: HashMap<u64, Vec<Assistant>>,
}

impl AssistantStore {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        map.insert(1089997843361693860, vec![
            Assistant {
                name: "lovelace".to_owned(),
                webhook: "https://discord.com/api/webhooks/1176163600608530512/FQ8et-KbqLrokXzEOQbObtKK9WkQIyiYmprB3kk-MS2d9AHpddul9xmszLoJ0Rs4zvi-".to_owned(),
                id: "asst_P66RVsW92Izpwky1qWDAZMO8".to_owned(),
                voice: Voice::Nova,
                author_id: "1176163600608530512".to_string()
            },
            Assistant {
                name: "mosscap".to_owned(),
                webhook: "https://discord.com/api/webhooks/1176163666345857034/YH4_E6AW7m1IZQ9sHDmBBiuFBpZJejMcSn9Po-cKnhjUWTdxTeX-WWoe4-jC0rAphwId".to_owned(),
                id: "asst_12pff2aZ3RLAVAPpAnusFgwV".to_owned(),
                voice: Voice::Onyx,
                author_id: "1176163666345857034".to_string()
            },
        ]);

        map.insert(1089999946859683911, vec![
            Assistant {
                name: "lovelace".to_owned(),
                webhook: "https://discord.com/api/webhooks/1176166728057761862/WX11YPMn91Rp1ugwvpZYfCwh6Lh-izLc4aX7xs1SwRD--m_tDXyjnUGspw1EbdbonxJ4".to_owned(),
                id: "asst_P66RVsW92Izpwky1qWDAZMO8".to_owned(),
                voice: Voice::Nova,
                author_id: "1176166728057761862".to_string()
            },
            Assistant {
                name: "mosscap".to_owned(),
                webhook: "https://discord.com/api/webhooks/1176166819246125066/5CtG8wcGrytSwtaKWbAOHV3uAvy1fgtgcMD9KY4bJW1aaDaQhVYL448YmJNRpXQR4F8s".to_owned(),
                id: "asst_12pff2aZ3RLAVAPpAnusFgwV".to_owned(),
                voice: Voice::Onyx,
                author_id: "1176166819246125066".to_string()
            },
        ]);
        AssistantStore { channels: map }
    }

    pub fn get_channel(&self, channel_id: &u64) -> Option<&Vec<Assistant>> {
        self.channels.get(channel_id)
    }
}

pub struct ThreadStore {
    threads: HashMap<u64, OpenAIThread>,
}

impl ThreadStore {
    pub fn new() -> Self {
        ThreadStore {
            threads: HashMap::new(),
        }
    }

    pub async fn thread(&mut self, id: &u64) -> &OpenAIThread {
        if self.threads.contains_key(&id) {
            return self.threads.get(&id).unwrap();
        } else {
            let thread = OpenAIThread::new().await;
            self.threads.insert(id.clone(), thread);
            return self.threads.get(&id).unwrap();
        }
    }
}

#[derive(Clone)]
pub struct OpenAI {
    pub client: Client<OpenAIConfig>,
}

impl OpenAI {
    pub fn new() -> Self {
        let client: Client<OpenAIConfig> = Client::new();
        OpenAI { client }
    }

    pub async fn generate_image(
        &self,
        prompt: &str,
        model: Option<ImageModel>,
        quality: ImageQuality,
        style: ImageStyle,
    ) -> Result<ImagesResponse, OpenAIError> {
        let model = model.unwrap_or(ImageModel::DallE3);

        let request = CreateImageRequestArgs::default()
            .model(model)
            .prompt(prompt)
            .n(1)
            .style(style)
            .response_format(ResponseFormat::B64Json)
            .quality(quality)
            .size(ImageSize::S1024x1024)
            .user("async-openai")
            .build()
            .expect("Failed to build request");

        self.client.images().create(request).await
    }

    pub async fn tts(
        &self,
        prompt: &str,
        voice: Voice,
        quality: SpeechModel,
    ) -> Result<CreateSpeechResponse, OpenAIError> {
        let request = CreateSpeechRequestArgs::default()
            .input(prompt)
            .voice(voice)
            .model(quality)
            .build()
            .expect("Failed to build request");

        self.client.audio().speech(request).await
    }

    // using it creates a compilation error https://github.com/64bit/async-openai/issues/140
    #[allow(dead_code)]
    async fn stt_broken(&self, file: &str) -> Result<CreateTranscriptionResponse, OpenAIError> {
        let request = CreateTranscriptionRequestArgs::default()
            .file(file)
            .model("whisper-1")
            .build()
            .expect("Failed to build request");

        self.client.audio().transcribe(request).await
    }

    pub fn stt(&self, url: &str) -> Result<String, String> {
        let file_name = rand::random::<u64>().to_string();
        debug!("starting yt-dlp");
        let file = Command::new("yt-dlp")
            .arg("--no-check-certificate") // TODO dirty fix for self signed cert
            .arg("-f")
            .arg("bestaudio")
            .arg("-o")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .arg(format!("{file_name}.webm"))
            .arg(url)
            .output()
            .expect("failed to execute process");

        if !file.status.success() {
            error!("yt-dlp failed: {:?}", file);
            return Err(format!("yt-dlp failed: {:?}", file));
        }

        debug!("starting whisper");
        let transcript = Command::new("venv/bin/python")
            .arg("whisper.py")
            .arg("--file")
            .arg(format!("{file_name}.webm"))
            .output()
            .expect("failed to execute process");

        if !transcript.status.success() {
            error!("whisper failed: {:?}", transcript);
            return Err(format!("whisper failed: {:?}", transcript));
        }

        let result = String::from_utf8_lossy(&transcript.stdout);
        Ok(result.to_string())
    }
}
