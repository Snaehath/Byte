use std::collections::HashMap;
use crate::tools::{DesktopTool, ToolFuture, ToolRiskLevel};

pub struct AnalyzeScreenTool;

impl DesktopTool for AnalyzeScreenTool {
    fn name(&self) -> &str { "analyze_screen" }
    fn description(&self) -> &str { "Capture a screenshot of your primary monitor to describe or answer questions about what is on screen." }
    fn parameter_schema(&self) -> &str { "{\"query\": \"question about screen contents (optional)\"}" }

    fn risk_level(&self) -> ToolRiskLevel { ToolRiskLevel::LowRisk }

    fn requires_confirmation(&self) -> bool { true }

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
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        client: &'a reqwest::Client,
        transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let config = crate::config::AppConfig::load();
            let model = config.llm.vision_model.trim();
            if model.is_empty() {
                return Err("Set llm.vision_model in Byte's config.json to an Ollama vision model, such as llava.".to_string());
            }

            let query = params
                .get("query")
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(transcription);
            let query = if query.trim().is_empty() {
                "Describe what is visible on my screen."
            } else {
                query
            };

            crate::ai::VisionService::analyze_screen(
                client,
                &config.llm.ollama_url,
                "",
                model,
                query,
            )
            .await
        })
    }
}
