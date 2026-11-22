use std::collections::HashMap;
use crate::platform::windows::{control_user_window, WindowAction};
use crate::tools::{str_param, DesktopTool, ToolFuture};

pub struct WindowControlTool;

impl DesktopTool for WindowControlTool {
    fn name(&self) -> &str { "window_control" }
    fn description(&self) -> &str { "Minimize, maximize, restore, or close the app window the user is currently working in." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["minimize", "maximize", "restore", "close"]
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
            let (action, verb) = match str_param(params, "action") {
                Some("minimize") => (WindowAction::Minimize, "Minimized"),
                Some("maximize") => (WindowAction::Maximize, "Maximized"),
                Some("restore") => (WindowAction::Restore, "Restored"),
                Some("close") => (WindowAction::Close, "Closed"),
                Some(other) => return Err(format!("Unknown window action: {other}")),
                None => return Err("Should I minimize, maximize, restore, or close the window?".to_string()),
            };
            let title = control_user_window(action)?;
            let short: String = title.rsplit(" - ").next().unwrap_or(&title).chars().take(40).collect();
            Ok(if short.is_empty() { format!("{verb} the window.") } else { format!("{verb} {short}.") })
        })
    }
}
