pub mod apps;
pub mod browser;
pub mod windows;
pub mod files;
pub mod system;
pub mod clipboard;
pub mod utility;
pub mod memory;
pub mod vision;

use serde::Deserialize;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

#[derive(serde::Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

pub fn parse_tool_call(text: &str) -> Option<ToolCall> {
    let text = text.trim();
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end > start {
        let json_str = &text[start..=end];
        if let Ok(tc) = serde_json::from_str::<ToolCall>(json_str) {
            return Some(tc);
        }
        let cleaned = json_str
            .replace("\\n", " ")
            .replace("\\\"", "\"");
        if let Ok(tc) = serde_json::from_str::<ToolCall>(&cleaned) {
            return Some(tc);
        }
        None
    } else {
        None
    }
}

pub fn coerce_to_u64(val: &serde_json::Value) -> Option<u64> {
    match val {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

pub fn coerce_to_f64(val: &serde_json::Value) -> Option<f64> {
    match val {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Deserialize)]
pub enum ToolRiskLevel {
    /// Read-only, informational, timers, notifications, system stats
    Safe,
    /// Window management, application launch, browser navigation, clipboard
    LowRisk,
    /// Bulk file reorganization, process killing, power state change, file removal
    Destructive,
}

pub trait DesktopTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameter_schema(&self) -> &str {
        "{}"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }
    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::Safe
    }
    fn requires_confirmation(&self) -> bool {
        self.risk_level() == ToolRiskLevel::Destructive
    }
    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        window: &'a tauri::WebviewWindow,
        client: &'a reqwest::Client,
        transcription: &'a str,
        state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a>;
}

pub struct ToolRegistry {
    pub tools: HashMap<&'static str, Box<dyn DesktopTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut tools: HashMap<&'static str, Box<dyn DesktopTool>> = HashMap::new();
        tools.insert("open_application", Box::new(apps::OpenApplicationTool));
        tools.insert("open_url", Box::new(browser::OpenUrlTool));
        tools.insert("web_search", Box::new(browser::WebSearchTool));
        tools.insert("window_control", Box::new(windows::WindowControlTool));
        tools.insert("open_folder", Box::new(files::OpenFolderTool));
        tools.insert("organize_folder", Box::new(files::OrganizeFolderTool));
        tools.insert("system_control", Box::new(system::SystemControlTool));
        tools.insert("get_system_stats", Box::new(system::GetSystemStatsTool));
        tools.insert("clipboard_access", Box::new(clipboard::ClipboardAccessTool));
        tools.insert("set_timer", Box::new(utility::SetTimerTool));
        tools.insert("show_notification", Box::new(utility::ShowNotificationTool));
        tools.insert("update_memory", Box::new(memory::UpdateMemoryTool));
        tools.insert("analyze_screen", Box::new(vision::AnalyzeScreenTool));
        Self { tools }
    }

    pub fn get_openai_tools(&self) -> Vec<serde_json::Value> {
        let mut tools = Vec::new();
        let mut sorted_keys: Vec<&&str> = self.tools.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            if let Some(tool) = self.tools.get(*key) {
                tools.push(serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters_schema()
                    }
                }));
            }
        }
        tools
    }

    pub fn get_instructions_prompt(&self) -> String {
        let mut prompt = String::new();
        let mut sorted_keys: Vec<&&str> = self.tools.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            if let Some(tool) = self.tools.get(*key) {
                prompt.push_str(&format!("- `{{\"tool\": \"{}\", \"parameters\": {}}}` ({})\n", tool.name(), tool.parameter_schema(), tool.description()));
            }
        }
        prompt
    }

    pub fn get_tool(&self, name: &str) -> Option<&Box<dyn DesktopTool>> {
        self.tools.get(name)
    }

    pub fn validate_execution(&self, tool_name: &str, confirmed: bool) -> Result<(), String> {
        if let Some(tool) = self.tools.get(tool_name) {
            if tool.requires_confirmation() && !confirmed {
                return Err(format!(
                    "Tool '{}' requires explicit user confirmation before execution.",
                    tool.name()
                ));
            }
            Ok(())
        } else {
            Err(format!("Unknown tool: {}", tool_name))
        }
    }

    pub async fn execute_tool(
        &self,
        tool_call: &ToolCall,
        window: &tauri::WebviewWindow,
        client: &reqwest::Client,
        transcription: &str,
        state: &tauri::State<'_, crate::AppState>,
        confirmed: bool,
    ) -> Result<String, String> {
        self.validate_execution(&tool_call.tool, confirmed)?;
        let tool = self.tools.get(tool_call.tool.as_str()).unwrap();
        tool.execute(&tool_call.parameters, window, client, transcription, state).await
    }
}

