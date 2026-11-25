use crate::ai::manager::ModelTarget;
use crate::tools::ToolCall;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: &str, content: impl Into<String>) -> Self {
        Self { role: role.to_string(), content: content.into() }
    }
}

#[derive(Debug, Clone)]
pub struct LlmResult {
    pub content: String,
    pub tool_call: Option<ToolCall>,
}

/// Per-model request settings for cloud APIs. Models differ in which
/// parameters they accept, so these start from a guess and are corrected
/// (and remembered) when the API rejects a parameter.
#[derive(Clone, Debug, PartialEq)]
struct CloudQuirks {
    send_temperature: bool,
    reasoning_effort: Option<String>,
}

impl CloudQuirks {
    fn guess(model: &str) -> Self {
        if is_reasoning_model(model) {
            // Reasoning models reject custom temperatures, and only allow function
            // tools on /chat/completions with reasoning off. "none" is also the
            // fastest option, which suits a voice assistant.
            Self { send_temperature: false, reasoning_effort: Some("none".to_string()) }
        } else {
            Self { send_temperature: true, reasoning_effort: None }
        }
    }

    fn apply(&self, body: &mut serde_json::Value, temperature: f32) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("temperature");
            obj.remove("reasoning_effort");
        }
        if self.send_temperature {
            body["temperature"] = temperature.into();
        }
        if let Some(effort) = &self.reasoning_effort {
            body["reasoning_effort"] = effort.clone().into();
        }
    }

    /// The next settings to try after a 400 error, if the error names a parameter we control.
    fn adjust(&self, error_body: &str) -> Option<Self> {
        let parsed: serde_json::Value = serde_json::from_str(error_body).unwrap_or_default();
        let param = parsed.pointer("/error/param").and_then(|p| p.as_str()).unwrap_or_default();
        let message = parsed
            .pointer("/error/message")
            .and_then(|m| m.as_str())
            .unwrap_or(error_body)
            .to_lowercase();
        let mut next = self.clone();
        if param == "reasoning_effort" || message.contains("reasoning_effort") {
            next.reasoning_effort = match self.reasoning_effort.as_deref() {
                Some("none") => None,
                _ => Some("none".to_string()),
            };
        } else if (param == "temperature" || message.contains("temperature")) && self.send_temperature {
            next.send_temperature = false;
        }
        (next != *self).then_some(next)
    }
}

/// o-series and gpt-5+ models reason by default.
fn is_reasoning_model(model: &str) -> bool {
    let m = model.trim().to_lowercase();
    if let Some(rest) = m.strip_prefix("gpt-") {
        let major: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return major.parse::<u32>().is_ok_and(|v| v >= 5);
    }
    m.strip_prefix('o').is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
}

fn quirks_cache() -> &'static std::sync::Mutex<HashMap<String, CloudQuirks>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<String, CloudQuirks>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Attach the auth header for cloud targets.
pub fn authorize(req: reqwest::RequestBuilder, target: &ModelTarget) -> reqwest::RequestBuilder {
    match &target.api_key {
        Some(key) => req.bearer_auth(key),
        None => req,
    }
}

/// POST a chat request and return the response JSON. For cloud targets,
/// parameters the model rejects are adjusted and the request retried.
pub async fn post_chat(
    client: &reqwest::Client,
    target: &ModelTarget,
    mut body: serde_json::Value,
    temperature: f32,
    timeout_secs: u64,
) -> Result<serde_json::Value, String> {
    let send = |body: &serde_json::Value| {
        let req = client.post(&target.url).timeout(std::time::Duration::from_secs(timeout_secs));
        authorize(req, target).json(body).send()
    };
    let failed = |e: reqwest::Error| format!("Network request failed to {}: {}", target.url, e);

    if !target.is_cloud() {
        body["temperature"] = temperature.into();
        let res = send(&body).await.map_err(failed)?;
        return read_response(res).await;
    }

    let cached = quirks_cache().lock().ok().and_then(|c| c.get(&target.model).cloned());
    let mut quirks = cached.unwrap_or_else(|| CloudQuirks::guess(&target.model));
    let mut attempt = 0;
    loop {
        attempt += 1;
        quirks.apply(&mut body, temperature);
        let res = send(&body).await.map_err(failed)?;
        if res.status().as_u16() == 400 {
            let text = res.text().await.unwrap_or_default();
            match quirks.adjust(&text) {
                Some(next) if attempt < 3 => {
                    log::warn!("{} rejected {:?}; retrying with {:?}", target.model, quirks, next);
                    quirks = next;
                    continue;
                }
                _ => return Err(format!("Chat completion error (status 400 Bad Request): {text}")),
            }
        }
        let result = read_response(res).await;
        if result.is_ok() {
            if let Ok(mut cache) = quirks_cache().lock() {
                cache.insert(target.model.clone(), quirks);
            }
        }
        return result;
    }
}

