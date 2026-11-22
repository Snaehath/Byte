use std::time::{Duration, Instant};
use crate::ai::ChatMessage;

/// Short-lived, in-memory context for follow-up questions within one
/// conversation. Never written to disk; it resets after a pause or a goodbye,
/// so past conversations are never silently brought back.
#[derive(Default)]
pub struct ConversationSession {
    messages: Vec<ChatMessage>,
    last_turn: Option<Instant>,
}

impl ConversationSession {
    /// Keep the last three exchanges; enough for follow-ups on a 3B model.
    const MAX_MESSAGES: usize = 6;
    const IDLE_RESET: Duration = Duration::from_secs(180);

    pub fn history(&mut self) -> Vec<ChatMessage> {
        if self.last_turn.is_some_and(|t| t.elapsed() > Self::IDLE_RESET) {
            self.clear();
        }
        self.messages.clone()
    }

    pub fn record(&mut self, user: &str, assistant: &str) {
        self.messages.push(ChatMessage::new("user", user));
        self.messages.push(ChatMessage::new("assistant", assistant));
        let excess = self.messages.len().saturating_sub(Self::MAX_MESSAGES);
        self.messages.drain(..excess);
        self.last_turn = Some(Instant::now());
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.last_turn = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_keeps_recent_pairs_only() {
        let mut session = ConversationSession::default();
        for i in 0..5 {
            session.record(&format!("q{i}"), &format!("a{i}"));
        }
        let history = session.history();
        assert_eq!(history.len(), ConversationSession::MAX_MESSAGES);
        assert_eq!(history.first().unwrap().role, "user");
        assert_eq!(history.last().unwrap().content, "a4");
    }

    #[test]
    fn test_session_resets_after_idle() {
        let mut session = ConversationSession::default();
        session.record("q", "a");
        session.last_turn = Some(Instant::now() - Duration::from_secs(600));
        assert!(session.history().is_empty());
    }
}
