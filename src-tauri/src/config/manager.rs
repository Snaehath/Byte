use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::config::paths::BytePaths;

/// Default local model for conversation and tool calling.
pub const DEFAULT_TEXT_MODEL: &str = "granite4.2:3b";
/// Default local model for screen understanding.
pub const DEFAULT_VISION_MODEL: &str = "minicpm-v4.6:latest";
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434/v1/chat/completions";
/// Cloud mode currently supports OpenAI only.
pub const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
/// Default cloud model: handles conversation, tool calling, and images.
pub const DEFAULT_CLOUD_MODEL: &str = "gpt-4.1-mini";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LlmConfig {
    /// "ollama" (local) or "cloud" (OpenAI).
    pub provider: String,
    pub ollama_url: String,
    /// Local model for conversation and tool calling.
    pub ollama_model: String,
    /// Local model for screen understanding (unused when `use_same_model`).
    pub vision_model: String,
    /// Use `ollama_model` for vision too (for models that handle images).
    pub use_same_model: bool,
    /// OpenAI model used for text, tools, and vision.
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
            ollama_model: DEFAULT_TEXT_MODEL.to_string(),
            vision_model: DEFAULT_VISION_MODEL.to_string(),
            use_same_model: false,
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

    /// Local model used when Byte looks at the screen.
    pub fn local_vision_model(&self) -> &str {
        if self.use_same_model { &self.ollama_model } else { &self.vision_model }
    }

    /// Fill in empty or unknown values with defaults. Returns true if anything changed.
    fn repair(&mut self) -> bool {
        fn fill(field: &mut String, valid: bool, default: &str) -> bool {
            if !valid {
                *field = default.to_string();
            }
            !valid
        }
        let provider_ok = matches!(self.provider.as_str(), "ollama" | "cloud");
        let url_ok = !self.ollama_url.trim().is_empty();
        let text_ok = !self.ollama_model.trim().is_empty();
        let vision_ok = !self.vision_model.trim().is_empty();
        let cloud_ok = !self.cloud_model.trim().is_empty();
        let changes = [
            fill(&mut self.provider, provider_ok, "ollama"),
            fill(&mut self.ollama_url, url_ok, DEFAULT_OLLAMA_URL),
            fill(&mut self.ollama_model, text_ok, DEFAULT_TEXT_MODEL),
            fill(&mut self.vision_model, vision_ok, DEFAULT_VISION_MODEL),
            fill(&mut self.cloud_model, cloud_ok, DEFAULT_CLOUD_MODEL),
        ];
        changes.contains(&true)
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
                    if cfg.llm.repair() {
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
