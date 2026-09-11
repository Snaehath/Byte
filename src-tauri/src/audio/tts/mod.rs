pub mod piper;
pub mod mars6;

use std::path::Path;

pub trait TextToSpeech: Send + Sync {
    fn synthesize(&self, text: &str, output_wav: &Path, speed: f32) -> Result<(), String>;
}

pub use piper::PiperTts;
pub use mars6::Mars6Tts;

/// Applies technical acronym pronunciation phonetic rules for TTS
pub fn apply_pronunciation_rules(text: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let mut clean_word = word.to_string();
        let mut suffix = String::new();
        while !clean_word.is_empty() && clean_word.ends_with(|c: char| c.is_ascii_punctuation()) {
            if let Some(c) = clean_word.pop() {
                suffix.insert(0, c);
            }
        }
        let mut prefix = String::new();
        while !clean_word.is_empty() && clean_word.starts_with(|c: char| c.is_ascii_punctuation()) {
            let c = clean_word.remove(0);
            prefix.push(c);
        }

        let replaced = match clean_word.as_str() {
            "Tauri" | "tauri" => "tore-ee",
            "LLM" | "llm" => "ell-ell-em",
            "TTS" | "tts" => "tee-tee-ess",
            "STT" | "stt" => "ess-tee-tee",
            "JSON" | "json" => "jay-son",
            "API" | "api" => "ay-pee-eye",
            "WAV" | "wav" => "wave",
            "CPU" | "cpu" => "see-pee-you",
            "RAM" | "ram" => "ram",
            "CLI" | "cli" => "see-ell-eye",
            "URL" | "url" => "you-are-ell",
            "HTML" | "html" => "aitch-tee-em-ell",
            "JS" | "js" => "jay-ess",
            "TS" | "ts" => "tee-ess",
            "VS" | "vs" => "versus",
            "UI" | "ui" => "you-eye",
            "VAD" | "vad" => "vad",
            "SSML" | "ssml" => "ess-ess-em-ell",
            "Ollama" | "ollama" => "oh-lah-mah",
            "Qwen" | "qwen" => "kwen",
            _ => &clean_word,
        };
        words.push(format!("{}{}{}", prefix, replaced, suffix));
    }
    words.join(" ")
}

/// Cleans markdown and control syntax before feeding text into TTS synthesizer
pub fn clean_for_speech(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Strip [MOOD: ...] tags if present
    if let Some(start_idx) = cleaned.find("[MOOD:") {
        if let Some(end_idx) = cleaned[start_idx..].find(']') {
            cleaned.replace_range(start_idx..=(start_idx + end_idx), "");
        }
    }

    // Strip raw JSON tool calls
    if let Some(start_idx) = cleaned.find('{') {
        if let Some(end_idx) = cleaned.rfind('}') {
            if end_idx > start_idx {
                cleaned.replace_range(start_idx..=end_idx, "");
            }
        }
    }

    cleaned
        .replace("**", "")
        .replace("*", "")
        .replace("`", "")
        .trim()
        .to_string()
}
