use std::sync::Arc;
use std::sync::atomic::Ordering;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::Emitter;
use crate::AppState;
use crate::audio::playback::{play_listening_chime, play_processing_chime, stop_audio};
use crate::audio::vad::AdaptiveVad;

pub fn start_recording(state: &AppState, window: tauri::WebviewWindow) -> Result<(), String> {
    stop_audio();
    play_listening_chime();

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No default input audio device found".to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Failed to get audio config: {}", e))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    // Clear previous recording buffer
    {
        let mut buffer = state.recorded_data.lock().unwrap();
        buffer.clear();
    }

    *state.device_sample_rate.lock().unwrap() = sample_rate;
    state.is_recording.store(true, Ordering::SeqCst);

    let recorded_data_clone = Arc::clone(&state.recorded_data);
    let is_recording_clone = Arc::clone(&state.is_recording);
    let window_clone = window.clone();
    let vad = Arc::new(std::sync::Mutex::new(AdaptiveVad::new()));
    let vad_clone = Arc::clone(&vad);

    let err_fn = |err| log::error!("Audio recording stream error: {}", err);

    std::thread::spawn(move || {
        let is_recording_cb = Arc::clone(&is_recording_clone);
        let stream_config: cpal::StreamConfig = config.clone().into();

        let stream_result = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_recording_cb.load(Ordering::SeqCst) {
                        return;
                    }

                    let mut buffer = recorded_data_clone.lock().unwrap();
                    let current_total = buffer.len();

                    for frame in data.chunks(channels as usize) {
                        if !frame.is_empty() {
                            buffer.push(frame[0]);
                        }
                    }

                    let silence_detected = {
                        if let Ok(mut v) = vad_clone.lock() {
                            v.process_frame(data, sample_rate, channels, current_total, 1500, 1000)
                        } else {
                            false
                        }
                    };

                    if silence_detected {
                        log::info!("VAD detected silence. Auto-stopping voice capture...");
                        is_recording_cb.store(false, Ordering::SeqCst);
                        play_processing_chime();
                        let _ = window_clone.emit("processing", ());
                    }
                },
                err_fn,
                None,
            ),
            _ => {
                log::error!("Unsupported microphone sample format");
                return;
            }
        };

        let stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to build input stream: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            log::error!("Failed to play input stream: {}", e);
            return;
        }

        while is_recording_clone.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        log::info!("Microphone capture finished.");
    });

    log::info!("Recording started at {}Hz ({} channels)", sample_rate, channels);
    Ok(())
}

pub fn start_wake_word_detector(
    is_recording: Arc<std::sync::atomic::AtomicBool>,
    is_interacting: Arc<std::sync::atomic::AtomicBool>,
    app_handle: tauri::AppHandle,
) {
    std::thread::spawn(move || {
        log::info!("Background wake word detector thread started.");
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                log::warn!("No default microphone found for wake word detection.");
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Wake detector failed to get input config: {}", e);
                return;
            }
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let audio_buffer = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
        let audio_buffer_clone = Arc::clone(&audio_buffer);
        let is_rec = Arc::clone(&is_recording);
        let is_int = Arc::clone(&is_interacting);

        let stream_config: cpal::StreamConfig = config.clone().into();
        let stream = match device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_rec.load(Ordering::SeqCst) || is_int.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(mut buf) = audio_buffer_clone.lock() {
                    for frame in data.chunks(channels as usize) {
                        if !frame.is_empty() {
                            buf.push(frame[0]);
                        }
                    }
                    // Keep max 3 seconds of audio
                    let max_samples = (sample_rate as usize) * 3;
                    if buf.len() > max_samples {
                        let excess = buf.len() - max_samples;
                        buf.drain(0..excess);
                    }
                }
            },
            |e| log::error!("Wake stream error: {}", e),
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Wake detector failed to build stream: {}", e);
                return;
            }
        };

        let _ = stream.play();

        loop {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            if is_recording.load(Ordering::SeqCst) || is_interacting.load(Ordering::SeqCst) {
                continue;
            }

            let samples = {
                if let Ok(mut buf) = audio_buffer.lock() {
                    let s = buf.clone();
                    buf.clear();
                    s
                } else {
                    Vec::new()
                }
            };

            if samples.len() < (sample_rate as usize) {
                continue;
            }

            let sum_sq: f32 = samples.iter().map(|&x| x * x).sum();
            let rms = (sum_sq / samples.len() as f32).sqrt();
            if rms > 0.04 {
                // High energy burst detected in background
                log::info!("Wake detector observed voice energy (RMS: {:.4}). Checking wake intent...", rms);
                let resampled = crate::audio::resampler::resample(&samples, sample_rate, 16000);
                let temp_path = crate::config::paths::BytePaths::temp_dir().join("wake_check.wav");
                if crate::audio::resampler::save_wav(&temp_path, &resampled).is_ok() {
                    let whisper = crate::audio::stt::WhisperStt::default();
                    use crate::audio::stt::SpeechToText;
                    if let Ok(text) = whisper.transcribe(&temp_path) {
                        let clean = text.to_lowercase();
                        if clean.contains("byte") || clean.contains("hey byte") || clean.contains("bight") || clean.contains("hi byte") {
                            log::info!("Wake word verified: '{}'!", text);
                            let _ = app_handle.emit("wakeup", ());
                        }
                    }
                    let _ = std::fs::remove_file(&temp_path);
                }
            }
        }
    });
}
