use std::sync::atomic::Ordering;
use tauri::{State, Emitter};
use serde::Serialize;
use crate::AppState;
use crate::audio::{self, SpeechService};
use crate::ai::{Capability, ModelManager};
use crate::ai::llm::{ChatMessage, LlmResult};
use crate::tools;
use crate::prompts;
use crate::core::{AssistantResponse, resolve_conversation_intent, InteractionContext};
use crate::diagnostics::{PipelineTelemetry, StageTimer};
use crate::platform::get_active_window_title;
use crate::config::paths::BytePaths;

#[derive(Serialize)]
pub struct ProcessResult {
    pub transcription: String,
    pub thinking: String,
    pub response: String,
    pub auto_listen: bool,
}

#[tauri::command]
pub fn stop_action(state: State<'_, AppState>) -> Result<(), String> {
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

    Ok(())
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

/// Active conversation lifecycle loop (Listen -> Auto-stop on silence -> Transcribe -> Query -> Play TTS)
#[tauri::command]
pub async fn start_interaction(state: State<'_, AppState>, window: tauri::WebviewWindow) -> Result<ProcessResult, String> {
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

    // 1. Start audio capture (silence detection runs automatically)
    let capture_timer = StageTimer::start();
    audio::start_recording(&state, window.clone())?;

    // 2. Wait while recording is active (silence detector or manual click sets is_recording = false)
    while state.is_recording.load(Ordering::SeqCst) {
        if context.is_cancelled() {
            log::info!("Interaction {} cancelled during audio recording.", context.id);
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
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription: String::new(),
            thinking: String::new(),
            response: "Cancelled.".to_string(),
            auto_listen: false,
        });
    }

    // 3. Emit processing state: recording done, now thinking
    let _ = window.emit("processing", ());

    // 4. Fetch the recorded samples
    let raw_samples = {
        let buffer = state.recorded_data.lock().unwrap();
        buffer.clone()
    };

    if raw_samples.is_empty() {
        state.is_interacting.store(false, Ordering::SeqCst);
        return Err("No audio data recorded".to_string());
    }

    let sample_rate = *state.device_sample_rate.lock().unwrap();
    let resampled = audio::resample(&raw_samples, sample_rate, 16000);

    // Save to temp WAV file
    let temp_wav = BytePaths::temp_wav();
    audio::save_wav(&temp_wav, &resampled).map_err(|e| format!("Failed to save WAV: {}", e))?;

    // Transcribe speech using SpeechService (Whisper STT)
    let speech_service = SpeechService::default();
    log::info!("Starting Whisper transcription...");
    let whisper_timer = StageTimer::start();
    let transcription = speech_service.transcribe(&temp_wav)?;
    telemetry.stt_ms = whisper_timer.elapsed_ms();
    log::info!("Whisper transcription finished in {}ms: '{}'", telemetry.stt_ms, transcription);
    let _ = std::fs::remove_file(&temp_wav);

    if context.is_cancelled() {
        log::info!("Interaction {} cancelled after STT transcription.", context.id);
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
        audio::stop_audio();
        state.is_interacting.store(false, Ordering::SeqCst);
        return Ok(ProcessResult {
            transcription,
            thinking: String::new(),
            response: "Stopped.".to_string(),
            auto_listen: false,
        });
    }

    let trimmed = transcription.trim();
    let is_noise = (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || trimmed.chars().all(|c| !c.is_alphabetic());

    if trimmed.is_empty() || is_noise {
        log::info!("Whisper transcription is empty or ambient noise: '{}'", transcription);
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
        let is_confirmed = {
            let lower = transcription.to_lowercase();
            lower.contains("yes") || lower.contains("sure") || lower.contains("confirm") || lower.contains("go ahead") || lower.contains("do it") || lower.contains("ok") || lower.contains("okay")
        };
        let is_cancelled = {
            let lower = transcription.to_lowercase();
            lower.contains("no") || lower.contains("cancel") || lower.contains("stop") || lower.contains("never") || lower.contains("don't")
        };

        if is_confirmed {
            log::info!("User confirmed pending tool execution: {:?}", pending_call);
            let registry = tools::ToolRegistry::new();
            let tool_exec_timer = StageTimer::start();
            let execute_result = match registry.execute_tool(&pending_call, &window, &state.http_client, &transcription, &state).await {
                Ok(output) => output,
                Err(e) => {
                    log::error!("Pending tool execution error: {}", e);
                    format!("I failed to perform that action: {}", e)
                }
            };
            telemetry.tool_ms = tool_exec_timer.elapsed_ms();
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            final_response = execute_result;
            is_tool = true;
            executed_tool_name = Some(pending_call.tool.clone());
        } else if is_cancelled {
            log::info!("User cancelled pending tool execution.");
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            final_response = "Okay, cancelled.".to_string();
        } else {
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            log::info!("User sent new command instead of confirming: {}", transcription);
        }
    }

    if final_response.is_empty() {
        let registry = tools::ToolRegistry::new();
        let tools_schema = registry.get_openai_tools();
        let model_manager = ModelManager::default();

        let current_time_str = chrono::Local::now().format("%A, %B %d, %Y %I:%M %p").to_string();
        let profile_context = {
            if let Ok(mem) = state.memory.lock() {
                let pref_str = mem.user_profile.preferences.iter()
                    .map(|(k, v)| format!("- {}: {}", k, v))
                    .collect::<Vec<String>>()
                    .join("\n");
                let habits_str = mem.user_profile.habits.join(", ");
                format!(
                    "User Profile:\nName: {}\nPreferences:\n{}\nHabits: {}\n\n",
                    mem.user_profile.name, pref_str, habits_str
                )
            } else {
                String::new()
            }
        };

        let active_window = get_active_window_title();
        let system_instructions = format!(
            "{}\n\nActive Window: {}\n\n{}",
            prompts::get_system_instructions(&current_time_str),
            active_window,
            profile_context
        );

        let mut messages: Vec<ChatMessage> = Vec::new();
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_instructions,
        });

        if let Ok(mem) = state.memory.lock() {
            for turn in &mem.recent_conversation {
                messages.push(ChatMessage {
                    role: turn.role.clone(),
                    content: turn.message.clone(),
                });
            }
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: transcription.clone(),
        });

        let llm_timer = StageTimer::start();
        log::info!("Interaction: Querying local model manager with native tools...");

    let cancel_llm = context.cancellation.cancelled();
    let llm_fut = model_manager.execute(
        Capability::ToolCalling,
        &state.http_client,
        &messages,
        Some(&tools_schema),
    );

    let llm_result = tokio::select! {
        res = llm_fut => {
            match res {
                Ok(r) => r,
                Err(err_msg) => {
                    log::error!("Interaction model error: {}", err_msg);
                    final_response = "I'm having trouble reaching my local AI provider. Please make sure Ollama is running.".to_string();
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
                executed_tool_name = Some(display_name);
                let tool_exec_timer = StageTimer::start();
                match registry.execute_tool(&tool_call, &window, &state.http_client, &transcription, &state).await {
                    Ok(tool_output) => {
                        log::info!("Tool success: {}", tool_output);
                        final_response = tool_output;
                    }
                    Err(e) => {
                        log::error!("Tool error: {}", e);
                        final_response = format!("I encountered an issue: {}", e);
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
    let auto_listen = intent.should_auto_listen();
    log::info!("Conversation intent resolved: {:?} (auto_listen: {})", intent, auto_listen);

    let is_farewell = intent == crate::core::ConversationIntent::Finished && (
        response_text.to_lowercase().contains("bye") || response_text.to_lowercase().contains("farewell")
    );

    // Record turn in persistent memory (only if interaction is still active and valid)
    if is_interaction_valid(&context, &state) {
        if let Ok(mut mem) = state.memory.lock() {
            mem.recent_conversation.push(crate::memory::ConversationTurn {
                role: "user".to_string(),
                message: transcription.clone(),
            });
            mem.recent_conversation.push(crate::memory::ConversationTurn {
                role: "assistant".to_string(),
                message: response_text.clone(),
            });
            if mem.recent_conversation.len() > 10 {
                let excess = mem.recent_conversation.len() - 10;
                mem.recent_conversation.drain(0..excess);
            }
            let _ = mem.save();
        }
    }

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
        res = synth_task => {
            match res {
                Ok(r) => r,
                Err(e) => Err(format!("TTS task join error: {}", e)),
            }
        }
        _ = cancel_synth => {
            log::info!("Interaction {} cancelled during TTS synthesis.", context.id);
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
            _ = play_task => {}
            _ = cancel_play => {
                log::info!("Interaction {} cancelled during audio playback.", context.id);
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
