use base64::Engine;
use std::io::Cursor;
use crate::config::paths::BytePaths;
use crate::Command;

/// Longest image side sent to the vision model. Full 4K screenshots are slow
/// to encode and process without improving answers.
const MAX_IMAGE_SIDE: u32 = 1344;

const VISION_INSTRUCTIONS: &str = "You are looking at a screenshot of the user's Windows screen. \
Answer the user's question about it in at most three short, plain sentences suitable for speaking aloud. \
No markdown, lists, or emojis. If text on screen is relevant, quote only the important part.";

pub struct VisionService;

impl VisionService {
    /// Capture the primary screen, downscale it, and return a JPEG data URL.
    fn capture_screen_data_url() -> Result<String, String> {
        let temp_png = BytePaths::temp_dir().join(format!("screen_{}.png", uuid::Uuid::new_v4()));
        let script = format!(
            "Add-Type -AssemblyName System.Drawing, System.Windows.Forms; \
             $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \
             $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height; \
             $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.X, $b.Y, 0, 0, $bmp.Size); \
             $bmp.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()",
            temp_png.to_string_lossy().replace('\'', "''")
        );
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("Couldn't capture the screen: {e}"))?;
        if !output.status.success() {
            return Err(format!("Screen capture failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
        }

        let image = image::open(&temp_png);
        let _ = std::fs::remove_file(&temp_png);
        let image = image.map_err(|e| format!("Couldn't read the screenshot: {e}"))?;
        let image = if image.width().max(image.height()) > MAX_IMAGE_SIDE {
            image.resize(MAX_IMAGE_SIDE, MAX_IMAGE_SIDE, image::imageops::FilterType::Triangle)
        } else {
            image
        };

        let mut jpeg = Vec::new();
        image
            .to_rgb8()
            .write_to(&mut Cursor::new(&mut jpeg), image::ImageOutputFormat::Jpeg(85))
            .map_err(|e| format!("Couldn't encode the screenshot: {e}"))?;
        Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(&jpeg)))
    }

    /// Start loading the vision model into memory so the real request is fast.
    /// Fire-and-forget: errors are only logged.
    pub fn warm_up(client: reqwest::Client, chat_endpoint: String, model: String) {
        tauri::async_runtime::spawn(async move {
            let root = chat_endpoint.trim_end_matches('/').trim_end_matches("/v1/chat/completions").to_string();
            let res = client
                .post(format!("{root}/api/generate"))
                .timeout(std::time::Duration::from_secs(110))
                .json(&serde_json::json!({ "model": model, "keep_alive": "5m" }))
                .send()
                .await;
            match res {
                Ok(r) if r.status().is_success() => log::info!("Vision model {model} is loaded."),
                Ok(r) => log::warn!("Vision warm-up returned {}", r.status()),
                Err(e) => log::warn!("Vision warm-up failed: {e}"),
            }
        });
    }

    /// Ask the selected vision model (local or cloud) about the current screen.
    pub async fn analyze_screen(
        client: &reqwest::Client,
        target: &crate::ai::ModelTarget,
        query: &str,
    ) -> Result<String, String> {
        let image_url = tokio::task::spawn_blocking(Self::capture_screen_data_url)
            .await
            .map_err(|e| format!("Screen capture task failed: {e}"))??;

        let payload = serde_json::json!({
            "model": target.model,
            "messages": [
                { "role": "system", "content": VISION_INSTRUCTIONS },
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": query },
                        { "type": "image_url", "image_url": { "url": image_url } }
                    ]
                }
            ]
        });
        let val = crate::ai::llm::post_chat(client, target, payload, 0.2, 110).await?;
        val.pointer("/choices/0/message/content")
            .and_then(|c| c.as_str())
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .ok_or_else(|| "The vision model returned an empty answer.".to_string())
    }
}
