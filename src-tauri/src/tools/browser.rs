use std::collections::HashMap;
use crate::platform::windows::shell_open;
use crate::tools::{str_param, DesktopTool, ToolFuture};

pub struct OpenUrlTool;

impl DesktopTool for OpenUrlTool {
    fn name(&self) -> &str { "open_url" }
    fn description(&self) -> &str { "Open a website in the default browser, e.g. youtube.com or a full URL." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "Website address or well-known site name, e.g. \"github.com\" or \"youtube\""
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
            let raw = str_param(params, "url").ok_or("Which website should I open?")?;
            let url = normalize_url(raw)?;
            shell_open(&url, None)?;
            Ok(format!("Opening {}.", display_host(&url)))
        })
    }
}

fn normalize_url(raw: &str) -> Result<String, String> {
    let lower = raw.to_lowercase();
    let known = match lower.as_str() {
        "youtube" => Some("https://www.youtube.com"),
        "google" => Some("https://www.google.com"),
        "github" => Some("https://github.com"),
        "gmail" => Some("https://mail.google.com"),
        "reddit" => Some("https://www.reddit.com"),
        "wikipedia" => Some("https://www.wikipedia.org"),
        _ => None,
    };
    if let Some(url) = known {
        return Ok(url.to_string());
    }
    if raw.chars().any(char::is_whitespace) {
        return Err("That doesn't look like a web address.".to_string());
    }
    let url = if lower.starts_with("https://") || lower.starts_with("http://") {
        raw.to_string()
    } else if lower.contains("://") {
        // Only web links; never hand other protocol handlers to the shell.
        return Err("I can only open web links.".to_string());
    } else {
        format!("https://{raw}")
    };
    if !url.split("://").nth(1).is_some_and(|rest| rest.split('/').next().is_some_and(|host| host.contains('.'))) {
        return Err("That doesn't look like a web address.".to_string());
    }
    Ok(url)
}

fn display_host(url: &str) -> &str {
    url.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or(url).trim_start_matches("www.")
}

pub struct WebSearchTool;

impl DesktopTool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str {
        "Look up facts on the web. Returns a short summary, or opens search results in the browser when no summary is available."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        })
    }
    fn answer_from_result(&self) -> bool { true }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        client: &'a reqwest::Client,
        _transcription: &'a str,
        _state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let query = str_param(params, "query").ok_or("What should I search for?")?;
            let encoded = urlencoding::encode(query);
            let api_url = format!("https://api.duckduckgo.com/?q={encoded}&format=json&no_html=1&skip_disambig=1");

            let summary = match client.get(&api_url).timeout(std::time::Duration::from_secs(6)).send().await {
                Ok(res) => res.json::<serde_json::Value>().await.ok().and_then(|json| instant_answer(&json)),
                Err(e) => {
                    log::warn!("web_search: instant answer request failed: {e}");
                    None
                }
            };

            match summary {
                Some(text) => Ok(format!("Web summary for \"{query}\": {text}")),
                None => {
                    shell_open(&format!("https://duckduckgo.com/?q={encoded}"), None)?;
                    Ok(format!("I opened search results for {query} in your browser."))
                }
            }
        })
    }
}

fn instant_answer(json: &serde_json::Value) -> Option<String> {
    let non_empty = |v: &serde_json::Value| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    json.get("AbstractText").and_then(non_empty)
        .or_else(|| json.get("Answer").and_then(non_empty))
        .or_else(|| {
            json.get("RelatedTopics")?.as_array()?.iter().find_map(|item| item.get("Text").and_then(non_empty))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("youtube").unwrap(), "https://www.youtube.com");
        assert_eq!(normalize_url("example.com/a?b=1&c=2").unwrap(), "https://example.com/a?b=1&c=2");
        assert!(normalize_url("file:///C:/Windows").is_err());
        assert!(normalize_url("not a url").is_err());
        assert!(normalize_url("localhost").is_err());
    }
}
