use std::sync::atomic::Ordering;
use tauri::{State, Emitter, Manager};
use serde::Serialize;
use crate::AppState;
use crate::audio::{self, SpeechService};
use crate::ai::{Capability, ModelManager};
use crate::ai::llm::{ChatMessage, LlmResult};
use crate::tools;
use crate::prompts;
use crate::core::{
    AssistantResponse, resolve_conversation_intent, InteractionContext, ByteError,
    STT_TIMEOUT_SECS, LLM_TIMEOUT_SECS, TOOL_TIMEOUT_SECS, TTS_SYNTH_TIMEOUT_SECS, TTS_PLAY_TIMEOUT_SECS,
};
use crate::diagnostics::{PipelineTelemetry, StageTimer};
use crate::platform::get_active_window_title;
use crate::config::paths::BytePaths;

use crate::presence::PresenceState;

#[derive(Serialize)]
pub struct ProcessResult {
    pub transcription: String,
    pub thinking: String,
    pub response: String,
    pub auto_listen: bool,
}

#[derive(Serialize)]
pub struct AppSettings {
    pub ollama_url: String,
    pub ollama_model: String,
    pub vision_model: String,
    pub whisper_exe: String,
    pub whisper_model: String,
    pub piper_exe: String,
    pub piper_model: String,
}

#[derive(PartialEq, Eq)]
enum ConfirmationIntent {
    Confirm,
    Cancel,
    Unclear,
}

fn confirmation_intent(transcription: &str) -> ConfirmationIntent {
    let lower = transcription.to_lowercase().replace('\'', "").replace('’', "");
    let normalized = lower
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // A refusal anywhere in the response takes precedence over agreement.
    let words: Vec<&str> = normalized.split_whitespace().collect();
    let has_refusal = words.iter().any(|word| {
        matches!(*word, "no" | "cancel" | "stop" | "never" | "not" | "dont" | "cant" | "cannot")
    }) || normalized.contains("do not");
    if has_refusal {
        return ConfirmationIntent::Cancel;
    }

    // Accept only short, unambiguous responses. Longer phrases are treated as
    // a new request, so incidental words like "yes" cannot approve an action.
    if matches!(
        normalized.as_str(),
        "yes" | "yes please" | "sure" | "sure thing" | "confirm" | "confirmed"
            | "go ahead" | "do it" | "okay" | "ok"
    ) {
        ConfirmationIntent::Confirm
    } else {
        ConfirmationIntent::Unclear
    }
}

#[tauri::command]
pub fn stop_action(state: State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    // 1. Cancel and take the active interaction context if present (idempotent)
    if let Ok(mut active) = state.active_interaction.lock() {
        if let Some(ctx) = active.take() {
            log::info!("Cancelling active interaction: {}", ctx.id);
            ctx.cancel();
        }
    }

    // 2. Immediately stop audio hardware capture & flush Rodio audio sink
    audio::stop_audio();
    state.is_recording.store(false, Ordering::SeqCst);
    state.is_interacting.store(false, Ordering::SeqCst);

    if let Some(window) = app_handle.get_webview_window("main") {
        state.presence.set_state(&window, PresenceState::Cancelled);
    }

    Ok(())
}

#[tauri::command]
pub async fn reposition_presence(window: tauri::WebviewWindow, current_mode: String) -> Result<String, String> {
    let desktop_foreground = crate::platform::windows::foreground_is_desktop()
        .unwrap_or(current_mode != "corner");
    let (anchor, mode) = if desktop_foreground {
        (crate::presence::Anchor::Center, "center")
    } else {
        (crate::presence::Anchor::BottomRight, "corner")
    };
    crate::presence::PlacementEngine::position_window_smooth(&window, anchor).await?;
    Ok(mode.to_string())
}

