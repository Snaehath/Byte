// DEPRECATED: This file is preserved temporarily for backwards-compatibility.
// Real implementation is located in crate::ai::*.

use crate::ai::{Capability, ModelManager};

pub async fn ask_llm_with_fallback(
    client: &reqwest::Client,
    prompt: &str,
    _memory: &crate::ByteMemory,
) -> Result<String, Box<dyn std::error::Error>> {
    let manager = ModelManager::default();
    manager.execute(Capability::TextGeneration, client, prompt).await
        .map_err(|e| e.into())
}
