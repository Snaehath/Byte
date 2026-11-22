use std::collections::HashMap;
use crate::platform::windows::{press_key, vk};
use crate::tools::{coerce_to_u64, str_param, DesktopTool, ToolFuture};

pub struct SystemControlTool;

impl DesktopTool for SystemControlTool {
    fn name(&self) -> &str { "system_control" }
    fn description(&self) -> &str { "Change volume, mute, screen brightness, or control media playback (play/pause, next, previous)." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["volume_up", "volume_down", "mute", "set_brightness", "play_pause", "next_track", "previous_track"]
                },
                "value": {
                    "type": "integer",
                    "description": "Brightness percent 0-100 for set_brightness"
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
            let action = str_param(params, "action").ok_or("What should I change?")?;
            // Each volume key press moves Windows volume by 2%.
            match action {
                "volume_up" => { press_key(vk::VOLUME_UP, 5); Ok("Volume up.".to_string()) }
                "volume_down" => { press_key(vk::VOLUME_DOWN, 5); Ok("Volume down.".to_string()) }
                "mute" => { press_key(vk::VOLUME_MUTE, 1); Ok("Toggled mute.".to_string()) }
                "play_pause" => { press_key(vk::MEDIA_PLAY_PAUSE, 1); Ok("Done.".to_string()) }
                "next_track" => { press_key(vk::MEDIA_NEXT, 1); Ok("Next track.".to_string()) }
                "previous_track" => { press_key(vk::MEDIA_PREV, 1); Ok("Previous track.".to_string()) }
                "set_brightness" => {
                    let level = params.get("value").and_then(coerce_to_u64)
                        .ok_or("What brightness level, from 0 to 100?")?
                        .min(100);
                    set_brightness(level).await?;
                    Ok(format!("Brightness set to {level} percent."))
                }
                other => Err(format!("Unknown system action: {other}")),
            }
        })
    }
}

async fn set_brightness(level: u64) -> Result<(), String> {
    let script = format!(
        "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop | Select-Object -First 1 | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{Timeout=1; Brightness={level}}}) | Out-Null"
    );
    let output = tokio::task::spawn_blocking(move || {
        crate::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
    })
    .await
    .map_err(|e| format!("Brightness task failed: {e}"))?
    .map_err(|e| format!("Couldn't run brightness command: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("This display doesn't support brightness control from Windows.".to_string())
    }
}

#[derive(serde::Deserialize, Debug, Default)]
struct RawSystemStats {
    cpu: Option<u64>,
    free: Option<u64>,
    total: Option<u64>,
    days: Option<u64>,
    hours: Option<u64>,
    minutes: Option<u64>,
}

pub struct GetSystemStatsTool;

impl DesktopTool for GetSystemStatsTool {
    fn name(&self) -> &str { "get_system_stats" }
    fn description(&self) -> &str { "Get current CPU usage, memory (RAM) usage, and system uptime." }
    fn answer_from_result(&self) -> bool { true }

    fn execute<'a>(
        &self,
        _params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let stats = tokio::task::spawn_blocking(|| {
                let script = r#"$cpu = (Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average; $os = Get-CimInstance Win32_OperatingSystem; $up = (Get-Date) - $os.LastBootUpTime; @{ cpu = [int]$cpu; free = $os.FreePhysicalMemory; total = $os.TotalVisibleMemorySize; days = [int]$up.Days; hours = [int]$up.Hours; minutes = [int]$up.Minutes } | ConvertTo-Json -Compress"#;
                crate::Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", script])
                    .output()
                    .ok()
                    .filter(|out| out.status.success())
                    .and_then(|out| serde_json::from_slice::<RawSystemStats>(&out.stdout).ok())
                    .unwrap_or_default()
            })
            .await
            .map_err(|e| format!("System stats task failed: {e}"))?;

            let mut parts = Vec::new();
            if let Some(cpu) = stats.cpu {
                parts.push(format!("CPU {cpu}%"));
            }
            if let (Some(free), Some(total)) = (stats.free, stats.total) {
                if total > 0 {
                    let used = total.saturating_sub(free);
                    parts.push(format!(
                        "RAM {:.0}% ({:.1} of {:.1} GB used)",
                        used as f64 / total as f64 * 100.0,
                        used as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ));
                }
            }
            if let (Some(d), Some(h), Some(m)) = (stats.days, stats.hours, stats.minutes) {
                parts.push(format!("uptime {d} days {h} hours {m} minutes"));
            }
            if parts.is_empty() {
                return Err("I couldn't read system stats.".to_string());
            }
            Ok(format!("System stats: {}.", parts.join(", ")))
        })
    }
}