#[tauri::command]
pub fn set_conversation_window_size(
    window: tauri::WebviewWindow,
    expanded: bool,
    placement_mode: String,
) -> Result<(), String> {
    let size = if expanded {
        tauri::LogicalSize::new(380.0, 370.0)
    } else {
        tauri::LogicalSize::new(220.0, 220.0)
    };
    window.set_size(size).map_err(|e| format!("Failed to resize Byte: {}", e))?;
    let anchor = match placement_mode.as_str() {
        "center" => Some(crate::presence::Anchor::Center),
        "corner" => Some(crate::presence::Anchor::BottomRight),
        _ => None,
    };
    if let Some(anchor) = anchor {
        crate::presence::PlacementEngine::position_window(&window, anchor)?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_app_settings() -> AppSettings {
    let config = crate::config::AppConfig::load();
    AppSettings {
        ollama_url: config.llm.ollama_url,
        ollama_model: config.llm.ollama_model,
        vision_model: config.llm.vision_model,
        whisper_exe: config.speech_tools.whisper_exe,
        whisper_model: config.speech_tools.whisper_model,
        piper_exe: config.speech_tools.piper_exe,
        piper_model: config.speech_tools.piper_model,
    }
}

#[tauri::command]
pub fn save_app_settings(
    ollama_url: String,
    ollama_model: String,
    vision_model: String,
    whisper_exe: String,
    whisper_model: String,
    piper_exe: String,
    piper_model: String,
) -> Result<AppSettings, String> {
    let ollama_url = ollama_url.trim();
    let ollama_model = ollama_model.trim();
    let vision_model = vision_model.trim();
    let whisper_exe = whisper_exe.trim();
    let whisper_model = whisper_model.trim();
    let piper_exe = piper_exe.trim();
    let piper_model = piper_model.trim();
    if !(ollama_url.starts_with("http://") || ollama_url.starts_with("https://"))
        || ollama_url.chars().any(|ch| ch.is_whitespace())
        || ollama_url.len() > 2048
    {
        return Err("Enter a valid HTTP or HTTPS Ollama chat endpoint.".to_string());
    }
    if ollama_model.is_empty() || ollama_model.len() > 120 || vision_model.len() > 120 {
        return Err("Enter a chat model name, and keep model names under 120 characters.".to_string());
    }
    if [whisper_exe, whisper_model, piper_exe, piper_model]
        .iter()
        .any(|path| path.len() > 2048)
    {
        return Err("Speech tool paths must be 2,048 characters or fewer.".to_string());
    }

    let mut config = crate::config::AppConfig::load();
    config.llm.ollama_url = ollama_url.to_string();
    config.llm.ollama_model = ollama_model.to_string();
    config.llm.vision_model = vision_model.to_string();
    config.speech_tools.whisper_exe = whisper_exe.to_string();
    config.speech_tools.whisper_model = whisper_model.to_string();
    config.speech_tools.piper_exe = piper_exe.to_string();
    config.speech_tools.piper_model = piper_model.to_string();
    config.save()?;
    Ok(AppSettings {
        ollama_url: config.llm.ollama_url,
        ollama_model: config.llm.ollama_model,
        vision_model: config.llm.vision_model,
        whisper_exe: config.speech_tools.whisper_exe,
        whisper_model: config.speech_tools.whisper_model,
        piper_exe: config.speech_tools.piper_exe,
        piper_model: config.speech_tools.piper_model,
    })
}

#[tauri::command]
pub fn get_memory_snapshot(state: State<'_, AppState>) -> Result<crate::memory::ByteMemory, String> {
    state.memory.lock()
        .map(|memory| memory.clone())
        .map_err(|_| "Saved memory is unavailable.".to_string())
}

#[tauri::command]
pub fn clear_conversation_history(state: State<'_, AppState>) -> Result<(), String> {
    let mut memory = state.memory.lock().map_err(|_| "Saved memory is unavailable.".to_string())?;
    memory.recent_conversation.clear();
    memory.save()
}

#[tauri::command]
pub fn reset_saved_memory(state: State<'_, AppState>) -> Result<(), String> {
    let mut memory = state.memory.lock().map_err(|_| "Saved memory is unavailable.".to_string())?;
    let mut cleared = crate::memory::ByteMemory::default();
    cleared.user_profile.name.clear();
    cleared.user_profile.preferences.clear();
    cleared.user_profile.habits.clear();
    cleared.is_first_run = false;
    *memory = cleared;
    memory.save()
}

#[tauri::command]
pub fn get_wake_word_enabled(state: State<'_, AppState>) -> bool {
    state.wake_word_enabled.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn set_wake_word_enabled(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let mut config = crate::config::AppConfig::load();
    config.wake_word_enabled = enabled;
    config.save()?;
    state.wake_word_enabled.store(enabled, Ordering::SeqCst);
    Ok(enabled)
}

#[tauri::command]
pub async fn get_system_health(state: State<'_, AppState>) -> Result<crate::diagnostics::SystemHealthReport, String> {
    Ok(crate::diagnostics::HealthChecker::check_system(&state.http_client).await)
}

/// Check whether an interaction context is still the currently active one and has not been cancelled
fn is_interaction_valid(ctx: &InteractionContext, state: &AppState) -> bool {
    if ctx.is_cancelled() {
        return false;
    }
    if let Ok(guard) = state.active_interaction.lock() {
        if let Some(active) = guard.as_ref() {
            return active.id == ctx.id && !active.is_cancelled();
        }
    }
    false
}

/// Helper: Sanitize conversation history to only retain completed user/assistant pairs.
/// Enforces strict alternation (user -> assistant) and guarantees history always ends with an assistant message.
#[cfg(test)]
fn sanitize_conversation_history(history: &[crate::memory::ConversationTurn]) -> Vec<ChatMessage> {
    let mut result = Vec::new();

    for turn in history {
        match turn.role.as_str() {
            "user" => {
                // Never allow two consecutive user messages in history.
                if result.last().map(|m: &ChatMessage| m.role.as_str()) == Some("user") {
                    continue;
                }

                result.push(ChatMessage {
                    role: "user".to_string(),
                    content: turn.message.clone(),
                });
            }

            "assistant" => {
                // Assistant must follow a user message.
                if result.last().map(|m: &ChatMessage| m.role.as_str()) == Some("user") {
                    result.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: turn.message.clone(),
                    });
                }
            }

            _ => {}
        }
    }

    // Critical invariant: history prior to current turn MUST end with an assistant message
    if result.last().map(|m| m.role.as_str()) == Some("user") {
        result.pop();
    }

    result
}

// Re-export is_simple_greeting from prompts module
pub use crate::prompts::is_simple_greeting;

/// Determines whether transcription is empty, ambient noise, or a bracketed non-verbal artifact
pub fn is_empty_or_noise(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || trimmed.chars().all(|c| !c.is_alphanumeric())
}

/// Active conversation lifecycle loop (Listen -> Auto-stop on silence -> Transcribe -> Query -> Play TTS)
#[tauri::command]
pub async fn start_interaction(state: State<'_, AppState>, window: tauri::WebviewWindow) -> Result<ProcessResult, String> {
    run_interaction(state, window, None).await
}

#[tauri::command]
pub async fn start_text_interaction(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    text: String,
) -> Result<ProcessResult, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("Type a message before sending it.".to_string());
    }
    if text.chars().count() > 4000 {
        return Err("Messages must be 4,000 characters or fewer.".to_string());
    }
    run_interaction(state, window, Some(text)).await
}

