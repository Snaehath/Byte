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

pub trait DesktopTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameter_schema(&self) -> &str;
    fn requires_confirmation(&self) -> bool {
        false
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

    pub async fn execute_tool(
        &self,
        tool_call: &ToolCall,
        window: &tauri::WebviewWindow,
        client: &reqwest::Client,
        transcription: &str,
        state: &tauri::State<'_, crate::AppState>,
    ) -> Result<String, String> {
        if let Some(tool) = self.tools.get(tool_call.tool.as_str()) {
            tool.execute(&tool_call.parameters, window, client, transcription, state).await
        } else {
            Err(format!("Unknown tool: {}", tool_call.tool))
        }
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
    registry.execute_tool(tool_call, window, client, transcription, state).await
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
}

