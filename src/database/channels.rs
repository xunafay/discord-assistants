use serde::{Deserialize, Serialize};
use sled::{open, Db, IVec};

use crate::thread::OpenAIThread;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelConfiguration {
    pub active_assistants: Vec<String>,
    pub thread: String,
    pub webhook: String,
}

pub async fn reset_channel_thread(channel_id: u64) -> Result<(), String> {
    let channel = get_channel(channel_id.clone())
        .expect("Failed to fetch channel")
        .expect("Failed to fetch channel");
    let thread = OpenAIThread::new().await;
    let channel_configuration = ChannelConfiguration {
        thread: thread.id().to_string(),
        webhook: channel.webhook,
        active_assistants: channel.active_assistants,
    };
    set_channel(channel_id, &channel_configuration).expect("Failed to update channel");
    Ok(())
}

pub fn get_channel(channel: u64) -> Result<Option<ChannelConfiguration>, String> {
    let db: Db = match open("/db/channels") {
        Ok(db) => db,
        Err(err) => {
            return Err(format!("Failed to open sled database: {}", err));
        }
    };

    let channel = match db.get(channel.to_string()) {
        Ok(channel) => channel,
        Err(_) => {
            return Err("Failed to query db".to_string());
        }
    };

    match channel {
        Some(channel) => {
            let channel: ChannelConfiguration = match serde_json::from_slice(&channel) {
                Ok(channel) => channel,
                Err(err) => {
                    return Err(format!("Failed to deserialize channel: {}", err));
                }
            };
            Ok(Some(channel))
        }
        None => Ok(None),
    }
}

pub fn set_channel(channel: u64, configuration: &ChannelConfiguration) -> Result<(), String> {
    let db: Db = match open("/db/channels") {
        Ok(db) => db,
        Err(err) => {
            return Err(format!("Failed to open sled database: {}", err));
        }
    };

    let channel_json = match serde_json::to_string(&configuration) {
        Ok(channel_json) => channel_json,
        Err(err) => {
            return Err(format!("Failed to serialize channel: {}", err));
        }
    };
    match db.insert(channel.to_string(), IVec::from(channel_json.as_str())) {
        Ok(_) => Ok(()),
        Err(err) => {
            return Err(format!("Failed to insert channel: {}", err));
        }
    }
}
