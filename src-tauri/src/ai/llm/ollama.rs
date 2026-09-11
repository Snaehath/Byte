use crate::ai::llm::client::{ChatMessage, OpenAiClient};

pub struct OllamaProvider {
    pub endpoint_url: String,
    pub model: String,
    pub temperature: f32,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self {
            endpoint_url: "http://localhost:11434/v1/chat/completions".to_string(),
            model: "qwen3-4b:latest".to_string(),
            temperature: 0.7,
        }
    }
}

impl OllamaProvider {
    pub fn new(endpoint_url: String, model: String, temperature: f32) -> Self {
        Self { endpoint_url, model, temperature }
    }

    pub async fn ask(&self, client: &reqwest::Client, prompt: &str) -> Result<String, String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        OpenAiClient::chat_completion(
            client,
            &self.endpoint_url,
            "",
            &self.model,
            &messages,
            self.temperature,
        ).await
    }
}
