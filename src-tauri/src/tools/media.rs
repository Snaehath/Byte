use std::collections::HashMap;
use crate::platform::windows::shell_open;
use crate::tools::{str_param, DesktopTool, ToolFuture};

pub struct PlayMediaTool;

impl DesktopTool for PlayMediaTool {
    fn name(&self) -> &str { "play_media" }
    fn description(&self) -> &str {
        "Play a song, artist, album, genre, or video on YouTube in the browser. Use for requests like \"play some music\" or \"play lofi on YouTube\"."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to play, e.g. \"Blinding Lights The Weeknd\" or \"lofi hip hop\". Use \"popular music mix\" if the user didn't say."
                }
            },
            "required": ["query"]
        })
    }
    fn timeout_secs(&self) -> u64 { 15 }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let query = str_param(params, "query").unwrap_or("popular music mix");
            let encoded = urlencoding::encode(query);
            let results_url = format!("https://www.youtube.com/results?search_query={encoded}");

            match first_video_id(client, &results_url).await {
                Some(id) => {
                    shell_open(&format!("https://www.youtube.com/watch?v={id}"), None)?;
                    Ok(format!("Playing {query} on YouTube."))
                }
                None => {
                    shell_open(&results_url, None)?;
                    Ok(format!("I opened YouTube results for {query}."))
                }
            }
        })
    }
}

/// Find the first video in a YouTube search results page.
async fn first_video_id(client: &reqwest::Client, results_url: &str) -> Option<String> {
    // Videos-only filter, so the top hit isn't a channel or playlist.
    let url = format!("{results_url}&sp=EgIQAQ%253D%253D");
    let html = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    extract_video_id(&html)
}

fn extract_video_id(html: &str) -> Option<String> {
    const MARKER: &str = "\"videoId\":\"";
    html.match_indices(MARKER).find_map(|(i, _)| {
        let id: String = html[i + MARKER.len()..].chars().take(11).collect();
        let valid = id.len() == 11 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        valid.then_some(id)
    })
}

#[cfg(test)]
mod tests {
    use super::extract_video_id;

    #[test]
    fn test_extract_video_id() {
        let html = r#"..."videoId":"bad"..."videoId":"dQw4w9WgXcQ","thumbnail"..."#;
        assert_eq!(extract_video_id(html).as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(extract_video_id("no results"), None);
    }
}
