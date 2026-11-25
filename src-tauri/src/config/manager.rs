use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::config::paths::BytePaths;

/// Fixed local model used for conversation and tool calling.
pub const TEXT_MODEL: &str = "granite4.2:3b";
/// Fixed local model used for screen understanding.
pub const VISION_MODEL: &str = "minicpm-v4.6:latest";
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434/v1/chat/completions";
pub const DEFAULT_CLOUD_URL: &str = "https://api.openai.com/v1/chat/completions";
/// Default cloud model: handles conversation, tool calling, and images.
pub const DEFAULT_CLOUD_MODEL: &str = "gpt-4.1-mini";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LlmConfig {
    /// "ollama" (local, fixed models) or "cloud" (OpenAI-compatible API).
    pub provider: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub vision_model: String,
    pub cloud_url: String,
    /// One cloud model is used for text, tools, and vision.
    pub cloud_model: String,
    pub setup_complete: bool,
    pub reasoning_effort: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            ollama_url: DEFAULT_OLLAMA_URL.to_string(),
            ollama_model: TEXT_MODEL.to_string(),
            vision_model: VISION_MODEL.to_string(),
            cloud_url: DEFAULT_CLOUD_URL.to_string(),
            cloud_model: DEFAULT_CLOUD_MODEL.to_string(),
            setup_complete: false,
            reasoning_effort: "low".to_string(),
            temperature: 0.2,
            max_tokens: 1024,
        }
    }
}

impl LlmConfig {
    pub fn is_cloud(&self) -> bool {
        self.provider == "cloud"
    }

    /// Local models are not user-configurable for now; always pin them,
    /// and repair empty or unknown cloud values.
    fn enforce_fixed_models(&mut self) -> bool {
        let changed = self.ollama_model != TEXT_MODEL
            || self.vision_model != VISION_MODEL
            || self.ollama_url.trim().is_empty()
            || !matches!(self.provider.as_str(), "ollama" | "cloud")
            || self.cloud_url.trim().is_empty()
            || self.cloud_model.trim().is_empty();
        if !matches!(self.provider.as_str(), "ollama" | "cloud") {
            self.provider = "ollama".to_string();
        }
        if self.cloud_url.trim().is_empty() {
            self.cloud_url = DEFAULT_CLOUD_URL.to_string();
        }
        if self.cloud_model.trim().is_empty() {
            self.cloud_model = DEFAULT_CLOUD_MODEL.to_string();
        }
        self.ollama_model = TEXT_MODEL.to_string();
        self.vision_model = VISION_MODEL.to_string();
        if self.ollama_url.trim().is_empty() {
            self.ollama_url = DEFAULT_OLLAMA_URL.to_string();
        }
        changed
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
                    if cfg.llm.enforce_fixed_models() {
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