pub async fn handle_tool_call(
    tool_call: &ToolCall,
    window: &tauri::WebviewWindow,
    client: &reqwest::Client,
    transcription: &str,
    state: &tauri::State<'_, crate::AppState>,
) -> Result<String, String> {
    log::info!("Executing tool call: {:?}", tool_call);
    let registry = ToolRegistry::new();
    registry.execute_tool(tool_call, window, client, transcription, state, false).await
}

pub fn extract_think(text: &str) -> (String, String) {
    if let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            let thinking = text[start + 7..end].trim().to_string();
            let response = format!("{}{}", &text[..start], &text[end + 8..]).trim().to_string();
            return (thinking, response);
        }
    }
    ("".to_string(), text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call() {
        let input = r#"{"tool": "open_application", "parameters": {"name": "notepad"}}"#;
        let parsed = parse_tool_call(input).expect("Should parse valid tool call");
        assert_eq!(parsed.tool, "open_application");
        assert_eq!(parsed.parameters.get("name").unwrap(), "notepad");
    }

    #[test]
    fn test_parse_tool_call_embedded() {
        let input = r#"Here is the call: {"tool": "window_control", "parameters": {"action": "minimize"}} thanks!"#;
        let parsed = parse_tool_call(input).expect("Should extract embedded tool call");
        assert_eq!(parsed.tool, "window_control");
    }

    #[test]
    fn test_tool_risk_level_policy() {
        let registry = ToolRegistry::new();
        let organize_tool = registry.get_tool("organize_folder").expect("organize_folder must exist");
        assert_eq!(organize_tool.risk_level(), ToolRiskLevel::Destructive);
        assert!(organize_tool.requires_confirmation());

        let stats_tool = registry.get_tool("get_system_stats").expect("get_system_stats must exist");
        assert_eq!(stats_tool.risk_level(), ToolRiskLevel::Safe);
        assert!(!stats_tool.requires_confirmation());
    }

    #[test]
    fn test_validate_execution_policy() {
        let registry = ToolRegistry::new();
        // Destructive tool without confirmation -> BLOCKED
        let unconfirmed_res = registry.validate_execution("organize_folder", false);
        assert!(unconfirmed_res.is_err(), "Destructive tool must be blocked without confirmation");
        assert!(unconfirmed_res.unwrap_err().contains("requires explicit user confirmation"));

        // Destructive tool with confirmation -> ALLOWED
        let confirmed_res = registry.validate_execution("organize_folder", true);
        assert!(confirmed_res.is_ok(), "Destructive tool must be allowed with confirmation");

        // Safe tool without confirmation -> ALLOWED
        let safe_res = registry.validate_execution("get_system_stats", false);
        assert!(safe_res.is_ok(), "Safe tool must be allowed without confirmation");

        // Unknown tool -> BLOCKED
        let unknown_res = registry.validate_execution("non_existent_tool", false);
        assert!(unknown_res.is_err());
    }

    #[test]
    fn test_parse_tool_call_malformed() {
        assert!(parse_tool_call("").is_none());
        assert!(parse_tool_call("just random text without braces").is_none());
        assert!(parse_tool_call(r#"{"tool": "broken"#).is_none());
        assert!(parse_tool_call(r#"{"tool": 12345, "parameters": {}}"#).is_none());
        assert!(parse_tool_call(r#"{"missing_tool_field": true}"#).is_none());
    }

    #[test]
    fn test_coerce_edge_cases() {
        use serde_json::json;
        // u64 coercion
        assert_eq!(coerce_to_u64(&json!(42)), Some(42));
        assert_eq!(coerce_to_u64(&json!("100")), Some(100));
        assert_eq!(coerce_to_u64(&json!("not_a_number")), None);
        assert_eq!(coerce_to_u64(&json!(null)), None);
        assert_eq!(coerce_to_u64(&json!({"obj": 1})), None);
        assert_eq!(coerce_to_u64(&json!([1, 2, 3])), None);

        // f64 coercion
        assert_eq!(coerce_to_f64(&json!(3.14)), Some(3.14));
        assert_eq!(coerce_to_f64(&json!("2.718")), Some(2.718));
        assert_eq!(coerce_to_f64(&json!("invalid")), None);
        assert_eq!(coerce_to_f64(&json!(null)), None);
    }
}

