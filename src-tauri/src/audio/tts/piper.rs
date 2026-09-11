use std::io::Write;
use std::path::{Path, PathBuf};
use crate::audio::tts::{TextToSpeech, apply_pronunciation_rules, clean_for_speech};
use crate::config::paths::BytePaths;
use crate::Command;

pub struct PiperTts {
    pub exe_path: PathBuf,
    pub model_path: PathBuf,
}

impl Default for PiperTts {
    fn default() -> Self {
        Self {
            exe_path: BytePaths::piper_exe(),
            model_path: BytePaths::piper_model(),
        }
    }
}

impl PiperTts {
    pub fn new(exe_path: PathBuf, model_path: PathBuf) -> Self {
        Self { exe_path, model_path }
    }
}

impl TextToSpeech for PiperTts {
    fn synthesize(&self, text: &str, output_wav: &Path, speed: f32) -> Result<(), String> {
        if !self.exe_path.exists() {
            return Err(format!("Piper binary not found at: {}", self.exe_path.display()));
        }
        if !self.model_path.exists() {
            return Err(format!("Piper model not found at: {}", self.model_path.display()));
        }

        let cleaned = clean_for_speech(text);
        if cleaned.is_empty() {
            return Ok(());
        }
        let pronounced = apply_pronunciation_rules(&cleaned);

        let length_scale = if speed > 0.1 { 1.0 / speed } else { 1.0 };

        let child = Command::new(&self.exe_path)
            .arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(output_wav)
            .arg("--length-scale")
            .arg(format!("{:.2}", length_scale))
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn Piper: {}", e))?;

        let mut child_proc = child;
        if let Some(mut stdin) = child_proc.stdin.take() {
            let _ = stdin.write_all(pronounced.as_bytes());
        }

        let status = child_proc.wait().map_err(|e| format!("Piper process failed: {}", e))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("Piper exited with status: {}", status))
        }
    }
}
