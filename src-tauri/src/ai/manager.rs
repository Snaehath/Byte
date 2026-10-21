use crate::ai::provider::Capability;
use crate::ai::llm::{ChatMessage, LlmResult, OllamaProvider};
use crate::config::AppConfig;

pub struct ModelManager {
    config: AppConfig,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self {
            config: AppConfig::load(),
        }
    }
}

impl ModelManager {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Route model requests through local Ollama or an OpenAI-compatible cloud API.
    pub async fn execute(
        &self,
        capability: Capability,
        client: &reqwest::Client,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> Result<LlmResult, String> {
        match capability {
            Capability::TextGeneration | Capability::ToolCalling => {
                if self.config.llm.provider == "cloud" {
                    let model = self.config.llm.cloud_model.trim();
                    if model.is_empty() {
                        return Err("Choose a cloud chat model in Byte's setup.".to_string());
                    }
                    let api_key = crate::ai::credentials::cloud_api_key()?
                        .filter(|key| !key.trim().is_empty())
                        .ok_or_else(|| "Add a cloud API key in Byte's model setup.".to_string())?;
                    log::info!("ModelManager: Querying configured cloud model {}.", model);
                    crate::ai::llm::OpenAiClient::chat_completion(
                        client,
                        &self.config.llm.cloud_url,
                        &api_key,
                        model,
                        messages,
                        tools,
                        self.config.llm.temperature,
                        "",
                        false,
                    ).await
                } else {
                    log::info!(
                        "ModelManager: Querying local Ollama at {} with model {} (thinking: {})...",
                        self.config.llm.ollama_url,
                        self.config.llm.ollama_model,
                        self.config.llm.reasoning_effort
                    );
                    let provider = OllamaProvider::new(
                        self.config.llm.ollama_url.clone(),
                        self.config.llm.ollama_model.clone(),
                        self.config.llm.reasoning_effort.clone(),
                        self.config.llm.temperature,
                    );
                    provider.ask(client, messages, tools).await
                }
            }
        }
    }
}
