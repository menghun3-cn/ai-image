use crate::{AppConfig, ModelLists, ProviderConfig, ProvidersConfig};
use std::collections::HashMap;
use std::fs;

pub fn load_config() -> anyhow::Result<AppConfig> {
    let project_root = crate::get_project_root();
    let env_path = project_root.join(".env");
    
    crate::log_message(&format!("[Config] 项目根目录: {}", project_root.display()));
    crate::log_message(&format!("[Config] 尝试读取 .env: {}", env_path.display()));

    // 如果 .env 不存在，尝试从 .env.example 创建
    if !env_path.exists() {
        let example_path = project_root.join(".env.example");
        crate::log_message(&format!("[Config] .env 不存在，检查 .env.example: {}", example_path.display()));
        if example_path.exists() {
            crate::log_message("[Config] 从 .env.example 创建 .env");
            if let Ok(content) = fs::read_to_string(&example_path) {
                let _ = fs::write(&env_path, content);
            }
        }
    }

    // 读取 .env 文件
    let env_content = if env_path.exists() {
        crate::log_message(&format!("[Config] 成功读取 .env: {}", env_path.display()));
        fs::read_to_string(&env_path).unwrap_or_default()
    } else {
        crate::log_message("[Config] .env 文件不存在，使用空配置");
        String::new()
    };

    let vars = parse_env(&env_content);

    let get_var = |key: &str, default: &str| -> String {
        vars.get(key).cloned().unwrap_or_else(|| default.to_string())
    };

    // 设置代理环境变量
    let proxy_enabled = get_var("PROXY_ENABLED", "false") == "true";
    let proxy = get_var("PROXY", "");
    if proxy_enabled && !proxy.is_empty() {
        std::env::set_var("HTTP_PROXY", &proxy);
        std::env::set_var("HTTPS_PROXY", &proxy);
    }

    Ok(AppConfig {
        providers: ProvidersConfig {
            modelscope: ProviderConfig {
                api_key: get_var("MODELSCOPE_API_KEY", ""),
                endpoint: get_var(
                    "MODELSCOPE_ENDPOINT",
                    "https://api.modelscope.cn/v1/models",
                ),
            },
            nvidia: ProviderConfig {
                api_key: get_var("NVIDIA_API_KEY", ""),
                endpoint: get_var("NVIDIA_ENDPOINT", "https://api.nvcf.nvidia.com"),
            },
            gemini: ProviderConfig {
                api_key: get_var("GEMINI_API_KEY", ""),
                endpoint: get_var(
                    "GEMINI_ENDPOINT",
                    "https://generativelanguage.googleapis.com/v1beta/models",
                ),
            },
            openrouter: ProviderConfig {
                api_key: get_var("OPENROUTER_API_KEY", ""),
                endpoint: get_var(
                    "OPENROUTER_ENDPOINT",
                    "https://openrouter.ai/api/v1/chat/completions",
                ),
            },
            openai: ProviderConfig {
                api_key: get_var("OPENAI_API_KEY", ""),
                endpoint: get_var(
                    "OPENAI_ENDPOINT",
                    "https://api.openai.com/v1/images/generations",
                ),
            },
            siliconflow: ProviderConfig {
                api_key: get_var("SILICONFLOW_API_KEY", ""),
                endpoint: get_var(
                    "SILICONFLOW_ENDPOINT",
                    "https://api.siliconflow.cn/v1/images/generations",
                ),
            },
            agnes: ProviderConfig {
                api_key: get_var("AGNES_API_KEY", ""),
                endpoint: get_var(
                    "AGNES_ENDPOINT",
                    "https://apihub.agnes-ai.com/v1",
                ),
            },
        },
        default_provider: get_var("DEFAULT_PROVIDER", "agnes"),
        default_output_dir: get_var("DEFAULT_OUTPUT_DIR", "images"),
        default_width: get_var("DEFAULT_WIDTH", "768").parse().unwrap_or(768),
        default_height: get_var("DEFAULT_HEIGHT", "1344").parse().unwrap_or(1344),
        default_steps: get_var("DEFAULT_STEPS", "30").parse().ok(),
        default_guidance_scale: get_var("DEFAULT_GUIDANCE_SCALE", "7.5").parse().ok(),
        default_seed: get_var("DEFAULT_SEED", "-1").parse().ok(),
        proxy,
        proxy_enabled,
        theme: get_var("THEME", "light"),
        models: ModelLists {
            modelscope: vec![
                "Qwen/Qwen-Image".to_string(),
                "damo/cv_diffusion_text-to-image".to_string(),
            ],
            nvidia: vec!["stable-diffusion-xl-1024-v1-0".to_string()],
            gemini: vec!["gemini-2.0-flash-exp-image-generation".to_string()],
            openrouter: vec![
                "bytedance-seed/seedream-4.5".to_string(),
                "openai/gpt-image-1".to_string(),
            ],
            openai: vec!["gpt-image-1".to_string(), "dall-e-3".to_string()],
            siliconflow: vec![
                "Kwai-Kolors/Kolors".to_string(),
                "stabilityai/stable-diffusion-3-5-large".to_string(),
            ],
            agnes: vec![
                "agnes-image-2.1-flash".to_string(),
            ],
        },
    })
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let project_root = crate::get_project_root();
    let env_path = project_root.join(".env");

    let content = format!(
        r#"# ModelScope Configuration
MODELSCOPE_API_KEY={}
MODELSCOPE_ENDPOINT={}

# NVIDIA Configuration
NVIDIA_API_KEY={}
NVIDIA_ENDPOINT={}

# Gemini Configuration
GEMINI_API_KEY={}
GEMINI_ENDPOINT={}

# OpenRouter Configuration
OPENROUTER_API_KEY={}
OPENROUTER_ENDPOINT={}

# OpenAI Configuration
OPENAI_API_KEY={}
OPENAI_ENDPOINT={}

# SiliconFlow Configuration
SILICONFLOW_API_KEY={}
SILICONFLOW_ENDPOINT={}

# Default Settings
DEFAULT_PROVIDER={}
DEFAULT_OUTPUT_DIR={}
DEFAULT_WIDTH={}
DEFAULT_HEIGHT={}
DEFAULT_STEPS={}
DEFAULT_GUIDANCE_SCALE={}
DEFAULT_SEED={}

# Default Models
DEFAULT_MODEL_MODELSCOPE=Qwen/Qwen-Image
DEFAULT_MODEL_NVIDIA=stable-diffusion-xl-1024-v1-0
DEFAULT_MODEL_GEMINI=gemini-2.0-flash-exp-image-generation
DEFAULT_MODEL_OPENROUTER=bytedance-seed/seedream-4.5
DEFAULT_MODEL_OPENAI=gpt-image-1
DEFAULT_MODEL_SILICONFLOW=Kwai-Kolors/Kolors

# Proxy Settings
PROXY_ENABLED={}
PROXY={}
"#,
        config.providers.modelscope.api_key,
        config.providers.modelscope.endpoint,
        config.providers.nvidia.api_key,
        config.providers.nvidia.endpoint,
        config.providers.gemini.api_key,
        config.providers.gemini.endpoint,
        config.providers.openrouter.api_key,
        config.providers.openrouter.endpoint,
        config.providers.openai.api_key,
        config.providers.openai.endpoint,
        config.providers.siliconflow.api_key,
        config.providers.siliconflow.endpoint,
        config.default_provider,
        config.default_output_dir,
        config.default_width,
        config.default_height,
        config.default_steps.unwrap_or(30),
        config.default_guidance_scale.unwrap_or(7.5),
        config.default_seed.unwrap_or(-1),
        config.proxy_enabled,
        config.proxy,
    );

    fs::write(&env_path, content)?;
    Ok(())
}

fn parse_env(content: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            vars.insert(key, value);
        }
    }

    vars
}
