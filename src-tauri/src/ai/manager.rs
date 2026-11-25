use crate::ai::llm::{self, ChatMessage, LlmResult};
use crate::config::AppConfig;

/// Where a model request goes: the local Ollama server or a cloud API.
#[derive(Clone)]
pub struct ModelTarget {
    pub url: String,
    pub model: String,
    /// Present only for cloud requests.
    pub api_key: Option<String>,
}

impl ModelTarget {
    pub fn is_cloud(&self) -> bool {
        self.api_key.is_some()
    }

    fn cloud(config: &AppConfig) -> Result<Self, String> {
        let api_key = crate::ai::credentials::saved_cloud_api_key()
            .ok_or_else(|| "Add your API key in Byte's settings to use the cloud model.".to_string())?;
        Ok(Self {
            url: crate::config::OPENAI_CHAT_URL.to_string(),
            model: config.llm.cloud_model.trim().to_string(),
            api_key: Some(api_key),
        })
    }

    /// Model used for conversation and tool calling.
    pub fn text(config: &AppConfig) -> Result<Self, String> {
        if config.llm.is_cloud() {
            return Self::cloud(config);
        }
        Ok(Self { url: config.llm.ollama_url.clone(), model: config.llm.ollama_model.clone(), api_key: None })
    }

    /// Model used for screen analysis. In cloud mode the same model handles images;
    /// locally it is either the vision model or, if chosen, the text model.
    pub fn vision(config: &AppConfig) -> Result<Self, String> {
        if config.llm.is_cloud() {
            return Self::cloud(config);
        }
        Ok(Self { url: config.llm.ollama_url.clone(), model: config.llm.local_vision_model().to_string(), api_key: None })
    }
}

/// Sends conversation and tool-calling requests to the selected text model.
pub struct ModelManager {
    config: AppConfig,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self { config: AppConfig::load() }
    }
}

impl ModelManager {
    pub fn is_cloud(&self) -> bool {
        self.config.llm.is_cloud()
    }

    pub async fn chat(
        &self,
        client: &reqwest::Client,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> Result<LlmResult, String> {
        let target = ModelTarget::text(&self.config)?;
        let llm_cfg = &self.config.llm;
        log::info!(
            "ModelManager: querying {} ({})",
            target.model,
            if target.is_cloud() { "cloud" } else { "local" }
        );
        llm::chat_completion(client, &target, messages, tools, llm_cfg.temperature, llm_cfg.max_tokens, &llm_cfg.reasoning_effort).await
    }
}
