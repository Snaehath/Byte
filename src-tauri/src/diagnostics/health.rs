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

        // 4. Configured local or cloud model API
        let config = AppConfig::load();
        let is_cloud = config.llm.provider == "cloud";
        let endpoint = if is_cloud { &config.llm.cloud_url } else { &config.llm.ollama_url };
        let service_root = endpoint.trim().trim_end_matches("/v1/chat/completions").trim_end_matches('/');
        let health_url = if is_cloud { format!("{}/models", service_root) } else { service_root.to_string() };
        let api_key = if is_cloud {
            crate::ai::credentials::cloud_api_key().ok().flatten().unwrap_or_default()
        } else {
            String::new()
        };
        let mut health_request = client.get(&health_url).timeout(std::time::Duration::from_millis(2500));
        if !api_key.trim().is_empty() {
            health_request = health_request.bearer_auth(api_key.trim());
        }
        let provider_ok = if let Ok(res) = health_request.send().await {
            if res.status().is_success() {
                true
            } else {
                false
            }
        } else { false };
        let model_name = if is_cloud { &config.llm.cloud_model } else { &config.llm.ollama_model };
        let provider_name = if is_cloud { "Cloud API" } else { "Ollama" };
        let provider_details = if provider_ok {
            format!("{} available at {} with model '{}'", provider_name, service_root, model_name)
        } else if is_cloud && api_key.trim().is_empty() {
            "Cloud API key is not configured in Windows Credential Manager.".to_string()
        } else {
            format!("{} is not responding at {}. Check the endpoint and try again.", provider_name, service_root)
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
