use async_trait::async_trait;
use crate::error::Result;
use crate::types::GenerationOptions;
use crate::ProviderConfig;

pub mod gemini;
pub mod modelscope;
pub mod nvidia;
pub mod openai;
pub mod openrouter;
pub mod siliconflow;

#[async_trait]
pub trait ImageProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn list_models(&self) -> Vec<String>;
    async fn generate(&self, options: &GenerationOptions) -> Result<crate::types::GenerationResult>;
}

pub fn create_provider(
    name: &str,
    config: ProviderConfig,
) -> Option<Box<dyn ImageProvider>> {
    match name {
        "modelscope" => Some(Box::new(modelscope::ModelScopeProvider::new(config))),
        "nvidia" => Some(Box::new(nvidia::NvidiaProvider::new(config))),
        "gemini" => Some(Box::new(gemini::GeminiProvider::new(config))),
        "openrouter" => Some(Box::new(openrouter::OpenRouterProvider::new(config))),
        "openai" => Some(Box::new(openai::OpenAiProvider::new(config))),
        "siliconflow" => Some(Box::new(siliconflow::SiliconFlowProvider::new(config))),
        _ => None,
    }
}
