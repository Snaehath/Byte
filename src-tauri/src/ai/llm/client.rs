use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct OpenAiClient;

impl OpenAiClient {
    pub async fn chat_completion(
        client: &reqwest::Client,
        endpoint_url: &str,
        api_key: &str,
        model: &str,
        messages: &[ChatMessage],
        temperature: f32,
    ) -> Result<String, String> {
        let mut req = client
            .post(endpoint_url)
            .timeout(std::time::Duration::from_secs(45));

        if !api_key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key.trim()));
        }

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
        });

        let res = req.json(&body).send().await
            .map_err(|e| format!("Network request failed to {}: {}", endpoint_url, e))?;

        if res.status().is_success() {
            let val: serde_json::Value = res.json().await
                .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

            if let Some(choices) = val.get("choices").and_then(|v| v.as_array()) {
                if let Some(first) = choices.first() {
                    if let Some(content) = first.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                        return Ok(content.to_string());
                    }
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
