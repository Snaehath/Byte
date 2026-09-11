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

    /// Synthesize speech to a temporary WAV file without playing it
    pub fn synthesize(&self, text: &str, speed: f32) -> Result<std::path::PathBuf, String> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let output_wav = BytePaths::tts_output_dir().join(format!("speech_{}.wav", ts));
        self.tts.synthesize(text, &output_wav, speed)?;
        Ok(output_wav)
    }

    /// Play a synthesized WAV file and remove it afterwards
    pub fn play(&self, wav_path: &Path) {
        play_audio_file(wav_path);
        let _ = std::fs::remove_file(wav_path);
    }

    /// Synthesize speech from text and play it out loud (blocking until done)
    pub fn speak(&self, text: &str, speed: f32) -> Result<(), String> {
        let output_wav = self.synthesize(text, speed)?;
        self.play(&output_wav);
        Ok(())
    }
}
