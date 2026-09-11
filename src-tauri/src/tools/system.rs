use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture, coerce_to_u64};

pub struct SystemControlTool;

impl DesktopTool for SystemControlTool {
    fn name(&self) -> &str { "system_control" }
    fn description(&self) -> &str { "Adjust system volume, brightness, or media playback tracks." }
    fn parameter_schema(&self) -> &str { "{\"action\": \"volume_up\" | \"volume_down\" | \"mute\" | \"set_brightness\" | \"play_pause\" | \"next_track\" | \"previous_track\", \"value\": 0-100}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["volume_up", "volume_down", "mute", "set_brightness", "play_pause", "next_track", "previous_track"],
                    "description": "System control action to perform"
                },
                "value": {
                    "type": "integer",
                    "description": "Optional numeric value from 0 to 100 (for brightness)"
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
                match action {
                    "volume_up" => {
                        let _ = Command::new("powershell")
                            .args(&["-Command", "for ($i=0; $i -lt 5; $i++) { (New-Object -ComObject Wscript.Shell).SendKeys([char]175) }"])
                            .spawn();
                        Ok("Volume turned up.".to_string())
                    }
                    "volume_down" => {
                        let _ = Command::new("powershell")
                            .args(&["-Command", "for ($i=0; $i -lt 5; $i++) { (New-Object -ComObject Wscript.Shell).SendKeys([char]174) }"])
                            .spawn();
                        Ok("Volume turned down.".to_string())
                    }
                    "mute" => {
                        let _ = Command::new("powershell")
                            .args(&["-Command", "(New-Object -ComObject Wscript.Shell).SendKeys([char]173)"])
                            .spawn();
                        Ok("Toggled volume mute.".to_string())
                    }
                    "set_brightness" => {
                        let brightness = params.get("value")
                            .and_then(coerce_to_u64)
                            .unwrap_or(50);
                        let cmd_str = format!("(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1, {})", brightness);
                        let _ = Command::new("powershell")
                            .args(&["-Command", &cmd_str])
                            .spawn();
                        Ok(format!("Brightness set to {}%.", brightness))
                    }
                    "play_pause" => {
                        let _ = Command::new("powershell")
                            .args(&["-Command", "(New-Object -ComObject Wscript.Shell).SendKeys([char]179)"])
                            .spawn();
                        Ok("Toggled media playback.".to_string())
                    }
                    "next_track" => {
                        let _ = Command::new("powershell")
                            .args(&["-Command", "(New-Object -ComObject Wscript.Shell).SendKeys([char]176)"])
                            .spawn();
                        Ok("Playing next track.".to_string())
                    }
                    "previous_track" => {
                        let _ = Command::new("powershell")
                            .args(&["-Command", "(New-Object -ComObject Wscript.Shell).SendKeys([char]177)"])
                            .spawn();
                        Ok("Playing previous track.".to_string())
                    }
                    _ => Err(format!("Unknown system control action: {}", action)),
                }
            } else {
                Err("System control action parameter is missing.".to_string())
            }
        })
    }
}

#[derive(serde::Deserialize, Debug, Default)]
struct RawSystemStats {
    #[serde(default)]
    cpu: Option<u64>,
    #[serde(default)]
    free: Option<u64>,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    days: Option<u64>,
    #[serde(default)]
    hours: Option<u64>,
    #[serde(default)]
    minutes: Option<u64>,
}

pub struct GetSystemStatsTool;

impl DesktopTool for GetSystemStatsTool {
    fn name(&self) -> &str { "get_system_stats" }
    fn description(&self) -> &str { "Retrieve current system CPU utilization, RAM usage, and uptime in a single fast call." }
    fn parameter_schema(&self) -> &str { "{}" }

    fn execute<'a>(
        &self,
        _params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let stats = tokio::task::spawn_blocking(move || {
                let ps_script = r#"$cpu = (Get-CimInstance Win32_Processor).LoadPercentage; $os = Get-CimInstance Win32_OperatingSystem; $up = (Get-Date) - $os.LastBootUpTime; @{ cpu = $cpu; free = $os.FreePhysicalMemory; total = $os.TotalVisibleMemorySize; days = [int]$up.Days; hours = [int]$up.Hours; minutes = [int]$up.Minutes } | ConvertTo-Json -Compress"#;
                let output = Command::new("powershell")
                    .args(&["-NoProfile", "-NonInteractive", "-Command", ps_script])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        serde_json::from_str::<RawSystemStats>(&stdout).unwrap_or_default()
                    }
                    _ => RawSystemStats::default(),
                }
            }).await.map_err(|e| format!("System stats task failed: {}", e))?;

            let cpu_pct = stats.cpu.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
            let ram_info = match (stats.free, stats.total) {
                (Some(free), Some(total)) if total > 0 => {
                    let used = total.saturating_sub(free);
                    let percent = (used as f64 / total as f64) * 100.0;
                    format!("{:.1}% ({:.1} GB used out of {:.1} GB)", percent, used as f64 / 1048576.0, total as f64 / 1048576.0)
                }
                _ => "unknown".to_string(),
            };
            let uptime_info = match (stats.days, stats.hours, stats.minutes) {
                (Some(d), Some(h), Some(m)) => format!("{} days, {} hours, {} minutes", d, h, m),
                _ => "unknown".to_string(),
            };

            Ok(format!(
                "System stats: CPU at {}%, RAM at {}, and uptime is {}.",
                cpu_pct, ram_info, uptime_info
            ))
        })
    }
}
