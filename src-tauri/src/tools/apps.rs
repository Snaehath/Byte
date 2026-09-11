use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture};

pub struct OpenApplicationTool;

impl DesktopTool for OpenApplicationTool {
    fn name(&self) -> &str { "open_application" }
    fn description(&self) -> &str { "Open a desktop application by name (e.g. notepad, chrome, explorer, code, terminal, etc.)." }
    fn parameter_schema(&self) -> &str { "{\"name\": \"app_name\", \"args\": \"optional command arguments\"}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Desktop application name (e.g. notepad, chrome, explorer, code, terminal)"
                },
                "args": {
                    "type": "string",
                    "description": "Optional command arguments"
                }
            },
            "required": ["name"]
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
            if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                let name_clean = name.trim().to_lowercase();
                
                let process_name = match name_clean.as_str() {
                    "chrome" | "google chrome" | "browser" => "chrome".to_string(),
                    "notepad" | "text editor" => "notepad".to_string(),
                    "explorer" | "file explorer" | "documents" => "explorer".to_string(),
                    "terminal" | "cmd" | "powershell" => "wt".to_string(),
                    "code" | "vscode" | "vs code" => "code".to_string(),
                    _ => {
                        match find_app_shortcut(&name_clean) {
                            Some(path) => path,
                            None => name_clean.clone(),
                        }
                    }
                };

                let mut cmd = Command::new("cmd");
                let mut cmd_args = vec!["/C", "start", "", &process_name];
                let args_str;
                if let Some(args_val) = params.get("args").and_then(|v| v.as_str()) {
                    args_str = args_val.trim().to_string();
                    cmd_args.push(&args_str);
                }

                let _ = cmd.args(&cmd_args).spawn();
                Ok(format!("Opening {} for you.", name))
            } else {
                Err("Application name parameter is missing.".to_string())
            }
        })
    }
}

fn find_app_shortcut(app_name: &str) -> Option<String> {
    let search_dirs = [
        std::env::var("APPDATA").map(|p| format!(r"{}\Microsoft\Windows\Start Menu\Programs", p)).ok(),
        std::env::var("ProgramData").map(|p| format!(r"{}\Microsoft\Windows\Start Menu\Programs", p)).ok(),
    ];

    let query = app_name.to_lowercase();
    for dir_opt in &search_dirs {
        if let Some(ref dir) = dir_opt {
            if let Some(found) = scan_shortcuts_recursive(std::path::Path::new(dir), &query) {
                return Some(found);
            }
        }
    }
    None
}

fn scan_shortcuts_recursive(dir: &std::path::Path, query: &str) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = scan_shortcuts_recursive(&path, query) {
                    return Some(found);
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("lnk") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem.to_lowercase().contains(query) {
                            return Some(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
