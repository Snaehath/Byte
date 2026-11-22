use crate::ai::llm::{self, ChatMessage, LlmResult};
use crate::config::AppConfig;

/// Sends conversation and tool-calling requests to the fixed local text model.
pub struct ModelManager {
    config: AppConfig,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self { config: AppConfig::load() }
    }
}

impl ModelManager {
    pub async fn chat(
        &self,
        client: &reqwest::Client,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> Result<LlmResult, String> {
        let llm_cfg = &self.config.llm;
        log::info!("ModelManager: querying {} at {}", llm_cfg.ollama_model, llm_cfg.ollama_url);
        llm::chat_completion(
            client,
            &llm_cfg.ollama_url,
            &llm_cfg.ollama_model,
            messages,
            tools,
            llm_cfg.temperature,
            llm_cfg.max_tokens,
            &llm_cfg.reasoning_effort,
        )
        .await
    }
}
