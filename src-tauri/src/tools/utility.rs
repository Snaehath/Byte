use std::collections::HashMap;
use crate::tools::{coerce_to_u64, str_param, DesktopTool, ToolFuture};

const MAX_TIMER_SECS: u64 = 7 * 24 * 60 * 60;

pub struct SetTimerTool;

impl DesktopTool for SetTimerTool {
    fn name(&self) -> &str { "set_timer" }
    fn description(&self) -> &str { "Set a timer or reminder that Byte announces out loud when it is due." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "duration_seconds": {
                    "type": "integer",
                    "description": "Total duration in seconds. Convert minutes and hours: 5 minutes = 300, 1 hour = 3600."
                },
                "label": {
                    "type": "string",
                    "description": "What the reminder is for, e.g. \"take the pizza out\""
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
            let duration = params.get("duration_seconds").and_then(coerce_to_u64).unwrap_or(0);
            if duration == 0 {
                return Err("How long should the timer be?".to_string());
            }
            if duration > MAX_TIMER_SECS {
                return Err("Timers can be at most seven days long.".to_string());
            }
            let label = str_param(params, "label").unwrap_or("Your timer is done").to_string();
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let mut mem = state.memory.lock().map_err(|_| "Reminders are unavailable right now.")?;
            mem.scheduled_reminders.push(crate::Reminder {
                id: uuid::Uuid::new_v4().to_string(),
                target_time_ms: now_ms + duration * 1000,
                label,
            });
            mem.save()?;
            Ok(format!("Timer set for {}.", humanize_duration(duration)))
        })
    }
}

pub fn humanize_duration(secs: u64) -> String {
    let unit = |n: u64, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let (d, h, m, s) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60, secs % 60);
    let parts: Vec<String> = [(d, "day"), (h, "hour"), (m, "minute"), (s, "second")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| unit(n, w))
        .collect();
    parts.join(" and ")
}

/// Show a Windows balloon notification without blocking the caller.
pub fn show_notification(title: &str, message: &str) -> Result<(), String> {
    let escape = |s: &str| s.replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; $n = New-Object System.Windows.Forms.NotifyIcon; \
         $n.Icon = [System.Drawing.SystemIcons]::Information; $n.Visible = $true; \
         $n.ShowBalloonTip(6000, '{}', '{}', 'Info'); Start-Sleep -Seconds 7; $n.Dispose()",
        escape(title),
        escape(message)
    );
    crate::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't show the notification: {e}"))
}

pub struct ShowNotificationTool;

impl DesktopTool for ShowNotificationTool {
    fn name(&self) -> &str { "show_notification" }
    fn description(&self) -> &str { "Show a Windows desktop notification with a title and message." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Short notification title" },
                "message": { "type": "string", "description": "Notification text" }
            },
            "required": ["message"]
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
            let message = str_param(params, "message").ok_or("What should the notification say?")?;
            let title = str_param(params, "title").unwrap_or("Byte");
            show_notification(title, message)?;
            Ok("Notification shown.".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::humanize_duration;

    #[test]
    fn test_humanize_duration() {
        assert_eq!(humanize_duration(1), "1 second");
        assert_eq!(humanize_duration(300), "5 minutes");
        assert_eq!(humanize_duration(3660), "1 hour and 1 minute");
        assert_eq!(humanize_duration(90_000), "1 day and 1 hour");
    }
}
