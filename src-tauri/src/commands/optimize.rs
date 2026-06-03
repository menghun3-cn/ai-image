use crate::config_store;
use crate::types::OptimizeResult;
use serde::{Deserialize, Serialize};

const SYSTEM_PROMPT: &str = r#"你是一名世界级 AI 绘图提示词架构师（Prompt Architect）。

你的任务是将用户输入的简单描述解析成结构化视觉数据，而不是直接生成最终绘图 Prompt。

请根据用户意图自动补充合理细节，但不要改变主题。补充细节时必须优先保留用户明确表达的主体、动作、场景、风格和限制。

输出必须是合法 JSON。

规则：
1. 仅返回 JSON
2. 不返回 Markdown
3. 不返回解释
4. 不返回代码块
5. 所有字段必须存在
6. 未知内容根据场景合理推断，不使用 null
7. 优先级：保留用户明确意图 > 合理补充视觉细节 > 提升画面丰富度与生成质量
8. details 数组包含 3-8 个画面中可见的具体细节
9. negative_prompt 输出中文负面词，用逗号分隔
10. 不要添加指定格式以外的字段

JSON格式：
{
  "subject": "",
  "action": "",
  "scene": "",
  "environment": "",
  "composition": "",
  "camera": "",
  "lighting": "",
  "style": "",
  "color_palette": "",
  "details": [],
  "mood": "",
  "quality": "",
  "negative_prompt": ""
}

字段说明：
subject: 主体
action: 动作
scene: 核心场景
environment: 环境细节
composition: 构图方式
camera: 镜头语言
lighting: 光影设计
style: 艺术风格
color_palette: 色彩方案
details: 增强细节数组
mood: 氛围
quality: 质量增强词
negative_prompt: 负面词"#;

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VisualDecomposition {
    subject: String,
    action: String,
    scene: String,
    environment: String,
    composition: String,
    camera: String,
    lighting: String,
    style: String,
    color_palette: String,
    details: Vec<String>,
    mood: String,
    quality: String,
    negative_prompt: String,
}

fn normalize_json_content(content: &str) -> &str {
    let trimmed = content.trim();
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim();
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim();
    }
    trimmed
}

fn push_sentence(parts: &mut Vec<String>, text: String) {
    let text = text.trim();
    if !text.is_empty() {
        parts.push(text.to_string());
    }
}

fn build_chinese_prompt(visual: &VisualDecomposition) -> String {
    let mut parts = Vec::new();

    push_sentence(
        &mut parts,
        format!(
            "{}正在{}，核心场景是{}。",
            visual.subject, visual.action, visual.scene
        ),
    );
    push_sentence(
        &mut parts,
        format!(
            "画面环境包含{}，采用{}构图，镜头语言为{}。",
            visual.environment, visual.composition, visual.camera
        ),
    );
    push_sentence(
        &mut parts,
        format!(
            "光影设计为{}，整体风格是{}，色彩方案为{}。",
            visual.lighting, visual.style, visual.color_palette
        ),
    );

    let details = visual
        .details
        .iter()
        .map(|detail| detail.trim())
        .filter(|detail| !detail.is_empty())
        .collect::<Vec<_>>()
        .join("、");
    if !details.is_empty() {
        push_sentence(&mut parts, format!("画面加入{}等增强细节。", details));
    }

    push_sentence(
        &mut parts,
        format!("整体氛围{}，{}。", visual.mood, visual.quality),
    );

    parts.join("")
}

#[tauri::command]
pub async fn optimize_prompt(prompt: String) -> Result<OptimizeResult, String> {
    let config = config_store::load_config_from_store().map_err(|e| e.to_string())?;

    if config.providers.agnes.api_key.is_empty() {
        return Ok(OptimizeResult {
            success: false,
            optimized_prompt: None,
            original_intent: None,
            style: None,
            negative_prompt: None,
            tips: None,
            error: Some("请先配置 Agnes API Key".to_string()),
        });
    }

    let request = ChatRequest {
        model: "agnes-2.0-flash".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: format!("请将以下图像描述拆解为结构化视觉 JSON：\n\n{}", prompt),
            },
        ],
    };

    let endpoint = format!(
        "{}/chat/completions",
        config.providers.agnes.endpoint.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header(
            "Authorization",
            format!("Bearer {}", config.providers.agnes.api_key),
        )
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Ok(OptimizeResult {
            success: false,
            optimized_prompt: None,
            original_intent: None,
            style: None,
            negative_prompt: None,
            tips: None,
            error: Some(format!("API 错误 [{}]: {}", status, text)),
        });
    }

    let result: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if let Some(choice) = result.choices.first() {
        let content = normalize_json_content(&choice.message.content);
        let visual = serde_json::from_str::<VisualDecomposition>(content)
            .map_err(|e| format!("解析视觉拆解 JSON 失败: {}", e))?;

        return Ok(OptimizeResult {
            success: true,
            optimized_prompt: Some(build_chinese_prompt(&visual)),
            original_intent: Some(prompt),
            style: Some(visual.style),
            negative_prompt: Some(visual.negative_prompt),
            tips: None,
            error: None,
        });
    }

    Ok(OptimizeResult {
        success: false,
        optimized_prompt: None,
        original_intent: None,
        style: None,
        negative_prompt: None,
        tips: None,
        error: Some("无法获取优化结果".to_string()),
    })
}
