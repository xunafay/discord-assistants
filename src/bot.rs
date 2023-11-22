use async_openai::types::MessageContent;
use log::{debug, info};
use regex::Regex;
use serenity::all::{Command, Interaction};
use serenity::async_trait;
use serenity::builder::{CreateAttachment, CreateMessage, ExecuteWebhook};
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult,
};
use serenity::model::webhook::Webhook;
use serenity::model::{channel::Message, gateway::Ready};
use serenity::prelude::*;
use songbird::SerenityInit;
use std::env;
use std::sync::Arc;

use crate::database::users::{User, UserStore};
use crate::openai::{Assistant, AssistantStore, OpenAI, ThreadStore};
use crate::thread::OpenAIThread;

struct Handler;

fn extract_local_files(message: &str) -> Vec<String> {
    let regex = Regex::new(r"sandbox:(.*\.(?:jpg|jpeg|png|gif|mp3|wav|mp4|avi|mov))").unwrap();
    regex
        .captures_iter(message)
        .map(|capture| format!(".{}", capture[1].to_string()))
        .collect::<Vec<String>>()
}

async fn webhook_say(ctx: &Context, webhook: &str, message: &str, files: Vec<&str>) {
    let webhook = Webhook::from_url(&ctx.http, webhook)
        .await
        .expect("Failed to get webhook");

    let mut attachments = vec![];
    for file in files {
        let attachment = CreateAttachment::path(file)
            .await
            .expect("Failed to create attachment");
        attachments.push(attachment);
    }

    let hook = ExecuteWebhook::new()
        .content(message)
        .add_files(attachments);
    webhook
        .execute(&ctx.http, false, hook)
        .await
        .expect("Failed to execute webhook");
}

pub async fn register_user(ctx: &Context, msg: &Message) {
    let data_read = ctx.data.read().await;
    let user_store = data_read
        .get::<UserStore>()
        .expect("Expected UserStore in TypeMap");
    let user_store = user_store.read().await;
    user_store
        .register_user(&User::new(
            msg.author.id.get().to_string(),
            msg.author.name.clone(),
            msg.author_nick(&ctx.http).await,
            None,
        ))
        .expect("Failed to register user");
}

