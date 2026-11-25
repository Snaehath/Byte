use std::collections::HashMap;
use crate::tools::{str_param, DesktopTool, ToolFuture, ToolRiskLevel};

pub struct AnalyzeScreenTool;

impl DesktopTool for AnalyzeScreenTool {
    fn name(&self) -> &str { "analyze_screen" }
    fn description(&self) -> &str {
        "Look at the user's screen to describe it or answer a question about what is visible (text, errors, images, apps)."
    }
    fn risk_level(&self) -> ToolRiskLevel { ToolRiskLevel::LowRisk }
    fn requires_confirmation(&self) -> bool { true }
    /// Loading the vision model can mean Ollama first unloads the text model
    /// (common on 4 GB GPUs), so allow far longer than ordinary tools.
    fn timeout_secs(&self) -> u64 { 120 }
    fn confirmation_prompt(&self, _params: &HashMap<String, serde_json::Value>) -> String {
        "Should I take a look at your screen?".to_string()
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The user's question about the screen"
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
            let target = crate::ai::ModelTarget::vision(&crate::config::AppConfig::load())?;
            let query = str_param(params, "query")
                .or_else(|| Some(transcription.trim()).filter(|t| !t.is_empty()))
                .unwrap_or("Describe what is on my screen.");
            crate::ai::VisionService::analyze_screen(client, &target, query)
                .await
                .map_err(|e| if target.is_cloud() { crate::core::cloud_error_message(&e) } else { e })
        })
    }
}
