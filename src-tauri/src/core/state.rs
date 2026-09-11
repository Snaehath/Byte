use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationState {
    Idle,
    Listening,
    Processing,
    Speaking,
    WaitingForUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationIntent {
    /// Task or session completed, or user signaled dismissal -> Transition to Idle.
    Finished,
    /// Destructive or sensitive action requires user approval -> Transition to WaitingForUser.
    NeedsConfirmation,
    /// Assistant actively asked a follow-up question -> Transition to WaitingForUser.
    AskedQuestion,
    /// Desktop action executed successfully -> Transition to Idle.
    ActionCompleted,
    /// Ongoing multiturn conversational exchange.
    ContinueConversation,
}

impl ConversationIntent {
    /// Returns true if Byte should keep listening for another user turn.
    pub fn should_auto_listen(&self) -> bool {
        matches!(
            self,
            ConversationIntent::NeedsConfirmation
                | ConversationIntent::AskedQuestion
                | ConversationIntent::ContinueConversation
        )
    }
}

/// Resolves the conversation intent based on user transcription, assistant response, and tool state.
pub fn resolve_conversation_intent(
    transcription: &str,
    response_text: &str,
    is_tool: bool,
    has_pending_confirmation: bool,
    has_error: bool,
) -> ConversationIntent {
    if has_error {
        return ConversationIntent::Finished;
    }

    if has_pending_confirmation {
        return ConversationIntent::NeedsConfirmation;
    }

    let trans_clean = transcription.trim().to_lowercase();
    let resp_clean = response_text.trim().to_lowercase();

    // 1. Explicit user dismissal / closure
    let is_user_dismissal = trans_clean.contains("thank")
        || trans_clean.contains("nothing")
        || trans_clean.contains("that's all")
        || trans_clean.contains("that is all")
        || trans_clean.contains("that's it")
        || trans_clean.contains("that is it")
        || trans_clean.contains("nevermind")
        || trans_clean.contains("never mind")
        || trans_clean.contains("i'm good")
        || trans_clean.contains("im good")
        || trans_clean.contains("no need")
        || trans_clean == "ok"
        || trans_clean == "okay"
        || trans_clean == "alright"
        || trans_clean == "got it"
        || trans_clean == "cool";

    if is_user_dismissal {
        return ConversationIntent::Finished;
    }

    // 2. Explicit assistant farewell or sign-off
    let is_assistant_farewell = resp_clean.contains("goodbye")
        || resp_clean.contains("bye")
        || resp_clean.contains("see you")
        || resp_clean.contains("farewell")
        || resp_clean.contains("have a great day")
        || resp_clean.contains("have a nice day")
        || resp_clean.contains("have a good day")
        || resp_clean.contains("take care")
        || resp_clean.contains("happy coding")
        || resp_clean.contains("you're welcome")
        || resp_clean.contains("you are welcome")
        || resp_clean.contains("anytime")
        || resp_clean.contains("my pleasure")
        || resp_clean.contains("no problem");

    if is_assistant_farewell {
        return ConversationIntent::Finished;
    }

    // 3. Desktop action execution complete
    if is_tool {
        return ConversationIntent::ActionCompleted;
    }

    // 4. Assistant asked a question to the user
    if response_text.trim().ends_with('?') {
        return ConversationIntent::AskedQuestion;
    }

    // Default policy: informative statements transition to Finished (Idle)
    ConversationIntent::Finished
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_prompts_auto_listen() {
        let intent = resolve_conversation_intent(
            "Hello, how are you?",
            "I'm doing great! How can I help you today?",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::AskedQuestion);
        assert!(intent.should_auto_listen());
    }

    #[test]
    fn test_user_dismissal_stops_listening() {
        let intent = resolve_conversation_intent(
            "Nothing. Thank you.",
            "You're welcome! Happy coding!",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::Finished);
        assert!(!intent.should_auto_listen());
    }

    #[test]
    fn test_action_completed_stops_listening() {
        let intent = resolve_conversation_intent(
            "Open Notepad.",
            "Opening Notepad for you.",
            true,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::ActionCompleted);
        assert!(!intent.should_auto_listen());
    }

    #[test]
    fn test_pending_confirmation_prompts_listening() {
        let intent = resolve_conversation_intent(
            "Delete the temp files.",
            "I need your confirmation to execute delete_files. Proceed?",
            false,
            true,
            false,
        );
        assert_eq!(intent, ConversationIntent::NeedsConfirmation);
        assert!(intent.should_auto_listen());
    }
}
