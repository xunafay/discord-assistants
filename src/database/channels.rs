use serde::{Serialize, Deserialize};
use sled::{Db, open, IVec};

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelConfiguration {
    pub active_assistants: Vec<String>,
    pub thread: String,
    pub webhook: String,
}

pub fn get_channel(channel: u64) -> Result<Option<ChannelConfiguration>, String> {
    let db: Db = match open("./db/channels") {
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
    let db: Db = match open("./db/channels") {
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