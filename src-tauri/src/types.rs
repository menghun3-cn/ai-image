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

// 视频生成相关类型
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoGenerationOptions {
    pub prompt: String,
    pub output_dir: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub num_frames: Option<u32>,
    pub frame_rate: Option<f32>,
    pub seed: Option<u64>,
    pub negative_prompt: Option<String>,
    // 图生视频相关参数
    /// 单图生视频：图片路径或URL（支持本地路径、http/https URL、base64）
    pub image: Option<String>,
    /// 多图生视频/关键帧：图片路径或URL数组
    pub images: Option<Vec<String>>,
    /// 图生视频模式："single" | "multi" | "keyframes"
    pub image_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoGenerationResult {
    pub success: bool,
    pub video_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub path: String,
    pub name: String,
    pub time: i64,
}
