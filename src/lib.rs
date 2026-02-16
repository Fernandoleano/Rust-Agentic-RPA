use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::BufReader;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum Status {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub status: Status,
}

pub const DB_FILE: &str = "tasks.json";

pub fn load_tasks() -> Result<Vec<Task>, Box<dyn std::error::Error>> {
    let path = std::path::Path::new(DB_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let tasks: Vec<Task> = serde_json::from_reader(reader)?;
    Ok(tasks)
}

pub fn save_tasks(tasks: &[Task]) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(DB_FILE)?;

    serde_json::to_writer_pretty(file, tasks)?;
    Ok(())
}
