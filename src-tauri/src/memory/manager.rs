use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::memory::storage::JsonFileStorage;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
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
#[serde(default)]
pub struct ByteMemory {
    pub user_profile: UserProfile,
    pub scheduled_reminders: Vec<Reminder>,
    pub is_first_run: bool,
}

impl Default for ByteMemory {
    fn default() -> Self {
        Self {
            user_profile: UserProfile::default(),
            scheduled_reminders: Vec::new(),
            is_first_run: true,
        }
    }
}

impl ByteMemory {
    pub fn load() -> Self {
        let storage = JsonFileStorage::default();
        let mut mem = storage.load().unwrap_or_default();
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

    pub fn save(&self) -> Result<(), String> {
        let storage = JsonFileStorage::default();
        storage.save(self)
    }
}
