use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::config::paths::BytePaths;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LlmConfig {
    pub provider: String,
    pub ollama_url: String,        // default "http://localhost:11434/v1/chat/completions"
    pub ollama_model: String,      // default "granite4.2:3b"
    pub vision_model: String,
    pub cloud_url: String,
    pub cloud_model: String,
    pub cloud_vision_model: String,
    pub setup_complete: bool,
    pub reasoning_effort: String,  // "low" for Granite fast answering
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            ollama_url: "http://localhost:11434/v1/chat/completions".to_string(),
            ollama_model: "granite4.2:3b".to_string(),
            vision_model: "llava".to_string(),
            cloud_url: "https://api.openai.com/v1/chat/completions".to_string(),
            cloud_model: String::new(),
            cloud_vision_model: String::new(),
            setup_complete: false,
            reasoning_effort: "low".to_string(),
            temperature: 0.2,
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
#[serde(default)]
pub struct SpeechToolPaths {
    pub whisper_exe: String,
    pub whisper_model: String,
    pub piper_exe: String,
    pub piper_model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub audio: AudioConfig,
    pub speech_tools: SpeechToolPaths,
    pub wake_word_enabled: bool,
    pub first_run_greeting_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            audio: AudioConfig::default(),
            speech_tools: SpeechToolPaths::default(),
            wake_word_enabled: true,
            first_run_greeting_enabled: false,
        }
    }
}

impl AppConfig {
    pub fn config_file_path() -> PathBuf {
        BytePaths::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_file_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&content) {
                    if cfg.llm.ollama_model == "qwen3-4b:latest" || cfg.llm.ollama_model.is_empty() {
                        cfg.llm.ollama_model = "granite4.2:3b".to_string();
                        let _ = cfg.save();
                    }
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
