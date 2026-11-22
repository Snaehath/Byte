use std::collections::HashMap;
use std::io::Write;
use std::process::Stdio;
use crate::tools::{str_param, DesktopTool, ToolFuture};

/// Longest clipboard excerpt Byte will read aloud.
const MAX_SPOKEN_CHARS: usize = 400;

pub struct ClipboardAccessTool;

impl DesktopTool for ClipboardAccessTool {
    fn name(&self) -> &str { "clipboard_access" }
    fn description(&self) -> &str { "Read the text currently on the clipboard, or copy new text to it." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write"]
                },
                "text": {
                    "type": "string",
                    "description": "Text to copy (only for write)"
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
            match str_param(params, "action").unwrap_or("read") {
                "read" => {
                    let text = tokio::task::spawn_blocking(read_clipboard)
                        .await
                        .map_err(|e| format!("Clipboard task failed: {e}"))??;
                    if text.is_empty() {
                        return Ok("Your clipboard is empty.".to_string());
                    }
                    let total = text.chars().count();
                    let excerpt: String = text.chars().take(MAX_SPOKEN_CHARS).collect();
                    Ok(if total > MAX_SPOKEN_CHARS {
                        format!("Your clipboard starts with: {excerpt}")
                    } else {
                        format!("Your clipboard says: {excerpt}")
                    })
                }
                "write" => {
                    let text = str_param(params, "text").ok_or("What should I copy?")?.to_string();
                    tokio::task::spawn_blocking(move || write_clipboard(&text))
                        .await
                        .map_err(|e| format!("Clipboard task failed: {e}"))??;
                    Ok("Copied to your clipboard.".to_string())
                }
                other => Err(format!("Unknown clipboard action: {other}")),
            }
        })
    }
}

fn read_clipboard() -> Result<String, String> {
    let out = crate::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "[Console]::OutputEncoding = [Text.Encoding]::UTF8; Get-Clipboard -Raw"])
        .output()
        .map_err(|e| format!("Couldn't read the clipboard: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Text is passed through stdin so it never needs shell quoting.
fn write_clipboard(text: &str) -> Result<(), String> {
    let mut child = crate::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "[Console]::InputEncoding = [Text.Encoding]::UTF8; Set-Clipboard -Value ([Console]::In.ReadToEnd())"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't write the clipboard: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| format!("Couldn't write the clipboard: {e}"))?;
    }
    let status = child.wait().map_err(|e| format!("Couldn't write the clipboard: {e}"))?;
    if status.success() { Ok(()) } else { Err("Couldn't write the clipboard.".to_string()) }
}
