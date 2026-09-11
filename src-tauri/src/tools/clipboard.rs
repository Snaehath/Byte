use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture};

pub struct ClipboardAccessTool;

impl DesktopTool for ClipboardAccessTool {
    fn name(&self) -> &str { "clipboard_access" }
    fn description(&self) -> &str { "Read the current text from the system clipboard or copy new text to it." }
    fn parameter_schema(&self) -> &str { "{\"action\": \"read\" | \"write\", \"text\": \"text to copy (optional)\"}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write"],
                    "description": "Whether to read current clipboard or write new text"
                },
                "text": {
                    "type": "string",
                    "description": "Text to write to clipboard (required if action is write)"
                }
            },
            "required": ["action"]
        })
    }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("read");
            match action {
                "read" => {
                    let output = Command::new("powershell")
                        .args(&["-Command", "Get-Clipboard"])
                        .output();
                    match output {
                        Ok(out) => {
                            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            if text.is_empty() {
                                Ok("Your clipboard is currently empty.".to_string())
                            } else {
                                Ok(format!("Clipboard contents: \"{}\".", text))
                            }
                        }
                        Err(e) => Err(format!("Failed to read clipboard: {}", e)),
                    }
                }
                "write" => {
                    if let Some(text) = params.get("text").and_then(|v| v.as_str()) {
                        let cmd_str = format!("Set-Clipboard -Value '{}'", text.replace("'", "''"));
                        let output = Command::new("powershell")
                            .args(&["-Command", &cmd_str])
                            .output();
                        match output {
                            Ok(_) => Ok("Text copied to clipboard.".to_string()),
                            Err(e) => Err(format!("Failed to write to clipboard: {}", e)),
                        }
                    } else {
                        Err("The 'text' parameter is required to write to clipboard.".to_string())
                    }
                }
                _ => Err(format!("Unknown clipboard action: {}", action)),
            }
        })
    }
}
