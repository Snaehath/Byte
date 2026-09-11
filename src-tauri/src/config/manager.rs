use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::config::paths::BytePaths;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LlmConfig {
    pub provider: String,          // "ollama" | "openai_compatible"
    pub ollama_url: String,        // default "http://localhost:11434/v1/chat/completions"
    pub ollama_model: String,      // default "qwen3-4b:latest"
    pub cloud_url: String,         // OpenRouter or Nvidia URL
    pub cloud_model: String,
    pub cloud_api_key: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            ollama_url: "http://localhost:11434/v1/chat/completions".to_string(),
            ollama_model: "qwen3-4b:latest".to_string(),
            cloud_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
            cloud_model: "google/gemini-flash-1.5".to_string(),
            cloud_api_key: String::new(),
            temperature: 0.7,
            max_tokens: 1024,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub silence_duration_ms: u64,
    pub min_recording_ms: u64,
    pub voice_speed: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            silence_duration_ms: 1500,
            min_recording_ms: 1000,
            voice_speed: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub audio: AudioConfig,
}

impl AppConfig {
    pub fn config_file_path() -> PathBuf {
        BytePaths::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_file_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        let default_cfg = AppConfig::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_file_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }
}
