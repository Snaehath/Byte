use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture};

pub struct WindowControlTool;

impl DesktopTool for WindowControlTool {
    fn name(&self) -> &str { "window_control" }
    fn description(&self) -> &str { "Minimize, maximize, or close the currently active window." }
    fn parameter_schema(&self) -> &str { "{\"action\": \"minimize\" | \"maximize\" | \"close\"}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["minimize", "maximize", "close"],
                    "description": "Action to perform on foreground active window"
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
            if let Some(action) = params.get("action").and_then(|v| v.as_str()) {
                let cmd_str = match action {
                    "minimize" => {
                        r#"
                        $w = Add-Type -memberDefinition '[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow); [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();' -name "Win32" -namespace "Win" -passThru
                        $active = $w::GetForegroundWindow()
                        $w::ShowWindow($active, 6)
                        "#
                    }
                    "maximize" => {
                        r#"
                        $w = Add-Type -memberDefinition '[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow); [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();' -name "Win32" -namespace "Win" -passThru
                        $active = $w::GetForegroundWindow()
                        $w::ShowWindow($active, 3)
                        "#
                    }
                    "close" => {
                        r#"
                        $w = Add-Type -memberDefinition '[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam); [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();' -name "Win32Close" -namespace "WinClose" -passThru
                        $active = $w::GetForegroundWindow()
                        $w::PostMessage($active, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
                        "#
                    }
                    _ => ""
                };

                if !cmd_str.is_empty() {
                    let _ = Command::new("powershell")
                        .args(&["-Command", cmd_str])
                        .spawn();
                    Ok(format!("Active window {} action executed.", action))
                } else {
                    Err(format!("Unknown window control action: {}", action))
                }
            } else {
                Err("Window control action parameter is missing.".to_string())
            }
        })
    }
}
