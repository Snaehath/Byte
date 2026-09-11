use crate::ai::provider::Capability;
use crate::ai::llm::{OllamaProvider, OpenAiCompatibleProvider};
use crate::ai::vision::VisionService;
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

    /// Primary execution router based on requested capability
    pub async fn execute(
        &self,
        capability: Capability,
        client: &reqwest::Client,
        prompt: &str,
    ) -> Result<String, String> {
        match capability {
            Capability::TextGeneration | Capability::ToolCalling => {
                if self.config.llm.provider == "openai_compatible" && !self.config.llm.cloud_api_key.is_empty() {
                    log::info!("ModelManager: Querying cloud LLM provider at {}", self.config.llm.cloud_url);
                    let provider = OpenAiCompatibleProvider::new(
                        self.config.llm.cloud_url.clone(),
                        self.config.llm.cloud_api_key.clone(),
                        self.config.llm.cloud_model.clone(),
                        self.config.llm.temperature,
                    );
                    provider.ask(client, prompt).await
                } else {
                    log::info!("ModelManager: Querying local Ollama LLM provider at {}", self.config.llm.ollama_url);
                    let provider = OllamaProvider::new(
                        self.config.llm.ollama_url.clone(),
                        self.config.llm.ollama_model.clone(),
                        self.config.llm.temperature,
                    );
                    provider.ask(client, prompt).await
                }
            }
            Capability::Vision => {
                log::info!("ModelManager: Routing task to VisionService");
                let api_key = if !self.config.llm.cloud_api_key.is_empty() {
                    self.config.llm.cloud_api_key.clone()
                } else {
                    std::env::var("OPENROUTER_API_KEY")
                        .or_else(|_| std::env::var("NVIDIA_API_KEY"))
                        .unwrap_or_default()
                };

                if api_key.is_empty() {
                    return Err("No Vision API key configured. Please set your cloud API key in config.".to_string());
                }

                VisionService::analyze_screen(
                    client,
                    &self.config.llm.cloud_url,
                    &api_key,
                    "google/gemini-flash-1.5",
                    prompt,
                ).await
            }
        }
    }
}