async fn multi_agent_response(
    msg: &Message,
    ctx: &Context,
    openai: &OpenAI,
    thread: &OpenAIThread,
    assistants: &Vec<Assistant>,
) {
    for assistant in assistants {
        if msg
            .content
            .to_lowercase()
            .contains(&assistant.name.to_lowercase())
        {
            if msg.author.id.get().to_string() == assistant.author_id {
                debug!("Ignoring message from self");
                continue;
            }
            let typing = msg.channel_id.start_typing(&ctx.http);
            let result = thread.run(&openai, &ctx, &assistant.id).await;
            match result {
                Ok(result) => {
                    for content in result {
                        match content {
                            MessageContent::Text(text) => {
                                for message_content in text.text.value.split_to_vector(2000) {
                                    let files = extract_local_files(&message_content);
                                    let files: Vec<&str> =
                                        files.iter().map(|s| s.as_str()).collect();
                                    debug!("detecte files: {:?}", files);
                                    webhook_say(&ctx, &assistant.webhook, &message_content, files)
                                        .await;
                                }
                            }
                            MessageContent::ImageFile(_image) => {
                                webhook_say(
                                    &ctx,
                                    &assistant.webhook,
                                    "IMAGE: Image format not supported yet",
                                    Vec::new(),
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(err) => {
                    webhook_say(
                        &ctx,
                        &assistant.webhook,
                        format!("error: {}", err).as_str(),
                        vec![],
                    )
                    .await
                }
            }
            typing.stop();
        }
    }
}

async fn default_response(msg: &Message, ctx: &Context, openai: &OpenAI, thread: &OpenAIThread) {
    if msg.content.to_lowercase().contains("lovelace") && msg.author.bot == false {
        let typing = msg.channel_id.start_typing(&ctx.http);
        let result = thread
            .run(&openai, &ctx, "asst_P66RVsW92Izpwky1qWDAZMO8")
            .await;
        if let Err(err_msg) = result {
            msg.channel_id
                .say(&ctx.http, format!("Error: {:?}", err_msg))
                .await
                .expect("Failed to send message");
        } else {
            for content in result.expect("Failed to get result") {
                match content {
                    MessageContent::Text(text) => {
                        for message_content in text.text.value.split_to_vector(2000) {
                            let files = extract_local_files(&message_content);
                            let files: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
                            debug!("detecte files: {:?}", files);
                            let mut attachments = vec![];
                            for file in files {
                                let attachment = CreateAttachment::path(file)
                                    .await
                                    .expect("Failed to create attachment");
                                attachments.push(attachment);
                            }

                            let message = CreateMessage::new()
                                .content(message_content)
                                .add_files(attachments);
                            msg.channel_id
                                .send_message(&ctx.http, message)
                                .await
                                .expect("Failed to send message");
                        }
                    }
                    MessageContent::ImageFile(_image) => {
                        msg.channel_id
                            .say(&ctx.http, "IMAGE: Image format not supported yet")
                            .await
                            .expect("Failed to send image");
                    }
                }
            }
        }
        typing.stop();
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            debug!("Received command {:#?}", command.data.name);

            if command.data.name.as_str() == "image" {
                crate::commands::image::run(&ctx, &command).await;
            };

            if command.data.name.as_str() == "tts" {
                crate::commands::tts::run(&ctx, &command).await;
            };

            if command.data.name.as_str() == "register" {
                crate::commands::register::run(&ctx, &command).await;
            };

            if command.data.name.as_str() == "voice" {
                crate::commands::join_voice::run(&ctx, &command).await;
            };

            if command.data.name.as_str() == "assistant" {
                crate::commands::assistant::run(&ctx, &command).await;
            };
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        debug!("Received message: {:?}", msg.content);
        let read_lock = ctx.data.read().await;
        let openai = read_lock
            .get::<OpenAI>()
            .expect("Expected OpenAI in TypeMap");
        let assistants = read_lock
            .get::<AssistantStore>()
            .expect("Expected AssistantStore in TypeMap")
            .read()
            .await;
        let mut store = read_lock
            .get::<ThreadStore>()
            .expect("Expected ThreadStore in TypeMap")
            .lock()
            .await;
        let thread = store.thread(&msg.channel_id.get()).await;
        thread
            .add_message(format!("{}: {}", msg.author.id.get(), msg.content.clone()))
            .await;

        register_user(&ctx, &msg).await;
        let assistants = assistants.get_channel(&msg.channel_id.get());
        match assistants {
            Some(assistants) => {
                multi_agent_response(&msg, &ctx, &openai, &thread, assistants).await
            }
            None => default_response(&msg, &ctx, &openai, &thread).await,
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        if env::var("DELETE_COMMANDS").is_ok_and(|v| v == "true") {
            recreate_commands(&ctx).await;
        };
    }
}

async fn recreate_commands(ctx: &Context) {
    let commands = Command::get_global_commands(&ctx.http)
        .await
        .expect("Failed to get global commands");
    for command in commands {
        Command::delete_global_command(&ctx.http, command.id)
            .await
            .expect("Failed to delete global command");
        debug!("Deleted global command {:?}", command);
    }

    Command::create_global_command(&ctx.http, crate::commands::image::register())
        .await
        .expect("Failed to create global command");

    Command::create_global_command(&ctx.http, crate::commands::tts::register())
        .await
        .expect("Failed to create global command");

    Command::create_global_command(&ctx.http, crate::commands::register::register())
        .await
        .expect("Failed to create global command");

    Command::create_global_command(&ctx.http, crate::commands::join_voice::register())
        .await
        .expect("Failed to create global command");

    Command::create_global_command(&ctx.http, crate::commands::assistant::register())
        .await
        .expect("Failed to create global command");
}

#[group]
#[commands(stt)]
struct General;

#[command]
async fn stt(ctx: &Context, msg: &Message) -> CommandResult {
    let openai = OpenAI::new();

    let typing = msg.channel_id.start_typing(&ctx.http);

    let regex = Regex::new(r"(?m)https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();
    let urls = regex
        .find_iter(&msg.content.as_str())
        .map(|m| m.as_str().to_owned())
        .collect::<Vec<String>>();

    let url = urls.first();
    if url.is_none() {
        msg.channel_id.say(&ctx.http, "No url found").await?;
        typing.stop();
        return Ok(());
    }

    let result = openai.stt(&url.unwrap());
    if let Err(err_msg) = result {
        msg.channel_id
            .say(&ctx.http, format!("Error: {:?}", err_msg))
            .await?;
        typing.stop();
        return Ok(());
    }

    typing.stop();

    let messages = result.unwrap().split_to_vector(2000);

    for part in messages {
        msg.channel_id.say(&ctx.http, part).await?;
    }

    Ok(())
}

impl TypeMapKey for OpenAI {
    type Value = OpenAI;
}

impl TypeMapKey for ThreadStore {
    type Value = Arc<Mutex<ThreadStore>>;
}

impl TypeMapKey for AssistantStore {
    type Value = Arc<RwLock<AssistantStore>>;
}

impl TypeMapKey for UserStore {
    type Value = Arc<RwLock<UserStore>>;
}

pub struct Bot;

impl Bot {
    pub fn new() -> Self {
        Bot
    }

    pub async fn start(&self, token: &str) {
        let mut client = Client::builder(token, GatewayIntents::all())
            .event_handler(Handler)
            .register_songbird()
            .await
            .expect("Error creating client");

        {
            let mut data = client.data.write().await;
            data.insert::<OpenAI>(OpenAI::new());
            data.insert::<ThreadStore>(Arc::new(Mutex::new(ThreadStore::new())));
            data.insert::<AssistantStore>(Arc::new(RwLock::new(AssistantStore::new())));
            data.insert::<UserStore>(Arc::new(RwLock::new(UserStore::new())));
        }

        if let Err(why) = client.start().await {
            println!("Client error: {:?}", why);
        }
    }
}

pub trait SplitToVector {
    fn split_to_vector(&self, length: usize) -> Vec<String>;
}

impl SplitToVector for String {
    fn split_to_vector(&self, length: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();

        for c in self.chars() {
            if current.len() + c.len_utf8() > length {
                if let Some(last_newline) = current.rfind('\n') {
                    result.push(current[..last_newline].trim_end().to_string());
                    current = current[last_newline..]
                        .trim_start_matches(|c| c == '\n' || c == ' ')
                        .to_string();
                } else if let Some(last_space) = current.rfind(' ') {
                    result.push(current[..last_space].trim_end().to_string());
                    current = current[last_space..]
                        .trim_start_matches(|c| c == '\n' || c == ' ')
                        .to_string();
                } else {
                    result.push(current.clone());
                    current.clear();
                }
            }
            current.push(c);
        }

        if !current.is_empty() {
            result.push(current);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_to_vector_empty_string() {
        let s = String::from("");
        assert_eq!(s.split_to_vector(2000), Vec::<String>::new());
    }

    #[test]
    fn test_split_to_vector_no_spaces_or_newlines() {
        let s = String::from("Hello");
        assert_eq!(s.split_to_vector(2000), vec!["Hello"]);
    }

    #[test]
    fn test_split_to_vector_with_spaces() {
        let s = String::from("Hello world");
        assert_eq!(s.split_to_vector(8), vec!["Hello", "world"]);
    }

    #[test]
    fn test_split_to_vector_with_newlines() {
        let s = String::from("Hello\nworld");
        assert_eq!(s.split_to_vector(8), vec!["Hello", "world"]);
    }

    #[test]
    fn test_split_to_vector_with_spaces_and_newlines() {
        let s = String::from("Hello world\nHow are you");
        assert_eq!(s.split_to_vector(18), vec!["Hello world", "How are you"]);
    }
}
