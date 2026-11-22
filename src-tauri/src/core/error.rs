use std::fmt;

#[derive(Debug, Clone)]
pub enum ByteError {
    /// Local AI model (Ollama) is unreachable or returned connection refused
    ModelUnreachable(String),
    /// Local model returned an inference error
    ModelError(String),
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
