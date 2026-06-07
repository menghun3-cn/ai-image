use crate::agnes_models::{self, AgnesModelsStore};
use serde::{Deserialize, Serialize};

/// 更新 Agnes 模型的请求参数
#[derive(Debug, Deserialize)]
pub struct UpdateAgnesModelsRequest {
    pub endpoint: String,
    pub api_key: String,
}

/// 更新 Agnes 模型的响应
#[derive(Debug, Serialize)]
pub struct UpdateAgnesModelsResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<AgnesModelsStore>,
}

/// 手动更新 Agnes 模型列表
#[tauri::command]
pub async fn update_agnes_models(
    request: UpdateAgnesModelsRequest,
) -> Result<UpdateAgnesModelsResponse, String> {
    crate::log_message(&format!(
        "[UpdateAgnesModels] 开始更新模型: endpoint={}",
        request.endpoint
    ));

    if request.api_key.is_empty() {
        return Ok(UpdateAgnesModelsResponse {
            success: false,
            message: "API Key 不能为空".to_string(),
            data: None,
        });
    }

    match agnes_models::update_agnes_models(&request.endpoint, &request.api_key).await {
        Ok(store) => {
            let msg = format!(
                "更新成功！文生文: {} 个, 文生图: {} 个, 文生视频: {} 个",
                store.text_to_text.len(),
                store.text_to_image.len(),
                store.text_to_video.len()
            );
            crate::log_message(&format!("[UpdateAgnesModels] {}", msg));
            Ok(UpdateAgnesModelsResponse {
                success: true,
                message: msg,
                data: Some(store),
            })
        }
        Err(e) => {
            let msg = format!("更新失败: {}", e);
            crate::log_message(&format!("[UpdateAgnesModels] {}", msg));
            Ok(UpdateAgnesModelsResponse {
                success: false,
                message: msg,
                data: None,
            })
        }
    }
}

/// 获取本地存储的 Agnes 模型列表
#[tauri::command]
pub fn get_agnes_models() -> Result<AgnesModelsStore, String> {
    match agnes_models::load_agnes_models() {
        Ok(store) => Ok(store),
        Err(e) => Err(format!("加载模型列表失败: {}", e)),
    }
}

/// 获取默认的 Agnes 模型列表（用于初始化）
#[tauri::command]
pub fn get_default_agnes_models() -> AgnesModelsStore {
    let models = agnes_models::get_default_models();
    AgnesModelsStore {
        text_to_text: models.text_to_text,
        text_to_image: models.text_to_image,
        text_to_video: models.text_to_video,
        last_updated: None,
    }
}