async fn read_response(res: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = res.status();
    if status.is_success() {
        res.json().await.map_err(|e| format!("Failed to parse response JSON: {e}"))
    } else {
        let text = res.text().await.unwrap_or_default();
        Err(format!("Chat completion error (status {status}): {text}"))
    }
}

/// Call an OpenAI-compatible chat endpoint (local Ollama or a cloud API).
pub async fn chat_completion(
    client: &reqwest::Client,
    target: &ModelTarget,
    messages: &[ChatMessage],
    tools: Option<&[serde_json::Value]>,
    temperature: f32,
    max_tokens: u32,
    reasoning_effort: &str,
) -> Result<LlmResult, String> {
    let mut body = serde_json::json!({
        "model": target.model,
        "messages": messages,
    });
    if target.is_cloud() {
        body["max_completion_tokens"] = max_tokens.into();
    } else {
        body["max_tokens"] = max_tokens.into();
        if !reasoning_effort.is_empty() {
            body["reasoning_effort"] = reasoning_effort.into();
        }
    }
    if let Some(tool_list) = tools.filter(|t| !t.is_empty()) {
        body["tools"] = serde_json::Value::Array(tool_list.to_vec());
    }

    let val = post_chat(client, target, body, temperature, 60).await?;
    let message = val
        .pointer("/choices/0/message")
        .ok_or_else(|| "No valid message content found in choices array".to_string())?;

    // 1. Native tool_calls
    if let Some(func) = message.pointer("/tool_calls/0/function") {
        let name = func.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string();
        let parameters = match func.get("arguments") {
            Some(serde_json::Value::String(args)) => serde_json::from_str(args).unwrap_or_default(),
            Some(serde_json::Value::Object(args)) => args.clone().into_iter().collect(),
            _ => HashMap::new(),
        };
        if !name.is_empty() {
            return Ok(LlmResult { content: String::new(), tool_call: Some(ToolCall { tool: name, parameters }) });
        }
    }

    // 2. Free text, with a fallback parser for models that write tool calls as JSON
    let content = message.get("content").and_then(|c| c.as_str()).unwrap_or_default().to_string();
    let tool_call = crate::tools::parse_tool_call(&content);
    Ok(LlmResult { content, tool_call })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_model_detection() {
        for m in ["gpt-5", "gpt-5-mini", "gpt-6.1-terra", "o3", "o4-mini"] {
            assert!(is_reasoning_model(m), "{m}");
        }
        for m in ["gpt-4.1-mini", "gpt-4o", "gpt-3.5-turbo", "omni-moderation"] {
            assert!(!is_reasoning_model(m), "{m}");
        }
    }

    #[test]
    fn test_quirks_adjust_from_api_errors() {
        let tools_error = r#"{"error":{"message":"Function tools with reasoning_effort are not supported for gpt-6.1-terra in /v1/chat/completions. To use function tools, use /v1/responses or set reasoning_effort to 'none'.","param":"reasoning_effort"}}"#;
        let start = CloudQuirks { send_temperature: false, reasoning_effort: None };
        assert_eq!(start.adjust(tools_error).unwrap().reasoning_effort.as_deref(), Some("none"));

        let temp_error = r#"{"error":{"message":"Unsupported value: 'temperature' does not support 0.2","param":"temperature"}}"#;
        assert!(!CloudQuirks::guess("gpt-4.1").adjust(temp_error).unwrap().send_temperature);

        let other = r#"{"error":{"message":"other","param":"messages"}}"#;
        assert!(CloudQuirks::guess("gpt-4.1").adjust(other).is_none());
    }

    #[test]
    fn test_guess_for_reasoning_model_disables_reasoning() {
        let mut body = serde_json::json!({ "model": "gpt-6.1-terra" });
        CloudQuirks::guess("gpt-6.1-terra").apply(&mut body, 0.2);
        assert_eq!(body["reasoning_effort"], "none");
        assert!(body.get("temperature").is_none());
    }
}
