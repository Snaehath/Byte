use std::path::PathBuf;

pub struct BytePaths;

impl BytePaths {
    /// Root application data directory for Byte (%APPDATA%\Byte)
    pub fn app_data_dir() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let p = PathBuf::from(app_data).join("Byte");
            let _ = std::fs::create_dir_all(&p);
            p
        } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(user_profile).join(".byte");
            let _ = std::fs::create_dir_all(&p);
            p
        } else {
            let p = PathBuf::from("data").join("byte");
            let _ = std::fs::create_dir_all(&p);
            p
        }
    }

    pub fn config_dir() -> PathBuf {
        let p = Self::app_data_dir().join("config");
        let _ = std::fs::create_dir_all(&p);
        p
    }

    pub fn models_dir() -> PathBuf {
        let p = Self::app_data_dir().join("models");
        let _ = std::fs::create_dir_all(&p);
        p
    }

    pub fn memory_dir() -> PathBuf {
        let p = Self::app_data_dir().join("memory");
        let _ = std::fs::create_dir_all(&p);
        p
    }

    pub fn temp_dir() -> PathBuf {
        let p = Self::app_data_dir().join("temp");
        let _ = std::fs::create_dir_all(&p);
        p
    }

    /// Resolve Whisper CLI binary path
    pub fn whisper_exe() -> PathBuf {
        if let Ok(p) = std::env::var("BYTE_WHISPER_EXE") {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return pb;
            }
        }
        let in_models = Self::models_dir().join("whisper-cli.exe");
        if in_models.exists() {
            return in_models;
        }
        // Dev fallback for user's existing machine
        let dev_path = PathBuf::from(r"D:\wisper\Release\whisper-cli.exe");
        if dev_path.exists() {
            return dev_path;
        }
        in_models
    }

    /// Resolve Whisper model path
    pub fn whisper_model() -> PathBuf {
        if let Ok(p) = std::env::var("BYTE_WHISPER_MODEL") {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return pb;
            }
        }
        let in_models = Self::models_dir().join("ggml-tiny.en.bin");
        if in_models.exists() {
            return in_models;
        }
        // Dev fallback for user's existing machine
        let dev_path = PathBuf::from(r"D:\wisper\Release\ggml-tiny.en.bin");
        if dev_path.exists() {
            return dev_path;
        }
        in_models
    }

    /// Resolve Piper binary path
    pub fn piper_exe() -> PathBuf {
        if let Ok(p) = std::env::var("BYTE_PIPER_EXE") {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return pb;
            }
        }
        let in_models = Self::models_dir().join("piper.exe");
        if in_models.exists() {
            return in_models;
        }
        // Dev fallback for user's existing machine
        let dev_path = PathBuf::from(r"D:\piper\piper.exe");
        if dev_path.exists() {
            return dev_path;
        }
        in_models
    }

    /// Resolve Piper model path
    pub fn piper_model() -> PathBuf {
        if let Ok(p) = std::env::var("BYTE_PIPER_MODEL") {
            let pb = PathBuf::from(p);
            if pb.exists() {
                return pb;
            }
        }
        let in_models = Self::models_dir().join("en_US-lessac-medium.onnx");
        if in_models.exists() {
            return in_models;
        }
        // Dev fallback for user's existing machine
        let dev_path = PathBuf::from(r"D:\piper\en_US-lessac-medium.onnx");
        if dev_path.exists() {
            return dev_path;
        }
        in_models
    }

    /// Resolve memory file path
    pub fn memory_file() -> PathBuf {
        if let Ok(p) = std::env::var("BYTE_MEMORY_PATH") {
            return PathBuf::from(p);
        }
        let in_memory_dir = Self::memory_dir().join("byte_memory.json");
        if in_memory_dir.exists() {
            return in_memory_dir;
        }
        // Dev fallback: if user previously had memory at D:\wisper\Release\byte_memory.json
        let dev_legacy = PathBuf::from(r"D:\wisper\Release\byte_memory.json");
        if dev_legacy.exists() {
            return dev_legacy;
        }
        in_memory_dir
    }

    /// Temporary WAV recording file path
    pub fn temp_wav() -> PathBuf {
        Self::temp_dir().join("temp_recording.wav")
    }

    /// Output directory for TTS generated audio
    pub fn tts_output_dir() -> PathBuf {
        Self::temp_dir()
    }
}
