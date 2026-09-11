use std::sync::{Arc, Mutex};
use tauri::{Emitter, WebviewWindow};
use crate::presence::state::PresenceState;
use crate::presence::placement::{Anchor, PlacementEngine};

#[derive(Clone)]
pub struct PresenceManager {
    current_state: Arc<Mutex<PresenceState>>,
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            current_state: Arc::new(Mutex::new(PresenceState::Idle)),
        }
    }

    /// Get current presence state
    pub fn get_state(&self) -> PresenceState {
        *self.current_state.lock().unwrap()
    }

    /// Set presence state and notify the frontend UI
    pub fn set_state(&self, window: &WebviewWindow, state: PresenceState) {
        {
            let mut guard = self.current_state.lock().unwrap();
            *guard = state;
        }
        let _ = window.emit("presence_state_changed", state);
    }

    /// Bring the Byte presence window onto the active monitor and show it
    pub fn show(&self, window: &WebviewWindow, anchor: Anchor) -> Result<(), String> {
        PlacementEngine::position_window(window, anchor)?;
        window.show().map_err(|e| format!("Failed to show window: {}", e))?;
        window.set_focus().map_err(|e| format!("Failed to focus window: {}", e))?;
        let _ = window.set_always_on_top(true);
        self.set_state(window, PresenceState::Idle);
        Ok(())
    }

    /// Hide the Byte presence window
    pub fn hide(&self, window: &WebviewWindow) -> Result<(), String> {
        self.set_state(window, PresenceState::Hidden);
        window.hide().map_err(|e| format!("Failed to hide window: {}", e))?;
        Ok(())
    }

    /// Handle Ctrl+B or global presence toggle
    pub fn toggle(&self, window: &WebviewWindow) -> Result<(), String> {
        let is_visible = window.is_visible().unwrap_or(false);
        let state = self.get_state();

        if is_visible && state != PresenceState::Hidden {
            log::info!("PresenceManager: Active orb visible (state: {:?}), collapsing...", state);
            // If in an active interaction, cancel first
            self.set_state(window, PresenceState::Cancelled);
            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = window_clone.hide();
            });
        } else {
            log::info!("PresenceManager: Summoning Byte presence to active monitor...");
            self.show(window, Anchor::BottomRight)?;
            let _ = window.emit("wakeup", ());
        }

        Ok(())
    }
}
