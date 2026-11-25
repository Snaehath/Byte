use std::fmt;

#[derive(Debug, Clone)]
pub enum ByteError {
    /// Local AI model (Ollama) is unreachable or returned connection refused
    ModelUnreachable(String),
    /// Local model returned an inference error
    ModelError(String),
    /// Cloud model request failed (network, key, quota, or model name)
    CloudError(String),
    /// Whisper Speech-to-Text transcription failure
    TranscriptionFailed(String),
    /// Piper Text-to-Speech synthesis failure
    SynthesisFailed(String),
    /// Pipeline operation timed out
    Timeout { stage: String, duration_ms: u64 },
}

impl fmt::Display for ByteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ByteError::ModelUnreachable(msg) => write!(f, "AI Model Unreachable: {}", msg),
            ByteError::ModelError(msg) => write!(f, "AI Model Error: {}", msg),
            ByteError::CloudError(msg) => write!(f, "Cloud Model Error: {}", msg),
            ByteError::TranscriptionFailed(msg) => write!(f, "Transcription Error: {}", msg),
            ByteError::SynthesisFailed(msg) => write!(f, "Speech Synthesis Error: {}", msg),
            ByteError::Timeout { stage, duration_ms } => write!(f, "Stage '{}' timed out after {}ms", stage, duration_ms),
        }
    }
}

impl std::error::Error for ByteError {}

impl ByteError {
    /// Translates internal errors into natural, spoken user-friendly explanations
    pub fn to_user_friendly_message(&self) -> String {
        match self {
            ByteError::ModelUnreachable(_) => {
                "I'm having trouble reaching my local AI model. Please ensure Ollama is running.".to_string()
            }
            ByteError::CloudError(msg) => cloud_error_message(msg),
            ByteError::ModelError(_) => {
                "I encountered an issue generating a response. Please try again in a moment.".to_string()
            }
            ByteError::TranscriptionFailed(_) => {
                "I couldn't hear that clearly. Please check your microphone and try again.".to_string()
            }
            ByteError::SynthesisFailed(_) => {
                "I couldn't generate voice audio for that response.".to_string()
            }
            ByteError::Timeout { stage, .. } => {
                format!("The {} step took too long and timed out.", stage)
            }
        }
    }
}

/// Spoken explanation for common cloud API failures.
pub fn cloud_error_message(raw: &str) -> String {
    log::error!("Cloud model error: {raw}");
    cloud_error_reason(raw).to_string()
        + &status_code(raw).map(|c| format!(" (error {c}, details in the log)")).unwrap_or_default()
}

/// The HTTP status code in an error string like "status 400 Bad Request".
fn status_code(raw: &str) -> Option<u16> {
    let rest = &raw[raw.find("status ")? + "status ".len()..];
    rest.get(..3)?.parse().ok()
}

fn cloud_error_reason(raw: &str) -> &'static str {
    if raw.contains("API key") && raw.contains("settings") {
        "Add your API key in Byte's settings to use the cloud model."
    } else if raw.contains("401") {
        "Your API key was rejected. Please check it in Byte's settings."
    } else if raw.contains("429") {
        "The cloud API is rate limiting requests or your account is out of credit."
    } else if raw.contains("404") || raw.contains("model_not_found") {
        "That cloud model isn't available on your account. Choose another one in settings."
    } else if raw.contains("request failed") {
        "I couldn't reach the cloud model. Please check your internet connection."
    } else {
        "The cloud model returned an error."
    }
}
