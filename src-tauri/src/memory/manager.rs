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
        Self {
            user_profile: UserProfile {
                name: String::new(),
                preferences: HashMap::new(),
                habits: Vec::new(),
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
        let mut mem = storage.load().unwrap_or_default();
        mem.sanitize_recent_turns();
        mem.remove_placeholder_profile_values();
        mem
    }

    fn remove_placeholder_profile_values(&mut self) {
        if self.user_profile.name == "Developer" {
            self.user_profile.name.clear();
        }
        self.user_profile.preferences.retain(|key, value| {
            !matches!(
                (key.as_str(), value.as_str()),
                ("theme", "dark")
                    | ("coding_style", "functional, clean, TypeScript/Rust")
                    | ("daily_greeting", "Good morning, happy coding!")
            )
        });
        self.user_profile.habits.retain(|habit| {
            habit != "developing cool software" && habit != "working in terminal"
        });
    }

    /// Guarantee that stored history only contains complete user-assistant pairs
    pub fn sanitize_recent_turns(&mut self) {
        let mut clean = Vec::new();
        for turn in &self.recent_conversation {
            match turn.role.as_str() {
                "user" => {
                    if clean.last().map(|t: &ConversationTurn| t.role.as_str()) == Some("user") {
                        continue;
                    }
                    clean.push(turn.clone());
                }
                "assistant" => {
                    if clean.last().map(|t: &ConversationTurn| t.role.as_str()) == Some("user") {
                        clean.push(turn.clone());
                    }
                }
                _ => {}
            }
        }
        if clean.last().map(|t| t.role.as_str()) == Some("user") {
            clean.pop();
        }
        self.recent_conversation = clean;
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
