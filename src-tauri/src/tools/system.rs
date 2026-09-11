use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture, coerce_to_u64};

pub struct SystemControlTool;

impl DesktopTool for SystemControlTool {
    fn name(&self) -> &str { "system_control" }
    fn description(&self) -> &str { "Adjust system volume, brightness, or media playback tracks." }
    fn parameter_schema(&self) -> &str { "{\"action\": \"volume_up\" | \"volume_down\" | \"mute\" | \"set_brightness\" | \"play_pause\" | \"next_track\" | \"previous_track\", \"value\": 0-100}" }

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

pub struct GetSystemStatsTool;

impl DesktopTool for GetSystemStatsTool {
    fn name(&self) -> &str { "get_system_stats" }
    fn description(&self) -> &str { "Retrieve current system CPU utilization, RAM usage, and uptime." }
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
            let cpu_output = Command::new("powershell")
                .args(&["-Command", "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty LoadPercentage"])
                .output();
            let ram_output = Command::new("powershell")
                .args(&["-Command", "Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory, TotalVisibleMemorySize | ConvertTo-Json"])
                .output();
            let uptime_output = Command::new("powershell")
                .args(&["-Command", "$os = Get-CimInstance Win32_OperatingSystem; $uptime = (Get-Date) - $os.LastBootUpTime; \"$($uptime.Days) days, $($uptime.Hours) hours, $($uptime.Minutes) minutes\""])
                .output();

            let cpu_percentage = if let Ok(out) = cpu_output {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                "unknown".to_string()
            };

            let mut ram_info = "unknown".to_string();
            if let Ok(out) = ram_output {
                let out_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&out_str) {
                    let free = val.get("FreePhysicalMemory").and_then(|v| v.as_u64()).unwrap_or(0);
                    let total = val.get("TotalVisibleMemorySize").and_then(|v| v.as_u64()).unwrap_or(0);
                    if total > 0 {
                        let used = total - free;
                        let percent = (used as f64 / total as f64) * 100.0;
                        ram_info = format!("{:.1}% ({:.1} GB used out of {:.1} GB)", percent, used as f64 / 1048576.0, total as f64 / 1048576.0);
                    }
                }
            }

            let uptime_info = if let Ok(out) = uptime_output {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                "unknown".to_string()
            };

            Ok(format!(
                "System stats: CPU at {}%, RAM at {}, and uptime is {}.",
                cpu_percentage, ram_info, uptime_info
            ))
        })
    }
}
