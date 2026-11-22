use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::platform::windows::shell_open;
use crate::tools::{str_param, DesktopTool, ToolFuture, ToolRiskLevel};

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

pub struct OrganizeFolderTool;

impl DesktopTool for OrganizeFolderTool {
    fn name(&self) -> &str { "organize_folder" }
    fn description(&self) -> &str {
        "Sort the files in a folder into subfolders by type (Images, Documents, Media, Installers, Archives, Code, Other)."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Folder to organize, e.g. downloads, desktop, or a full path. Defaults to downloads."
                }
            }
        })
    }
    fn risk_level(&self) -> ToolRiskLevel { ToolRiskLevel::Destructive }
    fn confirmation_prompt(&self, params: &HashMap<String, serde_json::Value>) -> String {
        let folder = str_param(params, "path").unwrap_or("downloads");
        format!("Should I sort the files in your {folder} folder into subfolders by type?")
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
            let spoken = str_param(params, "path").unwrap_or("downloads").to_string();
            let dir = resolve_folder(&spoken).ok_or_else(|| format!("I couldn't find a folder called {spoken}."))?;
            let label = folder_label(&dir);
            let (moved, skipped) = tokio::task::spawn_blocking(move || organize(&dir))
                .await
                .map_err(|e| format!("Organize task failed: {e}"))??;
            Ok(match (moved, skipped) {
                (0, 0) => format!("{label} has no loose files to organize."),
                (m, 0) => format!("Organized {m} files in {label}."),
                (m, s) => format!("Organized {m} files in {label}. I left {s} alone because a file with the same name already existed."),
            })
        })
    }
}

fn category(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "heic" | "tiff" | "ico" => "Images",
        "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" | "xls" | "xlsx" | "csv" | "ppt" | "pptx" | "md" | "epub" => "Documents",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "mp4" | "mkv" | "mov" | "avi" | "webm" => "Media",
        "exe" | "msi" | "msix" | "appx" => "Installers",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" => "Archives",
        "py" | "js" | "ts" | "rs" | "java" | "c" | "cpp" | "h" | "cs" | "go" | "html" | "css" | "json" | "xml" | "yml" | "yaml" | "ipynb" => "Code",
        // Downloads still in progress must never be moved.
        "crdownload" | "part" | "partial" | "tmp" | "download" | "lnk" | "ini" => return None,
        _ => "Other",
    })
}

fn organize(dir: &Path) -> Result<(usize, usize), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("I couldn't read {}: {e}", dir.display()))?;
    let (mut moved, mut skipped) = (0, 0);
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
        let Some(sub) = category(&ext) else { continue };
        let Some(file_name) = path.file_name() else { continue };
        let dest_dir = dir.join(sub);
        let dest = dest_dir.join(file_name);
        // fs::rename replaces existing files on Windows; never overwrite.
        if dest.exists() {
            skipped += 1;
            continue;
        }
        if std::fs::create_dir_all(&dest_dir).is_ok() && std::fs::rename(&path, &dest).is_ok() {
            moved += 1;
        }
    }
    Ok((moved, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organize_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("byte_organize_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("Images")).unwrap();
        std::fs::write(dir.join("a.png"), b"new").unwrap();
        std::fs::write(dir.join("Images").join("a.png"), b"old").unwrap();
        std::fs::write(dir.join("b.pdf"), b"doc").unwrap();
        std::fs::write(dir.join("c.crdownload"), b"partial").unwrap();

        let (moved, skipped) = organize(&dir).unwrap();
        assert_eq!((moved, skipped), (1, 1));
        assert_eq!(std::fs::read(dir.join("Images").join("a.png")).unwrap(), b"old");
        assert!(dir.join("Documents").join("b.pdf").exists());
        assert!(dir.join("c.crdownload").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
