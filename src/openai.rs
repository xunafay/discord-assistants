use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        CreateImageRequestArgs, CreateSpeechRequestArgs, CreateSpeechResponse,
        CreateTranscriptionRequestArgs, CreateTranscriptionResponse, ImageModel, ImageQuality,
        ImageSize, ImageStyle, ImagesResponse, ResponseFormat, SpeechModel, Voice, AssistantObject,
    },
    Client,
};
use log::{debug, error};
use std::{
    collections::HashMap,
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

    pub async fn assistants(&self) -> Vec<AssistantObject>{
        let query = [("limit", "100")];
        let response = self.client.assistants().list(&query).await.expect("Failed to list assistants");
        let mut assistants = response.data;
        while response.has_more {
            let query = [("limit", "100"), ("after", assistants.last().unwrap().id.as_str())];
            let response = self.client.assistants().list(&query).await.expect("Failed to list assistants");
            assistants.extend(response.data);
        }

        assistants
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
