use std::collections::HashMap;
use crate::tools::{DesktopTool, ToolFuture};

pub struct UpdateMemoryTool;

impl DesktopTool for UpdateMemoryTool {
    fn name(&self) -> &str { "update_memory" }
    fn description(&self) -> &str { "Update user name, preferences, or habits in persistent memory." }
    fn parameter_schema(&self) -> &str { "{\"name\": \"new name\", \"key\": \"preference key\", \"value\": \"preference value\", \"add_habit\": \"habit to add\", \"remove_habit\": \"habit to remove\"}" }

    fn execute<'a>(
        &self,
        params: &'a HashMap<String, serde_json::Value>,
        _window: &'a tauri::WebviewWindow,
        _client: &'a reqwest::Client,
        _transcription: &'a str,
        state: &'a tauri::State<'_, crate::AppState>,
    ) -> ToolFuture<'a> {
        Box::pin(async move {
            let mut updated_fields = Vec::new();
            if let Ok(mut mem) = state.memory.lock() {
                if let Some(new_name) = params.get("name").and_then(|v| v.as_str()) {
                    if !new_name.trim().is_empty() {
                        mem.user_profile.name = new_name.trim().to_string();
                        updated_fields.push(format!("username to '{}'", new_name));
                    }
                }
                
                let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("").trim();
                let value = params.get("value").and_then(|v| v.as_str()).unwrap_or("").trim();
                if !key.is_empty() && !value.is_empty() {
                    mem.user_profile.preferences.insert(key.to_string(), value.to_string());
                    updated_fields.push(format!("preference '{}' to '{}'", key, value));
                }

                if let Some(add_habit) = params.get("add_habit").and_then(|v| v.as_str()) {
                    if !add_habit.trim().is_empty() {
                        let habit = add_habit.trim().to_string();
                        if !mem.user_profile.habits.contains(&habit) {
                            mem.user_profile.habits.push(habit.clone());
                        }
                        updated_fields.push(format!("added habit '{}'", habit));
                    }
                }

                if let Some(remove_habit) = params.get("remove_habit").and_then(|v| v.as_str()) {
                    if !remove_habit.trim().is_empty() {
                        let habit = remove_habit.trim().to_string();
                        mem.user_profile.habits.retain(|h| h != &habit);
                        updated_fields.push(format!("removed habit '{}'", habit));
                    }
                }

                if !updated_fields.is_empty() {
                    if let Err(e) = mem.save() {
                        log::error!("update_memory: Failed to save memory: {}", e);
                        return Err(format!("I updated the values but failed to save them: {}", e));
                    }
                    let changes = updated_fields.join(", ");
                    Ok(format!("Updated your profile memory: {}.", changes))
                } else {
                    Ok("No updates were provided for memory.".to_string())
                }
            } else {
                Err("Memory store is locked or unavailable.".to_string())
            }
        })
    }
}
