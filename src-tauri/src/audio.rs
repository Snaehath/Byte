pub mod capture;
pub mod vad;
pub mod resampler;
pub mod playback;
pub mod stt;
pub mod tts;
pub mod service;

pub use service::SpeechService;
pub use playback::{stop_audio, reset_cancellation};
pub use capture::{start_recording, start_wake_word_detector};
pub use resampler::{resample, save_wav};

/// Speak text with the configured voice speed (blocking until playback ends).
pub fn speak_text(text: &str) {
    let speed = crate::config::AppConfig::load().audio.voice_speed;
    let _ = SpeechService::default().speak(text, speed);
}
