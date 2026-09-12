use std::fmt;

#[derive(Debug, Clone)]
pub enum ByteError {
    /// Local AI model (Ollama) is unreachable or returned connection refused
    ModelUnreachable(String),
    /// Local model returned an inference error
    ModelError(String),
    /// Audio input hardware or capture failure
    AudioHardware(String),
    /// Whisper Speech-to-Text transcription failure
    TranscriptionFailed(String),
    /// Piper Text-to-Speech synthesis failure
    SynthesisFailed(String),
    /// Desktop tool execution error
    ToolExecutionFailed { tool: String, error: String },
    /// Pipeline operation timed out
    Timeout { stage: String, duration_ms: u64 },
    /// Operation cancelled by user or pre-empted
    Cancelled,
}

impl fmt::Display for ByteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ByteError::ModelUnreachable(msg) => write!(f, "AI Model Unreachable: {}", msg),
            ByteError::ModelError(msg) => write!(f, "AI Model Error: {}", msg),
            ByteError::AudioHardware(msg) => write!(f, "Audio Hardware Error: {}", msg),
            ByteError::TranscriptionFailed(msg) => write!(f, "Transcription Error: {}", msg),
            ByteError::SynthesisFailed(msg) => write!(f, "Speech Synthesis Error: {}", msg),
            ByteError::ToolExecutionFailed { tool, error } => write!(f, "Tool '{}' Failed: {}", tool, error),
            ByteError::Timeout { stage, duration_ms } => write!(f, "Stage '{}' timed out after {}ms", stage, duration_ms),
            ByteError::Cancelled => write!(f, "Interaction Cancelled"),
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
            ByteError::AudioHardware(_) => {
                "I'm having trouble with the audio input device. Please check your microphone.".to_string()
            }
            ByteError::TranscriptionFailed(_) => {
                "I couldn't process your voice input clearly. Please try speaking again.".to_string()
            }
            ByteError::SynthesisFailed(_) => {
                "I couldn't generate voice audio for that response.".to_string()
            }
            ByteError::ToolExecutionFailed { tool, .. } => {
                format!("I ran into an issue trying to execute '{}'.", tool)
            }
            ByteError::Timeout { stage, .. } => {
                format!("The {} operation took longer than expected and timed out.", stage)
            }
            ByteError::Cancelled => {
                "Cancelled.".to_string()
            }
        }
    }
}
