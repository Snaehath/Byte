use std::path::Path;
use crate::audio::stt::{SpeechToText, WhisperStt};
use crate::audio::tts::{TextToSpeech, PiperTts};
use crate::audio::playback::play_audio_file;
use crate::config::paths::BytePaths;

pub struct SpeechService {
    stt: Box<dyn SpeechToText>,
    tts: Box<dyn TextToSpeech>,
}

impl Default for SpeechService {
    fn default() -> Self {
        Self {
            stt: Box::new(WhisperStt::default()),
            tts: Box::new(PiperTts::default()),
        }
    }
}

impl SpeechService {
    pub fn new(stt: Box<dyn SpeechToText>, tts: Box<dyn TextToSpeech>) -> Self {
        Self { stt, tts }
    }

    /// Transcribe a recorded WAV file into text
    pub fn transcribe(&self, wav_path: &Path) -> Result<String, String> {
        self.stt.transcribe(wav_path)
    }

    /// Synthesize speech from text and play it out loud (blocking until done)
    pub fn speak(&self, text: &str, speed: f32) -> Result<(), String> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let output_wav = BytePaths::tts_output_dir().join(format!("speech_{}.wav", ts));

        self.tts.synthesize(text, &output_wav, speed)?;
        play_audio_file(&output_wav);
        let _ = std::fs::remove_file(&output_wav);
        Ok(())
    }
}
