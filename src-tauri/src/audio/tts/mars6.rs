use std::path::Path;
use crate::audio::tts::TextToSpeech;

/// Mars6-Turbo TTS provider slot (to be implemented in Phase 1)
pub struct Mars6Tts;

impl TextToSpeech for Mars6Tts {
    fn synthesize(&self, _text: &str, _output_wav: &Path, _speed: f32) -> Result<(), String> {
        Err("Mars6-Turbo TTS is scheduled for integration in Phase 1. Currently active provider is Piper ONNX.".to_string())
    }
}
