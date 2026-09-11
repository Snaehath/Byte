use serde::{Deserialize, Serialize};
use crate::tools::ToolCall;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiState {
    pub status: String, // "idle" | "listening" | "processing" | "speaking"
    pub mood: String,   // "calm" | "thoughtful" | "energetic" | "excited" | "apologetic"
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            status: "idle".to_string(),
            mood: "calm".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpeechMeta {
    pub speed: f32,
}

impl Default for SpeechMeta {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssistantResponse {
    pub text: String,
    pub thinking: String,
    pub tool_call: Option<ToolCall>,
    pub ui_state: UiState,
    pub speech: SpeechMeta,
}

impl AssistantResponse {
    pub fn simple_text(text: String, thinking: String) -> Self {
        // Derive mood naturally based on content sentiment without polluting prompt
        let mood = derive_mood_from_text(&text);
        Self {
            text,
            thinking,
            tool_call: None,
            ui_state: UiState {
                status: "speaking".to_string(),
                mood,
            },
            speech: SpeechMeta::default(),
        }
    }

    pub fn tool_execution(tool_call: ToolCall, thinking: String) -> Self {
        Self {
            text: String::new(),
            thinking,
            tool_call: Some(tool_call),
            ui_state: UiState {
                status: "processing".to_string(),
                mood: "energetic".to_string(),
            },
            speech: SpeechMeta::default(),
        }
    }

    pub fn error(error_message: String) -> Self {
        Self {
            text: error_message,
            thinking: String::new(),
            tool_call: None,
            ui_state: UiState {
                status: "speaking".to_string(),
                mood: "apologetic".to_string(),
            },
            speech: SpeechMeta::default(),
        }
    }
}

fn derive_mood_from_text(text: &str) -> String {
    let lower = text.to_lowercase();
    if lower.contains("sorry") || lower.contains("couldn't") || lower.contains("failed") || lower.contains("unable") {
        "apologetic".to_string()
    } else if lower.contains("great") || lower.contains("awesome") || lower.contains("playing") || lower.contains("launching") {
        "excited".to_string()
    } else if lower.contains("cpu") || lower.contains("ram") || lower.contains("stats") || lower.contains("according to") || lower.contains("search") {
        "thoughtful".to_string()
    } else {
        "calm".to_string()
    }
}
