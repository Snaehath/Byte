use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use crate::memory::storage::{JsonFileStorage, MemoryStorage};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub name: String,
    pub preferences: HashMap<String, String>,
    pub habits: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Reminder {
    pub id: String,
    pub target_time_ms: u64,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConversationTurn {
    pub role: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ByteMemory {
    pub user_profile: UserProfile,
    pub scheduled_reminders: Vec<Reminder>,
    pub recent_conversation: Vec<ConversationTurn>,
    pub last_proactive_check: u64,
    pub is_first_run: bool,
}

impl Default for ByteMemory {
    fn default() -> Self {
        let mut preferences = HashMap::new();
        preferences.insert("theme".to_string(), "dark".to_string());
        preferences.insert("coding_style".to_string(), "functional, clean, TypeScript/Rust".to_string());
        preferences.insert("daily_greeting".to_string(), "Good morning, happy coding!".to_string());

        Self {
            user_profile: UserProfile {
                name: "Developer".to_string(),
                preferences,
                habits: vec!["developing cool software".to_string(), "working in terminal".to_string()],
            },
            scheduled_reminders: Vec::new(),
            recent_conversation: Vec::new(),
            last_proactive_check: 0,
            is_first_run: true,
        }
    }
}

impl ByteMemory {
    pub fn load() -> Self {
        let storage = JsonFileStorage::default();
        storage.load().unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let storage = JsonFileStorage::default();
        storage.save(self)
    }
}

pub struct MemoryManager {
    storage: Box<dyn MemoryStorage>,
    state: Arc<Mutex<ByteMemory>>,
}

impl Default for MemoryManager {
    fn default() -> Self {
        let storage = Box::new(JsonFileStorage::default());
        let memory = storage.load().unwrap_or_default();
        Self {
            storage,
            state: Arc::new(Mutex::new(memory)),
        }
    }
}

impl MemoryManager {
    pub fn new(storage: Box<dyn MemoryStorage>) -> Self {
        let memory = storage.load().unwrap_or_default();
        Self {
            storage,
            state: Arc::new(Mutex::new(memory)),
        }
    }

    pub fn get_state(&self) -> Arc<Mutex<ByteMemory>> {
        Arc::clone(&self.state)
    }

    pub fn snapshot(&self) -> ByteMemory {
        self.state.lock().unwrap().clone()
    }

    pub fn save(&self) -> Result<(), String> {
        let mem = self.state.lock().unwrap();
        self.storage.save(&mem)
    }

    pub fn add_conversation_turn(&self, role: &str, message: &str) {
        if let Ok(mut mem) = self.state.lock() {
            mem.recent_conversation.push(ConversationTurn {
                role: role.to_string(),
                message: message.to_string(),
            });
            // Keep last 10 turns
            if mem.recent_conversation.len() > 10 {
                let excess = mem.recent_conversation.len() - 10;
                mem.recent_conversation.drain(0..excess);
            }
            let _ = self.storage.save(&mem);
        }
    }
}
