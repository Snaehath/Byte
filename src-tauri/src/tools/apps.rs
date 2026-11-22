use std::collections::HashMap;
use std::path::Path;
use crate::platform::windows::shell_open;
use crate::tools::{str_param, DesktopTool, ToolFuture};

pub struct OpenApplicationTool;

impl DesktopTool for OpenApplicationTool {
    fn name(&self) -> &str { "open_application" }
    fn description(&self) -> &str {
        "Open an installed desktop application by its name, e.g. notepad, chrome, spotify, vs code, calculator, settings."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Application name as the user said it, e.g. \"chrome\" or \"visual studio code\""
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
            let name = str_param(params, "name").ok_or("Which app should I open?")?.to_string();
            if name.contains(['\\', '/', ':']) {
                return Err("I can only open apps by name.".to_string());
            }
            let display = name.clone();
            tokio::task::spawn_blocking(move || launch(&name))
                .await
                .map_err(|e| format!("App launch task failed: {e}"))??;
            Ok(format!("Opening {display}."))
        })
    }
}

/// Built-in Windows programs that are always on PATH.
fn builtin(name: &str) -> Option<&'static str> {
    Some(match name {
        "notepad" | "text editor" => "notepad.exe",
        "explorer" | "file explorer" | "files" => "explorer.exe",
        "calculator" | "calc" => "calc.exe",
        "task manager" => "taskmgr.exe",
        "paint" => "mspaint.exe",
        "command prompt" | "cmd" => "cmd.exe",
        "powershell" => "powershell.exe",
        "control panel" => "control.exe",
        "settings" | "windows settings" => "ms-settings:",
        "snipping tool" => "ms-screenclip:",
        _ => return None,
    })
}

/// Common nicknames mapped to their Start menu names.
fn start_menu_name(name: &str) -> &str {
    match name {
        "chrome" | "google chrome" | "browser" => "google chrome",
        "edge" | "microsoft edge" => "microsoft edge",
        "code" | "vscode" | "vs code" => "visual studio code",
        "terminal" | "windows terminal" => "terminal",
        "word" => "word",
        "excel" => "excel",
        other => other,
    }
}

fn launch(name: &str) -> Result<(), String> {
    let query = name.trim().to_lowercase();
    if let Some(program) = builtin(&query) {
        return shell_open(program, None);
    }

    let query = start_menu_name(&query);
    if let Some(shortcut) = find_start_menu_shortcut(query) {
        return shell_open(&shortcut, None);
    }
    if let Some(app_id) = find_store_app(query) {
        return shell_open("explorer.exe", Some(&format!("shell:AppsFolder\\{app_id}")));
    }
    shell_open(query, None).map_err(|_| format!("I couldn't find an app called {name}."))
}

/// Find the best-matching Start menu shortcut (exact name first, then the shortest partial match).
fn find_start_menu_shortcut(query: &str) -> Option<String> {
    let roots = [
        std::env::var("APPDATA").ok().map(|p| format!(r"{p}\Microsoft\Windows\Start Menu\Programs")),
        std::env::var("ProgramData").ok().map(|p| format!(r"{p}\Microsoft\Windows\Start Menu\Programs")),
    ];
    let mut best: Option<(usize, String)> = None;
    for root in roots.iter().flatten() {
        scan_shortcuts(Path::new(root), query, 0, &mut best);
    }
    best.map(|(_, path)| path)
}

fn scan_shortcuts(dir: &Path, query: &str, depth: u8, best: &mut Option<(usize, String)>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_shortcuts(&path, query, depth + 1, best);
            continue;
        }
        let is_lnk = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()).map(str::to_lowercase) else { continue };
        if !is_lnk || !stem.contains(query) || stem.contains("uninstall") {
            continue;
        }
        // Lower score is better: exact match wins, otherwise prefer the shortest name.
        let score = if stem == query { 0 } else { stem.len() };
        if best.as_ref().map_or(true, |(s, _)| score < *s) {
            *best = Some((score, path.to_string_lossy().into_owned()));
        }
    }
}

/// Microsoft Store apps have no .lnk shortcut; look them up by display name.
fn find_store_app(query: &str) -> Option<String> {
    if !query.chars().all(|c| c.is_alphanumeric() || " .-+".contains(c)) {
        return None;
    }
    let script = format!(
        "Get-StartApps | Where-Object {{ $_.Name -like '*{query}*' }} | Sort-Object {{ $_.Name.Length }} | Select-Object -First 1 -ExpandProperty AppID"
    );
    let output = crate::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    let app_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!app_id.is_empty()).then_some(app_id)
}