async fn run_interaction(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    text_input: Option<String>,
) -> Result<ProcessResult, String> {
    // Pre-empt any previous interaction if one was still in flight
    if let Ok(mut active) = state.active_interaction.lock() {
        if let Some(prev) = active.take() {
            log::info!("Pre-empting previous active interaction: {}", prev.id);
            prev.cancel();
        }
    }
    audio::stop_audio();
    audio::reset_cancellation();

    // Create a fresh, dedicated InteractionContext for this turn
    let context = InteractionContext::new();
    if let Ok(mut active) = state.active_interaction.lock() {
        *active = Some(context.clone());
    }

    let pipeline_start = StageTimer::start();
    let mut telemetry = PipelineTelemetry::new();

    state.is_interacting.store(true, Ordering::SeqCst);

    let text_mode = text_input.is_some();
    let transcription;
    if let Some(text) = text_input {
        state.presence.set_state(&window, PresenceState::Thinking);
        let _ = window.emit("processing", ());
        transcription = text;
    } else {
        // Start audio capture and wait for silence detection or cancellation.
        state.presence.set_state(&window, PresenceState::Listening);
        let capture_timer = StageTimer::start();
        if let Err(error) = audio::start_recording(&state, window.clone()) {
            state.is_recording.store(false, Ordering::SeqCst);
            state.is_interacting.store(false, Ordering::SeqCst);
            state.presence.set_state(&window, PresenceState::Idle);
            if let Ok(mut active) = state.active_interaction.lock() {
                if active.as_ref().map(|current| current.id) == Some(context.id) {
                    active.take();
                }
            }
            return Err(error);
        }

        while state.is_recording.load(Ordering::SeqCst) {
            if context.is_cancelled() {
                log::info!("Interaction {} cancelled during audio recording.", context.id);
                state.presence.set_state(&window, PresenceState::Cancelled);
                state.is_interacting.store(false, Ordering::SeqCst);
                return Ok(ProcessResult {
                    transcription: String::new(),
                    thinking: String::new(),
                    response: "Cancelled.".to_string(),
                    auto_listen: false,
                });
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        telemetry.vad_capture_ms = capture_timer.elapsed_ms();

        if context.is_cancelled() {
            state.presence.set_state(&window, PresenceState::Cancelled);
            state.is_interacting.store(false, Ordering::SeqCst);
            return Ok(ProcessResult {
                transcription: String::new(),
                thinking: String::new(),
                response: "Cancelled.".to_string(),
                auto_listen: false,
            });
        }

        state.presence.set_state(&window, PresenceState::Thinking);
        let _ = window.emit("processing", ());
        let raw_samples = {
            let buffer = state.recorded_data.lock().unwrap();
            buffer.clone()
        };
        if raw_samples.is_empty() {
            state.presence.set_state(&window, PresenceState::Idle);
            state.is_interacting.store(false, Ordering::SeqCst);
            return Err("No audio data recorded".to_string());
        }

        let sample_rate = *state.device_sample_rate.lock().unwrap();
        let resampled = audio::resample(&raw_samples, sample_rate, 16000);
        let temp_wav = BytePaths::temp_wav();
        audio::save_wav(&temp_wav, &resampled)
            .map_err(|e| format!("Failed to save WAV: {}", e))?;

        log::info!("Starting Whisper transcription...");
        let whisper_timer = StageTimer::start();
        let wav_for_transcribe = temp_wav.clone();
        let transcribe_task = tokio::task::spawn_blocking(move || {
            let speech = SpeechService::default();
            speech.transcribe(&wav_for_transcribe)
        });

        let cancel_stt = context.cancellation.cancelled();
        let transcription_res: Result<String, ByteError> = tokio::select! {
            res = tokio::time::timeout(std::time::Duration::from_secs(STT_TIMEOUT_SECS), transcribe_task) => {
                match res {
                    Ok(join_res) => match join_res {
                        Ok(trans_res) => trans_res.map_err(ByteError::TranscriptionFailed),
                        Err(e) => Err(ByteError::TranscriptionFailed(format!("Transcription task join error: {}", e))),
                    },
                    Err(_) => Err(ByteError::Timeout { stage: "transcription".to_string(), duration_ms: STT_TIMEOUT_SECS * 1000 }),
                }
            }
            _ = cancel_stt => {
                let _ = std::fs::remove_file(&temp_wav);
                log::info!("Interaction {} cancelled during STT transcription.", context.id);
                state.presence.set_state(&window, PresenceState::Cancelled);
                state.is_interacting.store(false, Ordering::SeqCst);
                return Ok(ProcessResult {
                    transcription: String::new(),
                    thinking: String::new(),
                    response: "Cancelled.".to_string(),
                    auto_listen: false,
                });
            }
        };
        let _ = std::fs::remove_file(&temp_wav);
        telemetry.stt_ms = whisper_timer.elapsed_ms();

        transcription = match transcription_res {
            Ok(text) => {
                log::info!("Whisper transcription finished in {}ms: '{}'", telemetry.stt_ms, text);
                text
            }
            Err(err) => {
                log::error!("STT stage error: {}", err);
                let friendly = err.to_user_friendly_message();
                state.presence.set_state(&window, PresenceState::Idle);
                state.is_interacting.store(false, Ordering::SeqCst);
                return Ok(ProcessResult {
                    transcription: String::new(),
                    thinking: String::new(),
                    response: friendly,
                    auto_listen: false,
                });
            }
        };
    }

    if context.is_cancelled() {
        log::info!("Interaction {} cancelled after STT transcription.", context.id);
        state.presence.set_state(&window, PresenceState::Cancelled);
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: String::new(),
            response: "Cancelled.".to_string(),
            auto_listen: false,
        });
    }

    // Check if user is asking to stop/cancel
    let clean_lower = transcription.trim().to_lowercase();
    if clean_lower == "stop" || clean_lower == "cancel" || clean_lower.contains("stop speaking") || clean_lower.contains("shut up") || clean_lower.contains("be quiet") {
        log::info!("Stop/Cancel voice command recognized: '{}'", clean_lower);
        context.cancel();
        state.presence.set_state(&window, PresenceState::Cancelled);
        audio::stop_audio();
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: String::new(),
            response: "Stopped.".to_string(),
            auto_listen: false,
        });
    }

    // Check if user is asking to clear conversation memory
    if clean_lower == "clear memory" || clean_lower == "clear history" || clean_lower == "reset memory" || clean_lower == "reset conversation" || clean_lower == "forget everything" {
        log::info!("Clearing conversation history memory on user command...");
        if let Ok(mut mem) = state.memory.lock() {
            mem.recent_conversation.clear();
            let _ = mem.save();
        }
        state.presence.set_state(&window, PresenceState::Idle);
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: String::new(),
            response: "I've cleared our conversation history.".to_string(),
            auto_listen: false,
        });
    }

    if is_empty_or_noise(&transcription) {
        log::info!("Whisper transcription is empty or ambient noise: '{}'", transcription);
        state.presence.set_state(&window, PresenceState::Idle);
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: String::new(),
            response: String::new(),
            auto_listen: false,
        });
    }

    // 5. Process input with Model Manager
    let mut final_response = String::new();
    let mut final_thinking = String::new();
    let mut is_tool = false;
    let mut has_error = false;
    let mut executed_tool_name: Option<String> = None;

    // Check if there is a pending tool call awaiting confirmation
    let pending_call_opt = {
        let mut p = state.pending_tool_call.lock().unwrap();
        p.take()
    };

    if let Some(pending_call) = pending_call_opt {
        match confirmation_intent(&transcription) {
        ConfirmationIntent::Confirm => {
            log::info!("User confirmed pending tool execution: {:?}", pending_call);
            let registry = tools::ToolRegistry::new();
            let tool_exec_timer = StageTimer::start();
            let execute_result = match tokio::time::timeout(
                std::time::Duration::from_secs(TOOL_TIMEOUT_SECS),
                registry.execute_tool(&pending_call, &window, &state.http_client, &transcription, &state, true)
            ).await {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    log::error!("Pending tool execution error: {}", e);
                    ByteError::ToolExecutionFailed { tool: pending_call.tool.clone(), error: e }.to_user_friendly_message()
                }
                Err(_) => {
                    log::warn!("Pending tool execution timed out after {}s", TOOL_TIMEOUT_SECS);
                    ByteError::Timeout { stage: format!("tool '{}'", pending_call.tool), duration_ms: TOOL_TIMEOUT_SECS * 1000 }.to_user_friendly_message()
                }
            };
            telemetry.tool_ms = tool_exec_timer.elapsed_ms();
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            final_response = execute_result;
            is_tool = true;
            executed_tool_name = Some(pending_call.tool.clone());
        }
        ConfirmationIntent::Cancel => {
            log::info!("User cancelled pending tool execution.");
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            final_response = "Okay, cancelled.".to_string();
        }
        ConfirmationIntent::Unclear => {
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            log::info!("User sent new command instead of confirming: {}", transcription);
        }
        }
    }

    if final_response.is_empty() {
        let registry = tools::ToolRegistry::new();
        let tools_schema = registry.get_openai_tools();
        let model_manager = ModelManager::default();

        let current_time_str = chrono::Local::now().format("%A, %B %d, %Y %I:%M %p").to_string();
        let is_greeting = prompts::is_simple_greeting(&transcription);
        let active_window = if is_greeting { String::new() } else { get_active_window_title() };

        // Only load saved preferences when the current message clearly asks for them.
        let trans_lower = transcription.to_lowercase();
        let asks_for_saved_preferences = [
            "my usual",
            "my favorite",
            "my preference",
            "my habit",
            "what do you remember about my",
        ]
        .iter()
        .any(|phrase| trans_lower.contains(phrase));
        let relevant_memory = if !is_greeting && asks_for_saved_preferences {
            state.memory.lock().ok().map(|m| {
                m.user_profile.preferences.iter()
                    .map(|(k, v)| format!("- {}: {}", k, v))
                    .collect::<Vec<String>>()
                    .join("\n")
            })
        } else {
            None
        };

        // Build minimum sufficient system instructions
        let system_instructions = prompts::build_system_prompt(
            &current_time_str,
            &active_window,
            is_greeting,
            relevant_memory.as_deref(),
        );

        let mut messages: Vec<ChatMessage> = Vec::new();
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_instructions,
        });

        // Each request starts with only the current message. Past conversations
        // are saved for user review, never silently reintroduced as context.
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: transcription.clone(),
        });

        let llm_timer = StageTimer::start();
        let messages_debug = serde_json::to_string_pretty(&messages)
            .unwrap_or_else(|_| "<failed to serialize messages>".to_string());
        log::debug!("LLM prompt messages payload:\n{}", messages_debug);
        log::info!("Interaction: Querying local model manager with native tools...");

    let cancel_llm = context.cancellation.cancelled();
    let llm_fut = model_manager.execute(
        Capability::ToolCalling,
        &state.http_client,
        &messages,
        Some(&tools_schema),
    );

    let llm_result = tokio::select! {
        res = tokio::time::timeout(std::time::Duration::from_secs(LLM_TIMEOUT_SECS), llm_fut) => {
            match res {
                Ok(call_res) => match call_res {
                    Ok(r) => r,
                    Err(err_msg) => {
                        log::error!("Interaction model error: {}", err_msg);
                        let byte_err = if err_msg.contains("Connection refused") || err_msg.contains("error sending request") {
                            ByteError::ModelUnreachable(err_msg)
                        } else {
                            ByteError::ModelError(err_msg)
                        };
                        final_response = byte_err.to_user_friendly_message();
                        has_error = true;
                        LlmResult {
                            content: String::new(),
                            tool_call: None,
                        }
                    }
                },
                Err(_) => {
                    log::warn!("LLM query timed out after {}s", LLM_TIMEOUT_SECS);
                    let byte_err = ByteError::Timeout { stage: "AI generation".to_string(), duration_ms: LLM_TIMEOUT_SECS * 1000 };
                    final_response = byte_err.to_user_friendly_message();
                    has_error = true;
                    LlmResult {
                        content: String::new(),
                        tool_call: None,
                    }
                }
            }
        }
        _ = cancel_llm => {
            log::info!("Interaction {} cancelled during LLM generation.", context.id);
            state.presence.set_state(&window, PresenceState::Cancelled);
            state.is_interacting.store(false, Ordering::SeqCst);
            return Ok(ProcessResult {
                transcription,
                thinking: String::new(),
                response: "Cancelled.".to_string(),
                auto_listen: false,
            });
        }
    };
    telemetry.llm_ms = llm_timer.elapsed_ms();

    // Verify interaction is still active before processing results or calling tools
    if !is_interaction_valid(&context, &state) {
        log::info!("Interaction {} superseded or cancelled before tool/response handling.", context.id);
        state.presence.set_state(&window, PresenceState::Cancelled);
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: String::new(),
            response: "Cancelled.".to_string(),
            auto_listen: false,
        });
    }

    if !has_error {
        log::info!(
            "Model responded in {}ms: content='{}', tool_call={:?}",
            telemetry.llm_ms,
            llm_result.content,
            llm_result.tool_call
        );

        let (thinking, clean_content) = tools::extract_think(&llm_result.content);
        final_thinking = thinking;

        if let Some(tool_call) = llm_result.tool_call {
            is_tool = true;
            log::info!("Tool call parsed: {:?}", tool_call);

            let mut requires_conf = false;
            let mut display_name = tool_call.tool.clone();
            if let Some(tool) = registry.get_tool(&tool_call.tool) {
                requires_conf = tool.requires_confirmation();
                display_name = tool.name().to_string();
            }

            if requires_conf {
                log::info!("Tool {} requires user confirmation.", display_name);
                {
                    let mut pending = state.pending_tool_call.lock().unwrap();
                    *pending = Some(tool_call.clone());
                }
                {
                    let mut pending_t = state.pending_tool_transcription.lock().unwrap();
                    *pending_t = Some(transcription.clone());
                }
                final_response = format!("I need your confirmation to execute '{}'. Would you like me to proceed?", display_name);
            } else {
                executed_tool_name = Some(display_name.clone());
                let tool_exec_timer = StageTimer::start();
                let tool_res = tokio::time::timeout(
                    std::time::Duration::from_secs(TOOL_TIMEOUT_SECS),
                    registry.execute_tool(&tool_call, &window, &state.http_client, &transcription, &state, false)
                ).await;

                match tool_res {
                    Ok(Ok(tool_output)) => {
                        log::info!("Tool success: {}", tool_output);
                        final_response = tool_output;
                    }
                    Ok(Err(e)) => {
                        log::error!("Tool error: {}", e);
                        final_response = ByteError::ToolExecutionFailed { tool: display_name, error: e }.to_user_friendly_message();
                        has_error = true;
                    }
                    Err(_) => {
                        log::warn!("Tool execution timed out after {}s", TOOL_TIMEOUT_SECS);
                        final_response = ByteError::Timeout { stage: format!("tool '{}'", display_name), duration_ms: TOOL_TIMEOUT_SECS * 1000 }.to_user_friendly_message();
                        has_error = true;
                    }
                }
                telemetry.tool_ms = tool_exec_timer.elapsed_ms();
            }
        } else {
            final_response = clean_content;
        }
    }
    }

    if final_response.is_empty() {
        final_response = "I'm not sure how to help with that.".to_string();
    }

    // Verify interaction is still active before emitting UI mood or preparing audio
    if !is_interaction_valid(&context, &state) {
        log::info!("Interaction {} superseded or cancelled before audio synthesis.", context.id);
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: final_thinking,
            response: final_response,
            auto_listen: false,
        });
    }

    // 6. Build structured assistant response and derive UI mood state cleanly
    let assistant_resp = AssistantResponse::simple_text(final_response, final_thinking);
    let response_text = assistant_resp.text.clone();
    let mood = assistant_resp.ui_state.mood.clone();

    if let Ok(mut m) = state.mood.lock() {
        *m = mood.clone();
    }

    // Conversation State Machine: Resolve intent & auto-listen state
    let has_pending = state.pending_tool_call.lock().unwrap().is_some();
    let intent = resolve_conversation_intent(
        &transcription,
        &response_text,
        is_tool,
        has_pending,
        has_error,
    );
    let auto_listen = !text_mode && intent.should_auto_listen();
    log::info!("Conversation intent resolved: {:?} (auto_listen: {})", intent, auto_listen);

    let is_farewell = intent == crate::core::ConversationIntent::Farewell;

    // 7. TTS synthesis and audio playback
    let text_to_speak = response_text.clone();
    let window_clone = window.clone();
    let mood_clone = mood.clone();

    // Step A: Audio Synthesis (WAV generation) wrapped in tokio::select! for cancellation
    let synth_timer = StageTimer::start();
    let speech = SpeechService::default();
    let synth_text = text_to_speak.clone();
    let cancel_synth = context.cancellation.cancelled();
    let synth_task = tokio::task::spawn_blocking(move || {
        speech.synthesize(&synth_text, 1.0)
    });

    let wav_path_res = tokio::select! {
        res = tokio::time::timeout(std::time::Duration::from_secs(TTS_SYNTH_TIMEOUT_SECS), synth_task) => {
            match res {
                Ok(join_res) => match join_res {
                    Ok(r) => r.map_err(ByteError::SynthesisFailed),
                    Err(e) => Err(ByteError::SynthesisFailed(format!("TTS task join error: {}", e))),
                },
                Err(_) => {
                    log::warn!("TTS synthesis timed out after {}s", TTS_SYNTH_TIMEOUT_SECS);
                    Err(ByteError::Timeout { stage: "speech synthesis".to_string(), duration_ms: TTS_SYNTH_TIMEOUT_SECS * 1000 })
                }
            }
        }
        _ = cancel_synth => {
            log::info!("Interaction {} cancelled during TTS synthesis.", context.id);
            state.presence.set_state(&window, PresenceState::Cancelled);
            state.is_interacting.store(false, Ordering::SeqCst);
            return Ok(ProcessResult {
                transcription,
                thinking: assistant_resp.thinking,
                response: response_text,
                auto_listen: false,
            });
        }
    };

    telemetry.tts_synthesis_ms = synth_timer.elapsed_ms();
    // TTFA: Exact turnaround time from silence detection until audio starts playing
    telemetry.ttfa_ms = telemetry.stt_ms + telemetry.llm_ms + telemetry.tool_ms + telemetry.tts_synthesis_ms;

    // Check interaction validity before audio playback
    if !is_interaction_valid(&context, &state) {
        log::info!("Interaction {} superseded or cancelled before audio playback.", context.id);
        state.presence.set_state(&window, PresenceState::Cancelled);
        if let Ok(wav_path) = wav_path_res {
            let _ = std::fs::remove_file(wav_path);
        }
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: assistant_resp.thinking,
            response: response_text,
            auto_listen: false,
        });
    }

    // Step B: Audio Playback wrapped in tokio::select! for cancellation
    let playback_timer = StageTimer::start();
    if let Ok(wav_path) = wav_path_res {
        state.presence.set_state(&window, PresenceState::Speaking);
        let cancel_play = context.cancellation.cancelled();
        let play_task = tokio::task::spawn_blocking(move || {
            let _ = window_clone.emit("speaking", &mood_clone);
            let speech = SpeechService::default();
            speech.play(&wav_path);
            let _ = window_clone.emit("processing", ());
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = window_clone.emit("speaking", "calm");
            if is_farewell {
                let _ = window_clone.hide();
            }
        });

        tokio::select! {
            res = tokio::time::timeout(std::time::Duration::from_secs(TTS_PLAY_TIMEOUT_SECS), play_task) => {
                if res.is_err() {
                    log::warn!("TTS playback exceeded timeout of {}s; stopping audio.", TTS_PLAY_TIMEOUT_SECS);
                    audio::stop_audio();
                }
            }
            _ = cancel_play => {
                log::info!("Interaction {} cancelled during audio playback.", context.id);
                state.presence.set_state(&window, PresenceState::Cancelled);
                audio::stop_audio();
            }
        }
    }
    telemetry.playback_ms = playback_timer.elapsed_ms();
    telemetry.total_pipeline_ms = pipeline_start.elapsed_ms();

    // Clear active_interaction if this interaction is still the active one
    if let Ok(mut active) = state.active_interaction.lock() {
        if let Some(current) = active.as_ref() {
            if current.id == context.id {
                active.take();
            }
        }
    }

    state.presence.set_state(&window, PresenceState::Idle);
    state.is_interacting.store(false, Ordering::SeqCst);

    // Log structured telemetry breakdown table
    telemetry.print_summary(&transcription, executed_tool_name.as_deref());

    Ok(ProcessResult {
        transcription,
        thinking: assistant_resp.thinking,
        response: response_text,
        auto_listen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ConversationTurn;

    #[test]
    fn test_is_simple_greeting() {
        assert!(is_simple_greeting("Hello"));
        assert!(is_simple_greeting("hello,"));
        assert!(is_simple_greeting("Hello, good morning."));
        assert!(is_simple_greeting("Good morning!"));
        assert!(is_simple_greeting("hi there..."));
        assert!(is_simple_greeting("Hey Byte"));
        assert!(is_simple_greeting("good evening?"));

        // Substantive commands or queries must NOT be classified as simple greetings
        assert!(!is_simple_greeting("Hey Byte, what CPU am I using?"));
        assert!(!is_simple_greeting("Hello, what is the weather today?"));
        assert!(!is_simple_greeting("good morning please open chrome"));
        assert!(!is_simple_greeting("What is her brother in Chennai now?"));
        assert!(!is_simple_greeting("set a timer for 10 minutes"));
    }

    #[test]
    fn test_sanitize_conversation_history_drops_trailing_user_turn() {
        let history = vec![
            ConversationTurn {
                role: "user".to_string(),
                message: "What is her brother in Chennai now?".to_string(),
            },
        ];

        let sanitized = sanitize_conversation_history(&history);
        assert!(sanitized.is_empty(), "Orphan trailing user turn must be dropped to prevent consecutive user turns");
    }

    #[test]
    fn test_sanitize_conversation_history_enforces_strict_pairs() {
        let history = vec![
            ConversationTurn {
                role: "user".to_string(),
                message: "Who was Alan Turing?".to_string(),
            },
            ConversationTurn {
                role: "assistant".to_string(),
                message: "Alan Turing was a British mathematician and pioneer of computer science.".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                message: "What is her brother in Chennai now?".to_string(),
            },
        ];

        let sanitized = sanitize_conversation_history(&history);
        assert_eq!(sanitized.len(), 2, "Only the completed pair should be preserved");
        assert_eq!(sanitized[0].role, "user");
        assert_eq!(sanitized[0].content, "Who was Alan Turing?");
        assert_eq!(sanitized[1].role, "assistant");
    }

    #[test]
    fn test_sanitize_conversation_history_drops_consecutive_user_turns() {
        let history = vec![
            ConversationTurn {
                role: "user".to_string(),
                message: "First query".to_string(),
            },
            ConversationTurn {
                role: "user".to_string(),
                message: "Second query without response".to_string(),
            },
            ConversationTurn {
                role: "assistant".to_string(),
                message: "Answer to query".to_string(),
            },
        ];

        let sanitized = sanitize_conversation_history(&history);
        assert_eq!(sanitized.len(), 2);
        assert_eq!(sanitized[0].content, "First query");
        assert_eq!(sanitized[1].role, "assistant");
    }

    #[test]
    fn test_byte_error_friendly_messages() {
        let unreachable_err = ByteError::ModelUnreachable("Connection refused (os error 10061)".to_string());
        assert!(unreachable_err.to_user_friendly_message().contains("Ollama is running"));

        let timeout_err = ByteError::Timeout { stage: "transcription".to_string(), duration_ms: 10000 };
        assert!(timeout_err.to_user_friendly_message().contains("timed out"));

        let tool_err = ByteError::ToolExecutionFailed {
            tool: "organize_folder".to_string(),
            error: "Permission denied".to_string(),
        };
        assert!(tool_err.to_user_friendly_message().contains("organize_folder"));
    }

    #[test]
    fn test_is_empty_or_noise() {
        // Noise and empty strings must return true
        assert!(is_empty_or_noise(""));
        assert!(is_empty_or_noise("   "));
        assert!(is_empty_or_noise("\n\t  "));
        assert!(is_empty_or_noise("[BLANK_AUDIO]"));
        assert!(is_empty_or_noise("[silence]"));
        assert!(is_empty_or_noise("(sigh)"));
        assert!(is_empty_or_noise("(cough)"));
        assert!(is_empty_or_noise("..."));
        assert!(is_empty_or_noise("???"));
        assert!(is_empty_or_noise("---"));
        assert!(is_empty_or_noise(" . . . "));

        // Substantive speech must return false
        assert!(!is_empty_or_noise("Hello"));
        assert!(!is_empty_or_noise("What is my CPU usage?"));
        assert!(!is_empty_or_noise("Open Chrome"));
        assert!(!is_empty_or_noise("yes"));
        assert!(!is_empty_or_noise("123"));
    }
}
