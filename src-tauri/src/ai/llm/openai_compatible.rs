use crate::ai::llm::client::{ChatMessage, OpenAiClient};

pub struct OpenAiCompatibleProvider {
    pub endpoint_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
}

impl OpenAiCompatibleProvider {
    pub fn new(endpoint_url: String, api_key: String, model: String, temperature: f32) -> Self {
        Self { endpoint_url, api_key, model, temperature }
    }

    pub async fn ask(&self, client: &reqwest::Client, prompt: &str) -> Result<String, String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        OpenAiClient::chat_completion(
            client,
            &self.endpoint_url,
            &self.api_key,
            &self.model,
            &messages,
            self.temperature,
        ).await
    }
}
