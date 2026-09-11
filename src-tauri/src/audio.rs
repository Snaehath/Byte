pub mod capture;
pub mod vad;
pub mod resampler;
pub mod playback;
pub mod stt;
pub mod tts;
pub mod service;

pub use service::SpeechService;
pub use playback::{stop_audio, reset_cancellation, play_listening_chime, play_processing_chime, play_audio_file};
pub use capture::{start_recording, start_wake_word_detector};
pub use resampler::{resample, save_wav};

// Backwards-compatible convenience helpers
pub fn speak_text(text: &str) {
    let service = SpeechService::default();
    let _ = service.speak(text, 1.0);
}

pub fn speak_text_with_speed(text: &str, speed: f32) {
    let service = SpeechService::default();
    let _ = service.speak(text, speed);
}

pub fn start_recording_internal(state: &crate::AppState, window: tauri::WebviewWindow) -> Result<(), String> {
    start_recording(state, window)
}
