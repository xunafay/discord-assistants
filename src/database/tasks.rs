use serde::{Deserialize, Serialize};
// Step 1: Import the sled crate
use sled::open;
use sled::{Db, IVec};

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    user_id: String,
    title: String,
    description: Option<String>,
    due_date: Option<String>,
    estimated_time: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListTaskToolArguments {
    pub user_id: String,
}

pub fn get_tasks(user_id: &str) -> Result<Vec<(String, Task)>, String> {
    let db = match open("/db/tasks") {
        Ok(db) => db,
        Err(err) => {
            return Err(format!("Failed to open sled database: {}", err));
        }
    };

    let mut tasks: Vec<(String, Task)> = Vec::new();
    for task in db.iter() {
        match task {
            Ok((key, value)) => {
                let task: Task =
                    serde_json::from_slice(&value).expect("Failed to deserialize task");
                if task.user_id == *user_id {
                    tasks.push((
                        std::str::from_utf8(&key)
                            .expect("Failed to convert key to string")
                            .to_owned(),
                        task,
                    ));
                }
            }
            Err(err) => {
                return Err(format!("Failed to get task: {}", err));
            }
        }
    }
    Ok(tasks)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTaskToolArguments {
    pub user_id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub estimated_time: Option<String>,
}

pub fn create_task(
    user_id: &str,
    title: &str,
    description: Option<String>,
    due_date: Option<String>,
    estimated_time: Option<String>,
) -> Result<(), String> {
    let db: Db = match open("/db/tasks") {
        Ok(db) => db,
        Err(err) => {
            return Err(format!("Failed to open sled database: {}", err));
        }
    };

    let task = Task {
        user_id: user_id.to_string(),
        title: title.to_string(),
        description: description,
        due_date: due_date,
        estimated_time: estimated_time,
    };

    let id = rand::random::<u64>().to_string();
    let task_json = match serde_json::to_string(&task) {
        Ok(task_json) => task_json,
        Err(err) => {
            return Err(format!("Failed to serialize task: {}", err));
        }
    };
    match db.insert(id, IVec::from(task_json.as_str())) {
        Ok(_) => Ok(()),
        Err(err) => {
            return Err(format!("Failed to insert task: {}", err));
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompleteTaskToolArgumetens {
    pub id: String,
}

pub fn complete_task(id: &str) -> Result<(), String> {
    let db: Db = match open("/db/tasks") {
        Ok(db) => db,
        Err(err) => {
            return Err(format!("Failed to open sled database: {}", err));
        }
    };

    match db.remove(id) {
        Ok(_) => Ok(()),
        Err(err) => {
            return Err(format!("Failed to delete task: {}", err));
        }
    }
}
