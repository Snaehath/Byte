use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture, coerce_to_u64};

pub struct SetTimerTool;

impl DesktopTool for SetTimerTool {
    fn name(&self) -> &str { "set_timer" }
    fn description(&self) -> &str { "Set a reminder or timer for a specified duration in seconds." }
    fn parameter_schema(&self) -> &str { "{\"duration_seconds\": number, \"label\": \"reminder label\"}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "duration_seconds": {
                    "type": "integer",
                    "description": "Timer duration in seconds"
                },
                "label": {
                    "type": "string",
                    "description": "Reminder description or label"
                }
            },
            "required": ["duration_seconds"]
        })
    }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let duration = params.get("duration_seconds")
                .and_then(coerce_to_u64)
                .unwrap_or(0);
            let label = params.get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("timer")
                .to_string();
            
            if duration > 0 {
                let target_time_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64 + (duration * 1000);
                
                let id = uuid::Uuid::new_v4().to_string();
                let reminder = crate::Reminder {
                    id,
                    target_time_ms,
                    label: label.clone(),
                };
                
                if let Ok(mut mem) = state.memory.lock() {
                    mem.scheduled_reminders.push(reminder);
                    let _ = mem.save();
                }
                
                Ok(format!("Timer set for {} seconds: {}.", duration, label))
            } else {
                Err("Timer duration must be greater than zero.".to_string())
            }
        })
    }
}

pub struct ShowNotificationTool;

impl DesktopTool for ShowNotificationTool {
    fn name(&self) -> &str { "show_notification" }
    fn description(&self) -> &str { "Display a native Windows notification balloon tip." }
    fn parameter_schema(&self) -> &str { "{\"title\": \"notification title\", \"message\": \"notification message\"}" }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Byte");
            let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("Hello!");
            
            let cmd_str = format!(
                r#"[void] [System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); $n = New-Object System.Windows.Forms.NotifyIcon; $n.Icon = [System.Drawing.SystemIcons]::Information; $n.Visible = $True; $n.ShowBalloonTip(5000, '{}', '{}', 'Info')"#,
                title.replace("'", "''"),
                message.replace("'", "''")
            );
            
            let output = Command::new("powershell")
                .args(&["-Command", &cmd_str])
                .spawn();
            match output {
                Ok(_) => Ok("Notification displayed.".to_string()),
                Err(e) => Err(format!("Failed to show notification: {}", e)),
            }
        })
    }
}
