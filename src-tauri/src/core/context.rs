use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use uuid::Uuid;

/// Lightweight, async-aware cancellation token using Tokio primitives
#[derive(Clone, Debug)]
pub struct CancellationToken {
    is_cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Trigger cancellation. Safe to call multiple times (idempotent).
    pub fn cancel(&self) {
        if !self.is_cancelled.swap(true, Ordering::SeqCst) {
            self.notify.notify_waiters();
        }
    }

    /// Returns true if cancellation has been triggered.
    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    /// Future that resolves when cancellation is triggered.
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }
}

/// Interaction context encapsulating an interaction lifecycle and its cancellation boundary
#[derive(Clone, Debug)]
pub struct InteractionContext {
    pub id: Uuid,
    pub cancellation: CancellationToken,
}

impl Default for InteractionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionContext {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            cancellation: CancellationToken::new(),
        }
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cancellation_token_basic() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            token_clone.cancel();
        });

        tokio::select! {
            _ = token.cancelled() => {
                assert!(token.is_cancelled());
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                panic!("Token cancellation timed out");
            }
        }
    }

    #[test]
    fn test_cancellation_idempotent() {
        let token = CancellationToken::new();
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_interaction_preemption_and_isolation() {
        use std::sync::Mutex;

        // Simulate AppState.active_interaction
        let active_interaction: Arc<Mutex<Option<InteractionContext>>> = Arc::new(Mutex::new(None));

        // 1. Interaction A starts
        let ctx_a = InteractionContext::new();
        {
            let mut guard = active_interaction.lock().unwrap();
            *guard = Some(ctx_a.clone());
        }

        assert!(!ctx_a.is_cancelled());

        // 2. Interaction B preempts Interaction A
        let ctx_b = InteractionContext::new();
        {
            let mut guard = active_interaction.lock().unwrap();
            if let Some(prev) = guard.take() {
                prev.cancel();
            }
            *guard = Some(ctx_b.clone());
        }

        // Context A is now cancelled, Context B is active and uncancelled
        assert!(ctx_a.is_cancelled(), "Superseded interaction A must be cancelled");
        assert!(!ctx_b.is_cancelled(), "New interaction B must be active and running");

        // Helper check matching commands.rs
        let is_valid = |ctx: &InteractionContext| -> bool {
            if ctx.is_cancelled() {
                return false;
            }
            if let Ok(guard) = active_interaction.lock() {
                if let Some(active) = guard.as_ref() {
                    return active.id == ctx.id && !active.is_cancelled();
                }
            }
            false
        };

        assert!(!is_valid(&ctx_a), "Interaction A must be rejected by validity check");
        assert!(is_valid(&ctx_b), "Interaction B must be accepted by validity check");
    }

    #[tokio::test]
    async fn test_cancellation_select_abort() {
        let ctx = InteractionContext::new();
        let cancel_clone = ctx.clone();

        // Spawn a background task simulating a slow in-flight LLM HTTP request (e.g. 5 seconds)
        let slow_llm_task = async {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            "finished"
        };

        // Cancel after 20ms
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            cancel_clone.cancel();
        });

        let aborted = tokio::select! {
            _ = slow_llm_task => false,
            _ = ctx.cancellation.cancelled() => true,
        };

        assert!(aborted, "Long running task must be cleanly aborted by cancellation token");
    }
}
