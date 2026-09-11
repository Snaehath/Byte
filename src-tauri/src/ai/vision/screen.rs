use base64::Engine;
use crate::config::paths::BytePaths;
use crate::Command;

pub struct VisionService;

impl VisionService {
    /// Captures the primary screen into a temporary PNG and returns Base64 data URL
    pub fn capture_screen_base64() -> Result<String, String> {
        let temp_screenshot = BytePaths::temp_dir().join("screenshot_temp.png");

        let cmd_str = format!(
            r#"[void] [System.Reflection.Assembly]::LoadWithPartialName('System.Drawing'); [void] [System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height; $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.X, $b.Y, 0, 0, $bmp.Size); $bmp.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose();"#,
            temp_screenshot.to_string_lossy().replace("'", "''")
        );

        let output = Command::new("powershell")
            .args(&["-Command", &cmd_str])
            .output()
            .map_err(|e| format!("Failed to run screen capture command: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Screenshot capture failed: {}", err));
        }

        let img_bytes = std::fs::read(&temp_screenshot)
            .map_err(|e| format!("Failed to read captured screenshot: {}", e))?;

        let _ = std::fs::remove_file(&temp_screenshot);

        let base64_image = base64::engine::general_purpose::STANDARD.encode(&img_bytes);
        Ok(format!("data:image/png;base64,{}", base64_image))
    }

    /// Queries a multimodal vision model with prompt and image
    pub async fn analyze_screen(
        client: &reqwest::Client,
        endpoint_url: &str,
        api_key: &str,
        model: &str,
        query: &str,
    ) -> Result<String, String> {
        let image_url = Self::capture_screen_base64()?;

        let mut req = client.post(endpoint_url).timeout(std::time::Duration::from_secs(30));
        if !api_key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
        }

        let payload = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": query
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": image_url
                            }
                        }
                    ]
                }
            ]
        });

        let res = req.json(&payload).send().await
            .map_err(|e| format!("Vision API request failed: {}", e))?;

        if res.status().is_success() {
            let val: serde_json::Value = res.json().await
                .map_err(|e| format!("Failed to parse vision response JSON: {}", e))?;

            if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
                if let Some(first) = choices.first() {
                    if let Some(content) = first.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                        return Ok(content.to_string());
                    }
                }
            }
            Err("No content in vision response choices".to_string())
        } else {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            Err(format!("Vision request failed (status {}): {}", status, text))
        }
    }
}
