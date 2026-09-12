use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture, ToolRiskLevel};

pub struct OpenFolderTool;

impl DesktopTool for OpenFolderTool {
    fn name(&self) -> &str { "open_folder" }
    fn description(&self) -> &str { "Open a local directory or folder in File Explorer." }
    fn parameter_schema(&self) -> &str { "{\"path\": \"C:\\\\...\"}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Folder path or shortcut name (e.g. downloads, documents, desktop, or full path)"
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
            if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
                let target = match path.to_lowercase().as_str() {
                    "downloads" | "download" => std::env::var("USERPROFILE").map(|p| format!(r"{}\Downloads", p)).unwrap_or_else(|_| path.to_string()),
                    "documents" | "docs" => std::env::var("USERPROFILE").map(|p| format!(r"{}\Documents", p)).unwrap_or_else(|_| path.to_string()),
                    "desktop" => std::env::var("USERPROFILE").map(|p| format!(r"{}\Desktop", p)).unwrap_or_else(|_| path.to_string()),
                    _ => path.to_string(),
                };

                let _ = Command::new("cmd").args(&["/C", "start", "", &target]).spawn();
                Ok(format!("Opened folder '{}'.", target))
            } else {
                Err("Folder path parameter is missing.".to_string())
            }
        })
    }
}

pub struct OrganizeFolderTool;

impl DesktopTool for OrganizeFolderTool {
    fn name(&self) -> &str { "organize_folder" }
    fn description(&self) -> &str { "Organize files in a directory into sorted subfolders by category." }
    fn parameter_schema(&self) -> &str { "{\"path\": \"directory path to organize (default: Downloads)\"}" }
    fn risk_level(&self) -> ToolRiskLevel { ToolRiskLevel::Destructive }
    fn requires_confirmation(&self) -> bool { true }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let target_path = params.get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    std::env::var("USERPROFILE")
                        .map(|p| format!(r"{}\Downloads", p))
                        .unwrap_or_default()
                });

            if target_path.is_empty() {
                return Err("Failed to resolve folder path.".to_string());
            }

            let dir_path = std::path::Path::new(&target_path);
            if !dir_path.exists() || !dir_path.is_dir() {
                return Err(format!("The folder '{}' does not exist.", target_path));
            }

            let mut count = 0;
            if let Ok(entries) = std::fs::read_dir(dir_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(ext) = p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
                            let subfolder = match ext.as_str() {
                                "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => "Images",
                                "pdf" | "doc" | "docx" | "txt" | "xlsx" | "csv" | "ppt" | "pptx" => "Documents",
                                "mp3" | "wav" | "flac" | "mp4" | "mkv" | "mov" | "avi" => "Media",
                                "exe" | "msi" => "Installers",
                                "zip" | "rar" | "7z" | "tar" | "gz" => "Archives",
                                _ => "Other",
                            };

                            let dest_dir = dir_path.join(subfolder);
                            let _ = std::fs::create_dir_all(&dest_dir);
                            if let Some(file_name) = p.file_name() {
                                let dest_path = dest_dir.join(file_name);
                                if std::fs::rename(&p, &dest_path).is_ok() {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }

            Ok(format!("Organized {} files in '{}' into subdirectories.", count, target_path))
        })
    }
}
