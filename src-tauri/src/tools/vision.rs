use std::collections::HashMap;
use crate::tools::{DesktopTool, ToolFuture};

pub struct AnalyzeScreenTool;

impl DesktopTool for AnalyzeScreenTool {
    fn name(&self) -> &str { "analyze_screen" }
    fn description(&self) -> &str { "Capture a screenshot of your primary monitor to describe or answer questions about what is on screen." }
    fn parameter_schema(&self) -> &str { "{\"query\": \"question about screen contents (optional)\"}" }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Optional question or analysis request regarding the screen contents"
                }
            }
        })
    }

    fn execute<'a>(
        &self,
        _params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            Ok("Screen vision is scheduled for local multimodal Ollama models (e.g. minicpm-v or llava) in Phase 4.".to_string())
        })
    }
}
