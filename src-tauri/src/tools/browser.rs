use std::collections::HashMap;
use std::process::Command;
use crate::tools::{DesktopTool, ToolFuture};

pub struct OpenUrlTool;

impl DesktopTool for OpenUrlTool {
    fn name(&self) -> &str { "open_url" }
    fn description(&self) -> &str { "Open a specific URL in the default web browser." }
    fn parameter_schema(&self) -> &str { "{\"url\": \"https://...\"}" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to open in the browser"
                }
            },
            "required": ["url"]
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
            if let Some(url) = params.get("url").and_then(|v| v.as_str()) {
                let url_clean = url.trim();
                let final_url = if !url_clean.starts_with("http://") && !url_clean.starts_with("https://") {
                    let clean_lower = url_clean.to_lowercase();
                    match clean_lower.as_str() {
                        "youtube" => "https://www.youtube.com".to_string(),
                        "google" => "https://www.google.com".to_string(),
                        "github" => "https://www.github.com".to_string(),
                        "gmail" => "https://mail.google.com".to_string(),
                        "reddit" => "https://www.reddit.com".to_string(),
                        _ => format!("https://{}", url_clean),
                    }
                } else {
                    url_clean.to_string()
                };

                let _ = Command::new("cmd").args(&["/C", "start", "", &final_url]).spawn();
                Ok(format!("Opening {} in your browser.", url_clean))
            } else {
                Err("URL parameter is missing.".to_string())
            }
        })
    }
}

pub struct WebSearchTool;

impl DesktopTool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str { "Search the web using DuckDuckGo to find information." }
    fn parameter_schema(&self) -> &str { "{\"query\": \"search query\"}" }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            if let Some(query) = params.get("query").and_then(|v| v.as_str()) {
                let clean_query = query.trim();
                let encoded = urlencoding::encode(clean_query);
                let api_url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1", encoded);

                let res = client.get(&api_url)
                    .timeout(std::time::Duration::from_secs(6))
                    .send()
                    .await
                    .map_err(|e| format!("Web search request failed: {}", e))?;

                if let Ok(json_val) = res.json::<serde_json::Value>().await {
                    let mut summary = String::new();
                    if let Some(abstract_text) = json_val.get("AbstractText").and_then(|v| v.as_str()) {
                        if !abstract_text.trim().is_empty() {
                            summary = abstract_text.to_string();
                        }
                    }

                    if summary.is_empty() {
                        if let Some(related) = json_val.get("RelatedTopics").and_then(|v| v.as_array()) {
                            for item in related {
                                if let Some(txt) = item.get("Text").and_then(|v| v.as_str()) {
                                    if !txt.trim().is_empty() {
                                        summary = txt.to_string();
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if !summary.is_empty() {
                        Ok(format!("According to the web: {}", summary))
                    } else {
                        // Open search results in browser as fallback
                        let search_url = format!("https://duckduckgo.com/?q={}", encoded);
                        let _ = Command::new("cmd").args(&["/C", "start", "", &search_url]).spawn();
                        Ok(format!("I opened search results for '{}' in your browser.", clean_query))
                    }
                } else {
                    let search_url = format!("https://duckduckgo.com/?q={}", encoded);
                    let _ = Command::new("cmd").args(&["/C", "start", "", &search_url]).spawn();
                    Ok(format!("Opened search for '{}' in browser.", clean_query))
                }
            } else {
                Err("Search query parameter is missing.".to_string())
            }
        })
    }
}
