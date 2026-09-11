use std::path::{Path, PathBuf};
use crate::audio::stt::SpeechToText;
use crate::config::paths::BytePaths;
use crate::Command;

pub struct WhisperStt {
    pub exe_path: PathBuf,
    pub model_path: PathBuf,
}

impl Default for WhisperStt {
    fn default() -> Self {
        Self {
            exe_path: BytePaths::whisper_exe(),
            model_path: BytePaths::whisper_model(),
        }
    }
}

impl WhisperStt {
    pub fn new(exe_path: PathBuf, model_path: PathBuf) -> Self {
        Self { exe_path, model_path }
    }
}

impl SpeechToText for WhisperStt {
    fn transcribe(&self, audio_path: &Path) -> Result<String, String> {
        if !self.exe_path.exists() {
            return Err(format!("Whisper CLI not found at: {}", self.exe_path.display()));
        }
        if !self.model_path.exists() {
            return Err(format!("Whisper model not found at: {}", self.model_path.display()));
        }

        let num_threads = std::thread::available_parallelism()
            .map(|val| val.get())
            .unwrap_or(4);

        let output = Command::new(&self.exe_path)
            .arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(audio_path)
            .arg("-nt")
            .arg("-t")
            .arg(num_threads.to_string())
            .output()
            .map_err(|e| format!("Failed to run whisper-cli: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(text.trim().to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("Whisper execution error: {}", error))
        }
    }
}
