use async_openai::types::{AssistantObject, MessageContent};
use log::{debug, info};
use regex::Regex;
use serenity::all::{Command, Interaction};
use serenity::async_trait;
use serenity::builder::{CreateAttachment, CreateMessage, CreateWebhook, ExecuteWebhook};
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

use crate::database::channels::{get_channel, set_channel, ChannelConfiguration};
use crate::database::users::{User, UserStore};
use crate::openai::{OpenAI, ThreadStore};
use crate::thread::OpenAIThread;

struct Handler;

async fn webhook_say(
    ctx: &Context,
    webhook: &str,
    message: &str,
    files: Vec<&str>,
    avatar: Option<&str>,
    username: Option<&str>,
) {
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

    let hook = if let Some(avatar) = avatar {
        hook.avatar_url(avatar)
    } else {
        hook
    };

    let hook = if let Some(username) = username {
        hook.username(username)
    } else {
        hook
    };

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
    thread: &OpenAIThread,
    webhook: &str,
    assistants: &Vec<AssistantObject>,
) {
    for assistant in assistants {
        if msg.content.to_lowercase().contains(
            &assistant
                .name
                .clone()
                .unwrap_or("assistant".to_string())
                .to_lowercase(),
        ) {
            if msg.author.bot {
                debug!("Ignoring message from bot");
                continue;
            }
            let typing = msg.channel_id.start_typing(&ctx.http);
            let result = thread.run(&ctx, &assistant.id).await;
            match result {
                Ok(result) => {
                    for content in result {
                        match content {
                            MessageContent::Text(text) => {
                                for message_content in text.text.value.split_to_vector(2000) {
                                    let avatar = if let Some(avatar) = &assistant.metadata {
                                        avatar.get("avatar").map(|v| v.as_str()).flatten()
                                    } else {
                                        None
                                    };
                                    webhook_say(
                                        &ctx,
                                        &webhook,
                                        &message_content,
                                        vec![],
                                        avatar,
                                        assistant.name.as_deref(),
                                    )
                                    .await;
                                }
                            }
                            MessageContent::ImageFile(_image) => {
                                webhook_say(
                                    &ctx,
                                    &webhook,
                                    "IMAGE: Image format not supported yet",
                                    Vec::new(),
                                    None,
                                    None,
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(err) => {
                    webhook_say(
                        &ctx,
                        &webhook,
                        format!("error: {}", err).as_str(),
                        vec![],
                        None,
                        None,
                    )
                    .await
                }
            }
            typing.stop();
        }
    }
}

async fn default_response(msg: &Message, ctx: &Context, thread: &OpenAIThread) {
    if msg.content.to_lowercase().contains("lovelace") && msg.author.bot == false {
        let typing = msg.channel_id.start_typing(&ctx.http);
        let result = thread.run(&ctx, "asst_P66RVsW92Izpwky1qWDAZMO8").await;
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
                            let message = CreateMessage::new().content(message_content);
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

async fn get_or_create_channel_config(
    msg: &Message,
    ctx: &Context,
    store: &mut ThreadStore,
) -> ChannelConfiguration {
    let channel_config = get_channel(msg.channel_id.get()).expect("Failed to get channel");
    if channel_config.is_none() {
        debug!("Channel not configured");
        let webhook = msg
            .channel_id
            .create_webhook(&ctx.http, CreateWebhook::new("assistants"))
            .await
            .expect("Failed to create webhook");

        let thread = OpenAIThread::new().await;

        let config = ChannelConfiguration {
            active_assistants: vec![],
            thread: thread.id().to_owned(),
            webhook: webhook.url().expect("Failed to get webhook url"),
        };
        set_channel(msg.channel_id.get(), &config).expect("Failed to set channel");

        store.add_thread(thread);
        config
    } else {
        let channel_config = channel_config.unwrap();
        if store.get(&channel_config.thread).is_none() {
            let thread = OpenAIThread::from_existing(&channel_config.thread);
            store.add_thread(thread);
        }

        debug!(
            "Channel configuration found with thread: {}",
            channel_config.thread.clone()
        );
        channel_config
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

            if command.data.name.as_str() == "reset" {
                crate::commands::reset::run(&ctx, &command).await;
            };
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        debug!("Received message: {:?}", msg.content);
        let read_lock = ctx.data.read().await;
        let mut store = read_lock
            .get::<ThreadStore>()
            .expect("Expected ThreadStore in TypeMap")
            .lock()
            .await;

        let channel_config = get_or_create_channel_config(&msg, &ctx, &mut store).await;
        debug!("Channel config: {:?}", channel_config);

        let thread = store
            .get(&channel_config.thread)
            .expect("Failed to get thread");

        debug!("Adding message to thread");
        thread
            .add_message(format!("{}: {}", msg.author.id.get(), msg.content.clone()))
            .await;

        let openai = read_lock
            .get::<OpenAI>()
            .expect("Expected OpenAI in TypeMap");
        let assistants = openai.assistants().await;

        debug!("processing message");
        register_user(&ctx, &msg).await;
        multi_agent_response(&msg, &ctx, &thread, &channel_config.webhook, &assistants).await
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

    Command::create_global_command(&ctx.http, crate::commands::reset::register())
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
            let openai = OpenAI::new();

            data.insert::<OpenAI>(openai);
            data.insert::<ThreadStore>(Arc::new(Mutex::new(ThreadStore::new())));
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
