use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationOptions {
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
    pub output_dir: String,
    pub width: u32,
    pub height: u32,
    pub steps: Option<u32>,
    pub guidance_scale: Option<f64>,
    pub seed: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerationResult {
    pub success: bool,
    pub image_path: Option<String>,
    pub error: Option<String>,
    pub retries: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizeResult {
    pub success: bool,
    pub optimized_prompt: Option<String>,
    pub original_intent: Option<String>,
    pub style: Option<String>,
    pub negative_prompt: Option<String>,
    pub tips: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageInfo {
    pub path: String,
    pub name: String,
    pub time: i64,
}
