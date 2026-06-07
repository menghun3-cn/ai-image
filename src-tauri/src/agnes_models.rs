use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agnes 模型分类
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgnesModels {
    /// 文生文模型 - 用于提示词优化
    #[serde(default)]
    pub text_to_text: Vec<AgnesModel>,
    /// 文生图模型 - 用于图片生成
    #[serde(default)]
    pub text_to_image: Vec<AgnesModel>,
    /// 文生视频模型 - 用于视频生成
    #[serde(default)]
    pub text_to_video: Vec<AgnesModel>,
}

/// 单个模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgnesModel {
    /// 模型ID
    pub id: String,
    /// 模型显示名称
    pub name: String,
    /// 模型描述
    #[serde(default)]
    pub description: String,
}

/// 模型列表存储结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgnesModelsStore {
    /// 文生文模型
    #[serde(default)]
    pub text_to_text: Vec<AgnesModel>,
    /// 文生图模型
    #[serde(default)]
    pub text_to_image: Vec<AgnesModel>,
    /// 文生视频模型
    #[serde(default)]
    pub text_to_video: Vec<AgnesModel>,
    /// 最后更新时间（Unix 时间戳，秒）
    #[serde(default)]
    pub last_updated: Option<i64>,
}

impl AgnesModelsStore {
    /// 创建空存储
    pub fn empty() -> Self {
        Self {
            text_to_text: Vec::new(),
            text_to_image: Vec::new(),
            text_to_video: Vec::new(),
            last_updated: None,
        }
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.text_to_text.is_empty()
            && self.text_to_image.is_empty()
            && self.text_to_video.is_empty()
    }

    /// 从 AgnesModels 创建
    pub fn from_models(models: AgnesModels) -> Self {
        Self {
            text_to_text: models.text_to_text,
            text_to_image: models.text_to_image,
            text_to_video: models.text_to_video,
            last_updated: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
            ),
        }
    }
}

/// OpenAI 格式的模型列表响应
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {
    pub data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub owned_by: String,
}

const AGNES_MODELS_FILE: &str = "agnes_models.json";

/// 获取 Agnes 模型文件路径
pub fn get_models_file_path() -> anyhow::Result<PathBuf> {
    let config_dir = crate::config_store::get_config_dir()?;
    Ok(config_dir.join(AGNES_MODELS_FILE))
}

/// 加载本地存储的 Agnes 模型
pub fn load_agnes_models() -> anyhow::Result<AgnesModelsStore> {
    let path = get_models_file_path()?;
    
    if !path.exists() {
        crate::log_message("[AgnesModels] 模型文件不存在，返回空配置");
        return Ok(AgnesModelsStore::empty());
    }

    let content = std::fs::read_to_string(&path)?;
    let store: AgnesModelsStore = serde_json::from_str(&content)?;
    
    crate::log_message(&format!(
        "[AgnesModels] 加载模型成功: 文生文={}, 文生图={}, 文生视频={}",
        store.text_to_text.len(),
        store.text_to_image.len(),
        store.text_to_video.len()
    ));
    
    Ok(store)
}

/// 保存 Agnes 模型到本地
pub fn save_agnes_models(store: &AgnesModelsStore) -> anyhow::Result<()> {
    let path = get_models_file_path()?;
    let content = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, content)?;
    
    crate::log_message(&format!(
        "[AgnesModels] 保存模型成功到: {}",
        path.display()
    ));
    
    Ok(())
}

