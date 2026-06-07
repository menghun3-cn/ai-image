use serde::{Deserialize, Serialize};

/// 获取提供商模型的请求参数
#[derive(Debug, Deserialize)]
pub struct FetchProviderModelsRequest {
    pub provider: String,
    pub api_key: String,
    pub endpoint: Option<String>,
}

/// 模型信息
#[derive(Debug, Clone, Serialize)]
pub struct ProviderModel {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// 获取提供商模型的响应
#[derive(Debug, Serialize)]
pub struct FetchProviderModelsResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<ProviderModel>>,
}

/// OpenAI 格式的模型列表响应
#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    #[serde(default)]
    object: String,
    #[serde(default)]
    created: i64,
    #[serde(default)]
    owned_by: String,
}

/// 获取提供商模型列表
#[tauri::command]
pub async fn fetch_provider_models(
    request: FetchProviderModelsRequest,
) -> Result<FetchProviderModelsResponse, String> {
    crate::log_message(&format!(
        "[FetchProviderModels] 开始获取模型: provider={}",
        request.provider
    ));

    if request.api_key.is_empty() {
        return Ok(FetchProviderModelsResponse {
            success: false,
            message: "API Key 不能为空".to_string(),
            models: None,
        });
    }

    let result = match request.provider.as_str() {
        "openai" => fetch_openai_models(&request.api_key).await,
        "siliconflow" => fetch_siliconflow_models(&request.api_key).await,
        "openrouter" => fetch_openrouter_models(&request.api_key).await,
        "nvidia" => fetch_nvidia_models(&request.api_key).await,
        "gemini" => fetch_gemini_models(&request.api_key).await,
        "modelscope" => fetch_modelscope_models(&request.api_key).await,
        _ => {
            return Ok(FetchProviderModelsResponse {
                success: false,
                message: format!("不支持的提供商: {}", request.provider),
                models: None,
            });
        }
    };

    match result {
        Ok(models) => {
            let msg = format!("获取成功！共 {} 个模型", models.len());
            crate::log_message(&format!("[FetchProviderModels] {}", msg));
            Ok(FetchProviderModelsResponse {
                success: true,
                message: msg,
                models: Some(models),
            })
        }
        Err(e) => {
            let msg = format!("获取失败: {}", e);
            crate::log_message(&format!("[FetchProviderModels] {}", msg));
            Ok(FetchProviderModelsResponse {
                success: false,
                message: msg,
                models: None,
            })
        }
    }
}

/// 获取 OpenAI 模型列表
async fn fetch_openai_models(api_key: &str) -> anyhow::Result<Vec<ProviderModel>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("API 错误: {} - {}", status, text);
    }

    let result: OpenAIModelsResponse = response.json().await?;
    
    // 过滤出图像生成相关的模型
    let models: Vec<ProviderModel> = result
        .data
        .into_iter()
        .filter(|m| {
            let id = m.id.to_lowercase();
            id.contains("dall-e") || id.contains("gpt-image") || id.contains("image")
        })
        .map(|m| ProviderModel {
            id: m.id.clone(),
            name: format_model_name(&m.id),
            description: String::new(),
        })
        .collect();

    Ok(models)
}

/// 获取 SiliconFlow 模型列表
async fn fetch_siliconflow_models(api_key: &str) -> anyhow::Result<Vec<ProviderModel>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get("https://api.siliconflow.cn/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("API 错误: {} - {}", status, text);
    }

    let result: OpenAIModelsResponse = response.json().await?;
    
    // 过滤出图像生成相关的模型
    let models: Vec<ProviderModel> = result
        .data
        .into_iter()
        .filter(|m| {
            let id = m.id.to_lowercase();
            id.contains("image") || id.contains("kolors") || id.contains("flux") || id.contains("stable")
        })
        .map(|m| ProviderModel {
            id: m.id.clone(),
            name: format_model_name(&m.id),
            description: String::new(),
        })
        .collect();

    Ok(models)
}

/// 获取 OpenRouter 模型列表
async fn fetch_openrouter_models(api_key: &str) -> anyhow::Result<Vec<ProviderModel>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("API 错误: {} - {}", status, text);
    }

    let result: OpenAIModelsResponse = response.json().await?;
    
    // 过滤出图像生成相关的模型
    let models: Vec<ProviderModel> = result
        .data
        .into_iter()
        .filter(|m| {
            let id = m.id.to_lowercase();
            id.contains("image") || id.contains("seedream") || id.contains("gpt-image")
        })
        .map(|m| ProviderModel {
            id: m.id.clone(),
            name: format_model_name(&m.id),
            description: String::new(),
        })
        .collect();

    Ok(models)
}

/// 获取 NVIDIA 模型列表
async fn fetch_nvidia_models(api_key: &str) -> anyhow::Result<Vec<ProviderModel>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get("https://integrate.api.nvidia.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("API 错误: {} - {}", status, text);
    }

    let result: OpenAIModelsResponse = response.json().await?;
    
    // 过滤出图像生成相关的模型
    let models: Vec<ProviderModel> = result
        .data
        .into_iter()
        .filter(|m| {
            let id = m.id.to_lowercase();
            id.contains("flux") || id.contains("stable") || id.contains("image")
        })
        .map(|m| ProviderModel {
            id: m.id.clone(),
            name: format_model_name(&m.id),
            description: String::new(),
        })
        .collect();

    Ok(models)
}

/// 获取 Gemini 模型列表
async fn fetch_gemini_models(api_key: &str) -> anyhow::Result<Vec<ProviderModel>> {
    // Gemini 使用不同的 API 格式
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("API 错误: {} - {}", status, text);
    }

    #[derive(Debug, Deserialize)]
    struct GeminiModelsResponse {
        models: Vec<GeminiModel>,
    }

    #[derive(Debug, Deserialize)]
    struct GeminiModel {
        name: String,
        #[serde(default)]
        displayName: String,
    }

    let result: GeminiModelsResponse = response.json().await?;
    
    // 过滤出图像生成相关的模型
    let models: Vec<ProviderModel> = result
        .models
        .into_iter()
        .filter(|m| {
            let name = m.name.to_lowercase();
            name.contains("image") || name.contains("flash-exp")
        })
        .map(|m| {
            let id = m.name.replace("models/", "");
            ProviderModel {
                id: id.clone(),
                name: if m.displayName.is_empty() {
                    format_model_name(&id)
                } else {
                    m.displayName
                },
                description: String::new(),
            }
        })
        .collect();

    Ok(models)
}

/// 获取 ModelScope 模型列表
async fn fetch_modelscope_models(api_key: &str) -> anyhow::Result<Vec<ProviderModel>> {
    // ModelScope 的 API 格式不同，暂时返回固定列表
    // 实际实现需要根据 ModelScope API 文档调整
    crate::log_message("[FetchProviderModels] ModelScope 使用固定模型列表");
    
    Ok(vec![
        ProviderModel {
            id: "Qwen/Qwen-Image".to_string(),
            name: "Qwen Image".to_string(),
            description: "通义千问图像生成模型".to_string(),
        },
    ])
}

/// 格式化模型名称
fn format_model_name(id: &str) -> String {
    // 移除常见的路径前缀
    let name = id
        .replace("models/", "")
        .replace("openai/", "")
        .replace("black-forest-labs/", "")
        .replace("stability-ai/", "")
        .replace("bytedance-seed/", "");
    
    // 将短横线和下划线替换为空格，并首字母大写
    name.split(&['-', '_', '/'][..])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
