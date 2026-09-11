use serde::{Deserialize, Serialize};

/// Visual presence states for the Byte desktop shell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceState {
    Hidden,
    Idle,
    Listening,
    Thinking,
    Speaking,
    Cancelled,
}

impl Default for PresenceState {
    fn default() -> Self {
        Self::Idle
    }
}

impl PresenceState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Idle => "idle",
            Self::Listening => "listening",
            Self::Thinking => "thinking",
            Self::Speaking => "speaking",
            Self::Cancelled => "cancelled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_state_serialization() {
        let state = PresenceState::Listening;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"listening\"");

        let deserialized: PresenceState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, PresenceState::Listening);
    }
}