/// 从远端拉取 Agnes 模型列表
pub async fn fetch_agnes_models(
    endpoint: &str,
    api_key: &str,
) -> anyhow::Result<AgnesModels> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let url = format!("{}/models", endpoint.trim_end_matches('/'));
    
    crate::log_message(&format!("[AgnesModels] 开始拉取模型列表: {}", url));

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("获取模型列表失败: {} - {}", status, text);
    }

    let result: OpenAIModelsResponse = response.json().await?;
    
    crate::log_message(&format!(
        "[AgnesModels] 成功获取 {} 个模型",
        result.data.len()
    ));

    // 分类模型
    let mut models = AgnesModels::default();
    
    for model in result.data {
        let id_lower = model.id.to_lowercase();
        
        // 根据模型ID特征分类
        if id_lower.contains("video") || id_lower.contains("vid") {
            // 视频模型
            models.text_to_video.push(AgnesModel {
                id: model.id.clone(),
                name: format_model_name(&model.id),
                description: String::new(),
            });
        } else if id_lower.contains("image") 
            || id_lower.contains("img") 
            || id_lower.contains("diffusion")
            || id_lower.contains("dall-e")
            || id_lower.contains("flux")
            || id_lower.contains("sd")
            || id_lower.contains("stable") {
            // 图片模型
            models.text_to_image.push(AgnesModel {
                id: model.id.clone(),
                name: format_model_name(&model.id),
                description: String::new(),
            });
        } else if id_lower.contains("gpt")
            || id_lower.contains("claude")
            || id_lower.contains("llm")
            || id_lower.contains("chat")
            || id_lower.contains("text")
            || id_lower.contains("qwen")
            || id_lower.contains("gemini")
            || (id_lower.contains("agnes") && !id_lower.contains("image") && !id_lower.contains("video")) {
            // 文本/LLM 模型 - 用于提示词优化
            // Agnes 的文生文模型（如 agnes-2.0-flash，不含 image/video 关键词）
            models.text_to_text.push(AgnesModel {
                id: model.id.clone(),
                name: format_model_name(&model.id),
                description: String::new(),
            });
        } else if id_lower.contains("agnes") && id_lower.contains("image") {
            // Agnes 文生图模型（如 agnes-image-2.1-flash、agnes-1.5-flash-image）
            models.text_to_image.push(AgnesModel {
                id: model.id.clone(),
                name: format_model_name(&model.id),
                description: String::new(),
            });
        } else if id_lower.contains("agnes") && id_lower.contains("video") {
            // Agnes 文生视频模型
            models.text_to_video.push(AgnesModel {
                id: model.id.clone(),
                name: format_model_name(&model.id),
                description: String::new(),
            });
        } else {
            // 无法识别的模型，默认归为文生图
            models.text_to_image.push(AgnesModel {
                id: model.id.clone(),
                name: format_model_name(&model.id),
                description: String::new(),
            });
        }
    }

    crate::log_message(&format!(
        "[AgnesModels] 模型分类完成: 文生文={}, 文生图={}, 文生视频={}",
        models.text_to_text.len(),
        models.text_to_image.len(),
        models.text_to_video.len()
    ));

    Ok(models)
}

