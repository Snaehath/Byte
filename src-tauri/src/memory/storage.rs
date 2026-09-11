use std::path::{Path, PathBuf};
use crate::memory::manager::ByteMemory;
use crate::config::paths::BytePaths;

pub trait MemoryStorage: Send + Sync {
    fn load(&self) -> Result<ByteMemory, String>;
    fn save(&self, memory: &ByteMemory) -> Result<(), String>;
}

pub struct JsonFileStorage {
    pub path: PathBuf,
}

impl Default for JsonFileStorage {
    fn default() -> Self {
        Self {
            path: BytePaths::memory_file(),
        }
    }
}

impl JsonFileStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl MemoryStorage for JsonFileStorage {
    fn load(&self) -> Result<ByteMemory, String> {
        if self.path.exists() {
            let content = std::fs::read_to_string(&self.path)
                .map_err(|e| format!("Failed to read memory file: {}", e))?;
            let memory: ByteMemory = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse memory JSON: {}", e))?;
            return Ok(memory);
        }
        let default_memory = ByteMemory::default();
        let _ = self.save(&default_memory);
        Ok(default_memory)
    }

    fn save(&self, memory: &ByteMemory) -> Result<(), String> {
        let content = serde_json::to_string_pretty(memory)
            .map_err(|e| format!("Failed to serialize memory: {}", e))?;
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&self.path, content)
            .map_err(|e| format!("Failed to write memory file: {}", e))?;
        Ok(())
    }
}
