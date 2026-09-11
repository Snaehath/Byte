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

    /// Primary execution router for local Ollama with native tool calling support
    pub async fn execute(
        &self,
        capability: Capability,
        client: &reqwest::Client,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> Result<LlmResult, String> {
        match capability {
            Capability::TextGeneration | Capability::ToolCalling => {
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
            Capability::Vision => {
                log::info!("ModelManager: Vision capability requested locally.");
                Err("Local vision model is not currently active in Ollama. Pull a local vision model (e.g. minicpm-v or llava) to enable local screen analysis.".to_string())
            }
        }
    }
}
