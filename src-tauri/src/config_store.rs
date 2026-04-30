use crate::{AppConfig, ModelLists, ProviderConfig, ProvidersConfig};
use std::path::PathBuf;

/// 配置文件名
const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_DIR_NAME: &str = ".ai-image";

/// 获取配置目录
/// Windows: %USERPROFILE%/.ai-image
/// Linux/macOS: ~/.ai-image
pub fn get_config_dir() -> anyhow::Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("无法获取用户主目录"))?;
    let config_dir = home_dir.join(CONFIG_DIR_NAME);
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

/// 获取配置文件路径
pub fn get_config_path() -> anyhow::Result<PathBuf> {
    Ok(get_config_dir()?.join(CONFIG_FILE_NAME))
}

/// 初始化配置存储
/// 应用启动时调用，确保目录存在
pub fn init_config_store() -> anyhow::Result<()> {
    let config_dir = get_config_dir()?;
    crate::log_message(&format!("[ConfigStore] 配置目录: {}", config_dir.display()));
    
    let config_path = get_config_path()?;
    if !config_path.exists() {
        crate::log_message("[ConfigStore] 配置文件不存在，创建默认配置");
        let config = default_config();
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content)?;
        setup_proxy(&config);
    }
    
    Ok(())
}

/// 从 Tauri 应用存储加载配置
/// 如果配置项缺失，自动补充默认值
pub fn load_config_from_store() -> anyhow::Result<AppConfig> {
    let config_path = get_config_path()?;
    
    crate::log_message(&format!("[ConfigStore] 尝试读取配置: {}", config_path.display()));
    
    if !config_path.exists() {
        crate::log_message("[ConfigStore] 配置文件不存在，使用默认配置");
        let config = default_config();
        // 保存默认配置
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content)?;
        setup_proxy(&config);
        return Ok(config);
    }
    
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: AppConfig = serde_json::from_str(&content)?;
    
    // 补充缺失的配置项
    let default = default_config();
    let mut needs_save = false;
    
    // 补充 providers 中的缺失项
    if config.providers.modelscope.endpoint.is_empty() {
        config.providers.modelscope.endpoint = default.providers.modelscope.endpoint;
        needs_save = true;
    }
    if config.providers.nvidia.endpoint.is_empty() {
        config.providers.nvidia.endpoint = default.providers.nvidia.endpoint;
        needs_save = true;
    }
    if config.providers.gemini.endpoint.is_empty() {
        config.providers.gemini.endpoint = default.providers.gemini.endpoint;
        needs_save = true;
    }
    if config.providers.openrouter.endpoint.is_empty() {
        config.providers.openrouter.endpoint = default.providers.openrouter.endpoint;
        needs_save = true;
    }
    if config.providers.openai.endpoint.is_empty() {
        config.providers.openai.endpoint = default.providers.openai.endpoint;
        needs_save = true;
    }
    if config.providers.siliconflow.endpoint.is_empty() {
        config.providers.siliconflow.endpoint = default.providers.siliconflow.endpoint;
        needs_save = true;
    }
    
    // 补充其他缺失项
    if config.default_provider.is_empty() {
        config.default_provider = default.default_provider;
        needs_save = true;
    }
    if config.default_output_dir.is_empty() {
        config.default_output_dir = default.default_output_dir;
        needs_save = true;
    }
    if config.theme.is_empty() {
        config.theme = default.theme;
        needs_save = true;
    }
    
    // 补充 models 中的缺失项
    if config.models.modelscope.is_empty() {
        config.models.modelscope = default.models.modelscope;
        needs_save = true;
    }
    if config.models.nvidia.is_empty() {
        config.models.nvidia = default.models.nvidia;
        needs_save = true;
    }
    if config.models.gemini.is_empty() {
        config.models.gemini = default.models.gemini;
        needs_save = true;
    }
    if config.models.openrouter.is_empty() {
        config.models.openrouter = default.models.openrouter;
        needs_save = true;
    }
    if config.models.openai.is_empty() {
        config.models.openai = default.models.openai;
        needs_save = true;
    }
    if config.models.siliconflow.is_empty() {
        config.models.siliconflow = default.models.siliconflow;
        needs_save = true;
    }
    
    // 如果补充了缺失项，保存更新后的配置
    if needs_save {
        crate::log_message("[ConfigStore] 配置项有缺失，已补充默认值");
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content)?;
    }
    
    crate::log_message("[ConfigStore] 配置加载成功");
    
    // 设置代理环境变量
    setup_proxy(&config);
    
    Ok(config)
}

/// 保存配置到 Tauri 应用存储
pub fn save_config_to_store(config: &AppConfig) -> anyhow::Result<()> {
    let config_path = get_config_path()?;
    
    crate::log_message(&format!("[ConfigStore] 保存配置到: {}", config_path.display()));
    
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(&config_path, content)?;
    
    // 设置代理环境变量
    setup_proxy(config);
    
    crate::log_message("[ConfigStore] 配置保存成功");
    Ok(())
}

/// 设置代理环境变量
fn setup_proxy(config: &AppConfig) {
    if config.proxy_enabled && !config.proxy.is_empty() {
        std::env::set_var("HTTP_PROXY", &config.proxy);
        std::env::set_var("HTTPS_PROXY", &config.proxy);
        crate::log_message(&format!("[ConfigStore] 代理已设置: {}", config.proxy));
    } else {
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        crate::log_message("[ConfigStore] 代理已清除");
    }
}

/// 默认配置
fn default_config() -> AppConfig {
    AppConfig {
        providers: ProvidersConfig {
            modelscope: ProviderConfig {
                api_key: "".to_string(),
                endpoint: "https://api.modelscope.cn/v1/models".to_string(),
            },
            nvidia: ProviderConfig {
                api_key: "".to_string(),
                endpoint: "https://api.nvcf.nvidia.com".to_string(),
            },
            gemini: ProviderConfig {
                api_key: "".to_string(),
                endpoint: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            },
            openrouter: ProviderConfig {
                api_key: "".to_string(),
                endpoint: "https://openrouter.ai/api/v1/chat/completions".to_string(),
            },
            openai: ProviderConfig {
                api_key: "".to_string(),
                endpoint: "https://api.openai.com/v1/images/generations".to_string(),
            },
            siliconflow: ProviderConfig {
                api_key: "".to_string(),
                endpoint: "https://api.siliconflow.cn/v1/images/generations".to_string(),
            },
        },
        default_provider: "openrouter".to_string(),
        default_output_dir: "images".to_string(),
        default_width: 768,
        default_height: 1344,
        default_steps: Some(30),
        default_guidance_scale: Some(7.5),
        default_seed: Some(-1),
        proxy: "".to_string(),
        proxy_enabled: false,
        theme: "light".to_string(),
        models: default_models(),
    }
}

/// 默认模型列表
fn default_models() -> ModelLists {
    ModelLists {
        modelscope: vec![
            "Qwen/Qwen-Image".to_string(),
        ],
        nvidia: vec![
            "black-forest-labs/flux.2-klein-4b".to_string(),
        ],
        gemini: vec!["gemini-2.0-flash-exp-image-generation".to_string()],
        openrouter: vec![
            "bytedance-seed/seedream-4.5".to_string(),
            "openai/gpt-5.4-image-2".to_string(),
            "openrouter/auto".to_string(),
        ],
        openai: vec!["gpt-image-2".to_string()],
        siliconflow: vec![
            "Kwai-Kolors/Kolors".to_string(),
        ],
    }
}
