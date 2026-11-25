use serde::{Deserialize, Serialize};
use cpal::traits::HostTrait;
use crate::config::paths::BytePaths;
use crate::config::AppConfig;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComponentHealth {
    pub name: String,
    pub available: bool,
    pub details: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemHealthReport {
    pub microphone: ComponentHealth,
    pub whisper_binary: ComponentHealth,
    pub whisper_model: ComponentHealth,
    pub piper_binary: ComponentHealth,
    pub piper_model: ComponentHealth,
    pub ollama_service: ComponentHealth,
    pub memory_store: ComponentHealth,
    pub overall_status: bool,
}

pub struct HealthChecker;

impl HealthChecker {
    pub async fn check_system(client: &reqwest::Client) -> SystemHealthReport {
        // 1. Microphone
        let host = cpal::default_host();
        let mic_available = host.default_input_device().is_some();
        let microphone = ComponentHealth {
            name: "Microphone".to_string(),
            available: mic_available,
            details: if mic_available { "Default audio input device ready".to_string() } else { "No input audio device found".to_string() },
        };

        // 2. Whisper
        let whisper_exe = BytePaths::whisper_exe();
        let whisper_binary = ComponentHealth {
            name: "Whisper Binary".to_string(),
            available: whisper_exe.exists(),
            details: format!("{}", whisper_exe.display()),
        };

        let whisper_model_path = BytePaths::whisper_model();
        let whisper_model = ComponentHealth {
            name: "Whisper Model".to_string(),
            available: whisper_model_path.exists(),
            details: format!("{}", whisper_model_path.display()),
        };

        // 3. Piper
        let piper_exe = BytePaths::piper_exe();
        let piper_binary = ComponentHealth {
            name: "Piper Binary".to_string(),
            available: piper_exe.exists(),
            details: format!("{}", piper_exe.display()),
        };

        let piper_model_path = BytePaths::piper_model();
        let piper_model = ComponentHealth {
            name: "Piper Model".to_string(),
            available: piper_model_path.exists(),
            details: format!("{}", piper_model_path.display()),
        };

        // 4. Model provider
        let config = AppConfig::load();
        let (provider_ok, provider_details) = if config.llm.is_cloud() {
            let has_key = crate::ai::credentials::saved_cloud_api_key().is_some();
            let details = if has_key {
                format!("OpenAI model {}", config.llm.cloud_model)
            } else {
                "Cloud mode is selected but no API key is saved.".to_string()
            };
            (has_key, details)
        } else {
            let service_root = config.llm.ollama_url.trim().trim_end_matches("/v1/chat/completions").trim_end_matches('/').to_string();
            let ok = client
                .get(&service_root)
                .timeout(std::time::Duration::from_millis(2500))
                .send()
                .await
                .map(|res| res.status().is_success())
                .unwrap_or(false);
            let details = if ok {
                format!("Ollama at {} (text: {}, vision: {})", service_root, config.llm.ollama_model, config.llm.local_vision_model())
            } else {
                format!("Ollama is not responding at {}. Make sure Ollama is running.", service_root)
            };
            (ok, details)
        };

        let ollama_service = ComponentHealth {
            name: "LLM Provider".to_string(),
            available: provider_ok,
            details: provider_details,
        };

        // 5. Memory
        let mem_path = BytePaths::memory_file();
        let memory_ok = mem_path.exists() || mem_path.parent().map(|p| p.exists()).unwrap_or(false);
        let memory_store = ComponentHealth {
            name: "Memory Store".to_string(),
            available: memory_ok,
            details: format!("{}", mem_path.display()),
        };

        let overall_status = microphone.available
            && whisper_binary.available
            && whisper_model.available
            && piper_binary.available
            && piper_model.available
            && ollama_service.available;

        SystemHealthReport {
            microphone,
            whisper_binary,
            whisper_model,
            piper_binary,
            piper_model,
            ollama_service,
            memory_store,
            overall_status,
        }
    }
}
