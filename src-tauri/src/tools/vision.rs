use std::collections::HashMap;
use crate::tools::{DesktopTool, ToolFuture};
use crate::ai::VisionService;

pub struct AnalyzeScreenTool;

impl DesktopTool for AnalyzeScreenTool {
    fn name(&self) -> &str { "analyze_screen" }
    fn description(&self) -> &str { "Capture a screenshot of your primary monitor and describe or answer questions about what is on screen." }
    fn parameter_schema(&self) -> &str { "{\"query\": \"question about screen contents (optional)\"}" }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        client: &'a reqwest::Client,
        _transcription: &'a str,
        state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("Describe what is currently visible on my screen.");
            
            let mut api_key = String::new();
            if let Ok(mem) = state.memory.lock() {
                if let Some(key) = mem.user_profile.preferences.get("openrouter_key").or_else(|| mem.user_profile.preferences.get("nvidia_key")) {
                    api_key = key.clone();
                }
            }

            if api_key.is_empty() {
                if let Ok(key) = std::env::var("OPENROUTER_API_KEY").or_else(|_| std::env::var("NVIDIA_API_KEY")) {
                    api_key = key;
                }
            }

            if api_key.is_empty() {
                return Err("No Vision API key found. Please configure an OPENROUTER_API_KEY or NVIDIA_API_KEY.".to_string());
            }

            let result = VisionService::analyze_screen(
                client,
                "https://openrouter.ai/api/v1/chat/completions",
                &api_key,
                "google/gemini-flash-1.5",
                query,
            ).await?;

            Ok(result)
        })
    }
}
