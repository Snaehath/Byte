use crate::ai::llm::client::{ChatMessage, LlmResult, OpenAiClient};

pub struct OllamaProvider {
    pub endpoint_url: String,
    pub model: String,
    pub reasoning_effort: String,
    pub temperature: f32,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self {
            endpoint_url: "http://localhost:11434/v1/chat/completions".to_string(),
            model: "granite4.2:3b".to_string(),
            reasoning_effort: "low".to_string(),
            temperature: 0.2,
        }
    }
}

impl OllamaProvider {
    pub fn new(endpoint_url: String, model: String, reasoning_effort: String, temperature: f32) -> Self {
        Self { endpoint_url, model, reasoning_effort, temperature }
    }

    pub async fn ask(
        &self,
        client: &reqwest::Client,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> Result<LlmResult, String> {
        OpenAiClient::chat_completion(
            client,
            &self.endpoint_url,
            "",
            &self.model,
            messages,
            tools,
            self.temperature,
            &self.reasoning_effort,
        ).await
    }
}

impl super::LlmProvider for OllamaProvider {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn chat<'a>(
        &'a self,
        client: &'a reqwest::Client,
        messages: &'a [ChatMessage],
        tools: Option<&'a [serde_json::Value]>,
    ) -> super::LlmFuture<'a> {
        Box::pin(async move {
            self.ask(client, messages, tools).await
        })
    }
}
