use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::platform::windows::shell_open;
use crate::tools::{str_param, DesktopTool, ToolFuture};

/// Resolve spoken folder names ("downloads", "my documents") or full paths.
fn resolve_folder(spoken: &str) -> Option<PathBuf> {
    let lower = spoken.trim().to_lowercase();
    let lower = lower.trim_start_matches("my ").trim_end_matches(" folder");
    let home = std::env::var("USERPROFILE").ok().map(PathBuf::from);
    let known = match lower {
        "downloads" | "download" => Some("Downloads"),
        "documents" | "docs" => Some("Documents"),
        "desktop" => Some("Desktop"),
        "pictures" | "photos" | "images" => Some("Pictures"),
        "music" => Some("Music"),
        "videos" => Some("Videos"),
        "home" | "user" => Some(""),
        _ => None,
    };
    let path = match known {
        Some(sub) => home?.join(sub),
        None => PathBuf::from(spoken.trim()),
    };
    path.is_dir().then_some(path)
}

fn folder_label(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

pub struct OpenFolderTool;

impl DesktopTool for OpenFolderTool {
    fn name(&self) -> &str { "open_folder" }
    fn description(&self) -> &str { "Open a folder in File Explorer, e.g. downloads, documents, desktop, pictures, or a full path." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Folder name (downloads, documents, desktop, pictures, music, videos) or a full path"
                }
            },
            "required": ["path"]
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
            let spoken = str_param(params, "path").ok_or("Which folder should I open?")?;
            let path = resolve_folder(spoken).ok_or_else(|| format!("I couldn't find a folder called {spoken}."))?;
            shell_open(&path.to_string_lossy(), None)?;
            Ok(format!("Opening {}.", folder_label(&path)))
        })
    }
}
