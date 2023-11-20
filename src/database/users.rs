use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub nickname: Option<String>,
    pub prefered_name: Option<String>,
}

impl User {
    pub fn new(id: String, name: String, nickname: Option<String>, prefered_name: Option<String>) -> Self {
        User {
            id,
            name,
            nickname,
            prefered_name,
        }
    }

    pub fn get_name(&self) -> String {
        match &self.prefered_name {
            Some(name) => name.to_owned(),
            None => match &self.nickname {
                Some(name) => name.to_owned(),
                None => self.name.to_owned(),
            },
        }
    }
}

pub struct UserStore {
    db: sled::Db,
}

impl UserStore {
    pub fn new() -> Self {
        let db = sled::open("./db/users").expect("Failed to open users database");
        UserStore { db }
    }

    pub fn get_user(&self, user_id: &str) -> Option<User> {
        let user = self.db.get(user_id.to_string()).expect("Failed to get user");
        match user {
            Some(user) => {
                let user: User = serde_json::from_slice(&user).expect("Failed to deserialize user");
                Some(user)
            }
            None => None,
        }
    }
    
    pub fn register_user(&self, user: &User) -> Result<(), String> {
        let user_json = match serde_json::to_string(&user) {
            Ok(user_json) => user_json,
            Err(err) => {
                return Err(format!("Failed to serialize user: {}", err));
            }
        };
        match self.db.insert(user.id.to_string(), user_json.as_bytes()) {
            Ok(_) => Ok(()),
            Err(err) => Err(format!("Failed to insert user: {}", err)),
        }
    }
}
