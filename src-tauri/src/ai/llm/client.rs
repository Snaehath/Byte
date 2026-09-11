use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::tools::ToolCall;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct LlmResult {
    pub content: String,
    pub tool_call: Option<ToolCall>,
}

pub struct OpenAiClient;

impl OpenAiClient {
    pub async fn chat_completion(
        client: &reqwest::Client,
        endpoint_url: &str,
        api_key: &str,
        model: &str,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
        temperature: f32,
        reasoning_effort: &str,
    ) -> Result<LlmResult, String> {
        let mut req = client
            .post(endpoint_url)
            .timeout(std::time::Duration::from_secs(45));

        if !api_key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "options": {
                "temperature": temperature
            }
        });

        if let Some(tool_list) = tools {
            if !tool_list.is_empty() {
                body["tools"] = serde_json::Value::Array(tool_list.to_vec());
            }
        }

        if !reasoning_effort.is_empty() {
            body["reasoning_effort"] = serde_json::Value::String(reasoning_effort.to_string());
        }

        let res = req.json(&body).send().await
            .map_err(|e| format!("Network request failed to {}: {}", endpoint_url, e))?;

        if res.status().is_success() {
            let val: serde_json::Value = res.json().await
                .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

            if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
                if let Some(first) = choices.first() {
                    let message = first.get("message");

                    // 1. Native OpenAI / Ollama tool_calls parsing
                    if let Some(tool_calls) = message.and_then(|m| m.get("tool_calls")).and_then(|tc| tc.as_array()) {
                        if let Some(tc) = tool_calls.first() {
                            if let Some(func) = tc.get("function") {
                                let name = func.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_string();
                                let mut params = HashMap::new();
                                if let Some(args_str) = func.get("arguments").and_then(|a| a.as_str()) {
                                    if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(args_str) {
                                        params = map;
                                    }
                                } else if let Some(args_obj) = func.get("arguments").and_then(|a| a.as_object()) {
                                    for (k, v) in args_obj {
                                        params.insert(k.clone(), v.clone());
                                    }
                                }
                                if !name.is_empty() {
                                    return Ok(LlmResult {
                                        content: String::new(),
                                        tool_call: Some(ToolCall {
                                            tool: name,
                                            parameters: params,
                                        }),
                                    });
                                }
                            }
                        }
                    }

                    // 2. Free-text response with fallback JSON tool call parser
                    let content = message.and_then(|m| m.get("content")).and_then(|c| c.as_str()).unwrap_or_default().to_string();
                    let fallback_tool = crate::tools::parse_tool_call(&content);
                    return Ok(LlmResult {
                        content,
                        tool_call: fallback_tool,
                    });
                }
            }
            Err("No valid message content found in choices array".to_string())
        } else {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            Err(format!("Chat completion error (status {}): {}", status, text))
        }
    }
}
