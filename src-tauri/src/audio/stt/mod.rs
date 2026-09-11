pub mod whisper;

use std::path::Path;

pub trait SpeechToText: Send + Sync {
    fn transcribe(&self, audio_path: &Path) -> Result<String, String>;
}

pub use whisper::WhisperStt;
