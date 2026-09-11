pub mod client;
pub mod ollama;
pub mod openai_compatible;

pub use client::{ChatMessage, OpenAiClient};
pub use ollama::OllamaProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
