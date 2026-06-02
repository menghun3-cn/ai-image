use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProgress {
    pub task_id: String,
    pub status: String,
    pub progress: i32,
    pub video_url: Option<String>,
    pub error: Option<String>,
}

// 全局进度存储
static PROGRESS_STORE: Lazy<Mutex<HashMap<String, VideoProgress>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn update_progress(task_id: &str, status: &str, progress: i32, video_url: Option<String>, error: Option<String>) {
    let mut store = PROGRESS_STORE.lock().unwrap();
    store.insert(task_id.to_string(), VideoProgress {
        task_id: task_id.to_string(),
        status: status.to_string(),
        progress,
        video_url,
        error,
    });
}

pub fn get_progress(task_id: &str) -> Option<VideoProgress> {
    let store = PROGRESS_STORE.lock().unwrap();
    store.get(task_id).cloned()
}

pub fn remove_progress(task_id: &str) {
    let mut store = PROGRESS_STORE.lock().unwrap();
    store.remove(task_id);
}
