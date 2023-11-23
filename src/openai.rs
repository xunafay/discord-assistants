use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        AssistantObject, AssistantTools, CreateImageRequestArgs, CreateSpeechRequestArgs,
        CreateSpeechResponse, CreateTranscriptionRequestArgs, CreateTranscriptionResponse,
        ImageModel, ImageQuality, ImageSize, ImageStyle, ImagesResponse, ModifyAssistantRequest,
        ResponseFormat, SpeechModel, Voice,
    },
    Client,
};
use log::{debug, error};
use serde_json::json;
use serenity::client::Context;
use std::{
    collections::HashMap,
    fmt,
    process::{Command, Stdio},
};

use crate::thread::OpenAIThread;

#[derive(Debug)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub webhook: String,
    pub voice: Voice,
    pub author_id: String,
}

pub struct ThreadStore {
    threads: HashMap<String, OpenAIThread>,
}

impl ThreadStore {
    pub fn new() -> Self {
        ThreadStore {
            threads: HashMap::new(),
        }
    }

    pub fn get(
        &mut self,
        id: &str
    ) -> Option<&OpenAIThread> {
        self.threads.get(id)
    }

    pub fn add_thread(&mut self, thread: OpenAIThread) {
        debug!("registring new thread in store");
        self.threads.insert(thread.id().to_owned(), thread);
    }
}

#[derive(Clone)]
pub struct OpenAI {
    pub client: Client<OpenAIConfig>,
}

fn voice_to_string(voice: &Voice) -> String {
    match voice {
        Voice::Alloy => "alloy".to_owned(),
        Voice::Echo => "echo".to_owned(),
        Voice::Fable => "fable".to_owned(),
        Voice::Nova => "nova".to_owned(),
        Voice::Onyx => "onyx".to_owned(),
        Voice::Shimmer => "shimmer".to_owned(),
        _ => "nova".to_owned(),
    }
}

impl OpenAI {
    pub fn new() -> Self {
        let client: Client<OpenAIConfig> = Client::new();
        OpenAI { client }
    }

    pub async fn assistants(&self) -> Vec<AssistantObject> {
        let query = [("limit", "100")];
        let response = self
            .client
            .assistants()
            .list(&query)
            .await
            .expect("Failed to list assistants");
        let mut assistants = response.data;
        while response.has_more {
            let query = [
                ("limit", "100"),
                ("after", assistants.last().unwrap().id.as_str()),
            ];
            let response = self
                .client
                .assistants()
                .list(&query)
                .await
                .expect("Failed to list assistants");
            assistants.extend(response.data);
        }

        assistants
    }

    pub async fn set_assistant_image(
        &self,
        assistent: &AssistantObject,
        image: &str,
    ) -> Result<(), OpenAIError> {
        let mut meta = assistent.metadata.clone().unwrap_or_default();
        meta.insert(
            "avatar".to_string(),
            serde_json::Value::String(image.to_string()),
        );
        self.client
            .assistants()
            .update(
                &assistent.id,
                ModifyAssistantRequest {
                    model: assistent.model.clone(),
                    metadata: Some(meta),
                    ..Default::default()
                },
            )
            .await?;
        Ok(())
    }

    pub async fn set_assistant_tools(
        &self,
        assistant: &AssistantObject,
        tools: Vec<AssistantTools>,
    ) -> Result<(), OpenAIError> {
        self.client
            .assistants()
            .update(
                &assistant.id,
                ModifyAssistantRequest {
                    model: assistant.model.clone(),
                    tools: Some(tools),
                    ..Default::default()
                },
            )
            .await?;
        Ok(())
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
