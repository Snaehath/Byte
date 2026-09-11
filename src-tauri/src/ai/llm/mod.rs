pub mod client;
pub mod ollama;

pub use client::{ChatMessage, LlmResult, OpenAiClient};
pub use ollama::OllamaProvider;

pub type LlmFuture<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmResult, String>> + Send + 'a>>;

pub trait LlmProvider: Send + Sync {
    fn model_name(&self) -> &str;
    fn chat<'a>(
        &'a self,
        client: &'a reqwest::Client,
        messages: &'a [ChatMessage],
        tools: Option<&'a [serde_json::Value]>,
    ) -> LlmFuture<'a>;
}
