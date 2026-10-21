pub mod provider;
pub mod manager;
pub mod llm;
pub mod vision;
pub mod credentials;

pub use provider::Capability;
pub use manager::ModelManager;
pub use llm::{ChatMessage, OpenAiClient, OllamaProvider};
pub use vision::VisionService;