/// 格式化模型名称
fn format_model_name(id: &str) -> String {
    // 提取模型名称，去掉路径前缀
    let name = id.split('/').last().unwrap_or(id);
    
    // 简单的格式化：替换下划线和连字符为空格，首字母大写
    name.split(|c| c == '-' || c == '_')
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 更新 Agnes 模型（拉取并保存）
pub async fn update_agnes_models(
    endpoint: &str,
    api_key: &str,
) -> anyhow::Result<AgnesModelsStore> {
    let models = fetch_agnes_models(endpoint, api_key).await?;
    
    let store = AgnesModelsStore::from_models(models);
    
    save_agnes_models(&store)?;
    
    Ok(store)
}

/// 启动时尝试拉取模型（失败不报错）
pub async fn try_fetch_on_startup(endpoint: &str, api_key: &str) {
    if api_key.is_empty() {
        crate::log_message("[AgnesModels] API Key 为空，跳过启动拉取");
        return;
    }

    crate::log_message("[AgnesModels] 启动时尝试拉取模型...");
    
    match update_agnes_models(endpoint, api_key).await {
        Ok(store) => {
            crate::log_message(&format!(
                "[AgnesModels] 启动拉取成功: 文生文={}, 文生图={}, 文生视频={}",
                store.text_to_text.len(),
                store.text_to_image.len(),
                store.text_to_video.len()
            ));
        }
        Err(e) => {
            crate::log_message(&format!(
                "[AgnesModels] 启动拉取失败，保持本地配置: {}",
                e
            ));
        }
    }
}

/// 获取默认模型（当没有配置时使用）
pub fn get_default_models() -> AgnesModels {
    AgnesModels {
        text_to_text: vec![
            AgnesModel {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                description: "用于提示词优化".to_string(),
            },
        ],
        text_to_image: vec![
            AgnesModel {
                id: "agnes-image-2.1-flash".to_string(),
                name: "Agnes Image 2.1 Flash".to_string(),
                description: "默认图片生成模型".to_string(),
            },
        ],
        text_to_video: vec![
            AgnesModel {
                id: "agnes-video-1.0".to_string(),
                name: "Agnes Video 1.0".to_string(),
                description: "默认视频生成模型".to_string(),
            },
        ],
    }
}

/// 获取模型ID列表（用于兼容旧接口）
pub fn get_model_ids(models: &[AgnesModel]) -> Vec<String> {
    models.iter().map(|m| m.id.clone()).collect()
}

/// 从模型列表中选择最佳模型（优先最新版本）
/// 选择策略：
/// 1. 优先选择版本号最高的模型（如 2.0 > 1.5）
/// 2. 如果版本号相同，优先选择 flash 版本（更快）
/// 3. 如果没有匹配，返回第一个模型或默认模型
pub fn select_best_model(models: &[AgnesModel], default_model: &str) -> String {
    if models.is_empty() {
        return default_model.to_string();
    }

    // 尝试按版本号排序选择最新版本
    let mut best_model = &models[0];
    let mut best_version = extract_version(&best_model.id);
    let mut best_is_flash = best_model.id.to_lowercase().contains("flash");

    for model in models.iter().skip(1) {
        let version = extract_version(&model.id);
        let is_flash = model.id.to_lowercase().contains("flash");

        // 比较版本号
        if version > best_version {
            best_model = model;
            best_version = version;
            best_is_flash = is_flash;
        } else if version == best_version && is_flash && !best_is_flash {
            // 版本相同，优先选择 flash 版本
            best_model = model;
            best_is_flash = true;
        }
    }

    crate::log_message(&format!(
        "[AgnesModels] 选择最佳模型: {} (版本: {:?}, flash: {})",
        best_model.id, best_version, best_is_flash
    ));

    best_model.id.clone()
}

/// 提取版本号（支持 x.x 格式，如 2.0, 1.5）
fn extract_version(model_id: &str) -> (u32, u32) {
    // 匹配版本号模式，如 2.0, 1.5, v2.0 等
    let re = regex::Regex::new(r"[vV]?(\d+)\.(\d+)").unwrap();

    if let Some(caps) = re.captures(model_id) {
        let major = caps.get(1).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        let minor = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap_or(0));
        (major, minor)
    } else {
        (0, 0)
    }
}

/// 获取文生图最佳模型
pub fn get_best_text_to_image_model() -> String {
    match load_agnes_models() {
        Ok(store) if !store.text_to_image.is_empty() => {
            select_best_model(&store.text_to_image, "agnes-image-2.1-flash")
        }
        _ => "agnes-image-2.1-flash".to_string(),
    }
}

/// 获取文生文最佳模型（用于提示词优化）
pub fn get_best_text_to_text_model() -> String {
    match load_agnes_models() {
        Ok(store) if !store.text_to_text.is_empty() => {
            select_best_model(&store.text_to_text, "agnes-2.0-flash")
        }
        _ => "agnes-2.0-flash".to_string(),
    }
}

/// 获取文生视频最佳模型
pub fn get_best_text_to_video_model() -> String {
    match load_agnes_models() {
        Ok(store) if !store.text_to_video.is_empty() => {
            select_best_model(&store.text_to_video, "agnes-video-v2.0")
        }
        _ => "agnes-video-v2.0".to_string(),
    }
}
