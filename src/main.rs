use std::io::{self, BufRead};
use std::process::Command;
use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    response: String,
}

fn main() {
    // 1. Setup paths
    let whisper_exe = r"D:\wisper\Release\whisper-cli.exe";
    let whisper_model = r"D:\wisper\Release\ggml-tiny.en.bin";
    let temp_wav = r"D:\wisper\Release\temp_recording.wav";

    // 2. Record from microphone
    println!("=== Audio Recording ===");
    let recorded_samples = match record_microphone() {
        Ok(samples) => samples,
        Err(e) => {
            eprintln!("Error recording audio: {}", e);
            return;
        }
    };

    if recorded_samples.is_empty() {
        println!("No audio was recorded.");
        return;
    }

    // 3. Save to WAV (16000Hz, Mono, 16-bit PCM)
    println!("Saving audio to {}...", temp_wav);
    if let Err(e) = save_wav(temp_wav, &recorded_samples) {
        eprintln!("Error saving WAV file: {}", e);
        return;
    }

    // 4. Transcribe using Whisper
    println!("\n=== Transcribing ===");
    match transcribe_audio(whisper_exe, whisper_model, temp_wav) {
        Ok(user_speech) => {
            if user_speech.is_empty() {
                println!("No speech detected.");
                return;
            }
            println!("User said: \"{}\"", user_speech);

            // 5. Query Qwen model
            println!("\n=== Asking Qwen ===");
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(ask_qwen(&user_speech)) {
                Ok(llm_response) => {
                    println!("\nByte: {}", llm_response);
                }
                Err(e) => {
                    println!("Error querying Qwen: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error transcribing: {}", e);
        }
    }

    // Cleanup temp file
    let _ = std::fs::remove_file(temp_wav);
}

// Capture audio from default input device until user presses Enter
fn record_microphone() -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No default input audio device found")?;
    
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    println!("Using input device: {}", device.name()?);
    println!("Recording configuration: Sample Rate = {}Hz, Channels = {}", sample_rate, channels);

    let recorded_data = Arc::new(Mutex::new(Vec::new()));
    let recorded_data_clone = Arc::clone(&recorded_data);

    // Audio stream callback
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = recorded_data_clone.lock().unwrap();
                // Convert multi-channel input to mono by taking the first channel's samples
                for frame in data.chunks(channels as usize) {
                    if !frame.is_empty() {
                        buffer.push(frame[0]);
                    }
                }
            },
            err_fn,
            None,
        )?,
        _ => return Err("Only F32 sample format is currently supported by this recorder".into()),
    };

    stream.play()?;
    println!("Recording started. Press [ENTER] to stop speaking...");

    // Wait for user to press enter
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();
    let _ = iterator.next();

    stream.pause()?;
    println!("Recording stopped.");

    let raw_samples = recorded_data.lock().unwrap().clone();
    
    // Resample to 16000Hz (Whisper's required rate) if different
    let resampled = resample(&raw_samples, sample_rate, 16000);
    Ok(resampled)
}

// Quick linear resampler to convert device sample rate to 16000Hz
fn resample(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz {
        return input.to_vec();
    }
    let factor = to_hz as f64 / from_hz as f64;
    let new_len = (input.len() as f64 * factor) as usize;
    let mut output = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let idx = i as f64 / factor;
        let low = idx.floor() as usize;
        let high = idx.ceil() as usize;
        if high >= input.len() {
            output.push(input[low]);
            continue;
        }
        let diff = idx - low as f64;
        let val = input[low] * (1.0 - diff as f32) + input[high] * diff as f32;
        output.push(val);
    }
    output
}

// Save raw f32 samples to a 16-bit mono PCM WAV file
fn save_wav(path: &str, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in samples {
        // Clamp and scale f32 float [-1.0, 1.0] to i16 range
        let scaled = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(scaled)?;
    }
    writer.finalize()?;
    Ok(())
}

fn transcribe_audio(whisper_exe: &str, model_path: &str, audio_path: &str) -> Result<String, String> {
    let output = Command::new(whisper_exe)
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(audio_path)
        .arg("-nt")
        .output()
        .map_err(|e| format!("Failed to run whisper-cli: {}", e))?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(text.trim().to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Whisper Error: {}", error))
    }
}

async fn ask_qwen(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = "http://localhost:11434/api/generate";
    
    let request_body = OllamaRequest {
        model: "qwen3-4b".to_string(),
        prompt: prompt.to_string(),
        stream: false,
    };

    let http_response = client
        .post(url)
        .json(&request_body)
        .send()
        .await?;

    if http_response.status().is_success() {
        let response_data: OllamaResponse = http_response.json().await?;
        Ok(response_data.response)
    } else {
        Err(format!("Ollama API returned status: {}", http_response.status()).into())
    }
}
