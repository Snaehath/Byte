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
    /// User or assistant greeted -> auto_listen = true (ready for user command)
    Greeting,
    /// User or assistant bid farewell / dismissed -> auto_listen = false
    Farewell,
    /// Desktop tool or action executed -> auto_listen = false (let user work)
    Command,
    /// Assistant asked a question to the user -> auto_listen = true
    Question,
    /// Ongoing multi-turn conversational exchange -> auto_listen = true
    Conversation,
    /// User or system cancelled / stopped -> auto_listen = false
    Cancellation,
    /// Destructive or sensitive action requires user approval -> auto_listen = true
    NeedsConfirmation,
}

impl ConversationIntent {
    /// Returns true if Byte should keep listening for another user turn.
    pub fn should_auto_listen(&self) -> bool {
        matches!(
            self,
            ConversationIntent::Greeting
                | ConversationIntent::Question
                | ConversationIntent::Conversation
                | ConversationIntent::NeedsConfirmation
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
        return ConversationIntent::Cancellation;
    }

    if has_pending_confirmation {
        return ConversationIntent::NeedsConfirmation;
    }

    let trans_clean = transcription.trim().to_lowercase();
    let resp_clean = response_text.trim().to_lowercase();

    // 1. Explicit cancellation or stop
    if trans_clean == "stop"
        || trans_clean == "cancel"
        || trans_clean.contains("shut up")
        || trans_clean.contains("be quiet")
        || trans_clean.contains("stop speaking")
    {
        return ConversationIntent::Cancellation;
    }

    // 2. Explicit user dismissal / closure
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
        || trans_clean == "cool"
        || trans_clean.contains("bye")
        || trans_clean.contains("goodbye")
        || trans_clean.contains("see ya")
        || trans_clean.contains("see you");

    // 3. Explicit assistant farewell
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

    if is_user_dismissal || is_assistant_farewell {
        return ConversationIntent::Farewell;
    }

    // 4. Desktop action execution complete
    if is_tool {
        return ConversationIntent::Command;
    }

    // 5. Greeting (user greeted, or assistant responded to a greeting)
    let is_greeting = crate::prompts::is_simple_greeting(&transcription)
        || resp_clean.starts_with("good morning")
        || resp_clean.starts_with("good afternoon")
        || resp_clean.starts_with("good evening")
        || resp_clean.starts_with("hello")
        || resp_clean.starts_with("hi ");

    if is_greeting {
        return ConversationIntent::Greeting;
    }

    // 6. Question inquiry (user asked a question or assistant asked a clarifying question)
    let is_user_question = trans_clean.ends_with('?')
        || trans_clean.starts_with("what")
        || trans_clean.starts_with("who")
        || trans_clean.starts_with("where")
        || trans_clean.starts_with("when")
        || trans_clean.starts_with("why")
        || trans_clean.starts_with("how")
        || trans_clean.starts_with("is ")
        || trans_clean.starts_with("are ")
        || trans_clean.starts_with("can ")
        || trans_clean.starts_with("could ")
        || trans_clean.contains("what is")
        || trans_clean.contains("what's");

    let is_assistant_question = response_text.trim().ends_with('?');

    if is_user_question || is_assistant_question {
        return ConversationIntent::Question;
    }

    // 7. Casual conversation, jokes, or open chat statements
    ConversationIntent::Conversation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_intent_auto_listens() {
        let intent = resolve_conversation_intent(
            "Hello, good morning.",
            "Good morning! How can I help you today?",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::Greeting);
        assert!(intent.should_auto_listen());
    }

    #[test]
    fn test_question_prompts_auto_listen() {
        let intent = resolve_conversation_intent(
            "Check my system",
            "Which drive would you like me to inspect?",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::Question);
        assert!(intent.should_auto_listen());
    }

    #[test]
    fn test_user_dismissal_stops_listening() {
        let intent = resolve_conversation_intent(
            "Nothing. Thank you.",
            "You're welcome! Have a great day!",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::Farewell);
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
        assert_eq!(intent, ConversationIntent::Command);
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

    #[test]
    fn test_cpu_query_resolves_to_question() {
        let intent = resolve_conversation_intent(
            "What is my CPU usage?",
            "Your CPU is currently at 14% utilization.",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::Question);
        assert!(intent.should_auto_listen());
    }

    #[test]
    fn test_tell_joke_resolves_to_conversation() {
        let intent = resolve_conversation_intent(
            "Tell me a joke.",
            "Why do programmers prefer dark mode? Because light attracts bugs!",
            false,
            false,
            false,
        );
        assert_eq!(intent, ConversationIntent::Conversation);
        assert!(intent.should_auto_listen());
    }
}
