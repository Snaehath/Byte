use std::sync::atomic::Ordering;
use tauri::{State, Emitter};
use serde::Serialize;
use crate::AppState;
use crate::audio::{self, SpeechService};
use crate::ai::{Capability, ModelManager};
use crate::ai::llm::{ChatMessage, LlmResult};
use crate::tools;
use crate::prompts;
use crate::core::AssistantResponse;
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
    audio::stop_audio();
    state.is_recording.store(false, Ordering::SeqCst);
    state.is_interacting.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn get_system_health(state: State<'_, AppState>) -> Result<crate::diagnostics::SystemHealthReport, String> {
    Ok(crate::diagnostics::HealthChecker::check_system(&state.http_client).await)
}

/// Active conversation lifecycle loop (Listen -> Auto-stop on silence -> Transcribe -> Query -> Play TTS)
#[tauri::command]
pub async fn start_interaction(state: State<'_, AppState>, window: tauri::WebviewWindow) -> Result<ProcessResult, String> {
    state.is_interacting.store(true, Ordering::SeqCst);

    // 1. Start audio capture (silence detection runs automatically)
    audio::start_recording(&state, window.clone())?;

    // 2. Wait while recording is active (silence detector or manual click sets is_recording = false)
    while state.is_recording.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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
    let whisper_start = std::time::Instant::now();
    let transcription = speech_service.transcribe(&temp_wav)?;
    log::info!("Whisper transcription finished in {}ms: '{}'", whisper_start.elapsed().as_millis(), transcription);
    let _ = std::fs::remove_file(&temp_wav);

    // Check if user is asking to stop/cancel
    let clean_lower = transcription.trim().to_lowercase();
    if clean_lower == "stop" || clean_lower == "cancel" || clean_lower.contains("stop speaking") || clean_lower.contains("shut up") || clean_lower.contains("be quiet") {
        log::info!("Stop/Cancel command recognized: '{}'", clean_lower);
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
    let mut auto_listen = false;
    let mut is_tool = false;
    let mut has_error = false;

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
            let execute_result = match registry.execute_tool(&pending_call, &window, &state.http_client, &transcription, &state).await {
                Ok(output) => output,
                Err(e) => {
                    log::error!("Pending tool execution error: {}", e);
                    format!("I failed to perform that action: {}", e)
                }
            };
            let _ = state.pending_tool_transcription.lock().unwrap().take();
            final_response = execute_result;
            is_tool = true;
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

        let model_start = std::time::Instant::now();
        log::info!("Interaction: Querying local model manager with native tools...");

        let llm_result = match model_manager.execute(
            Capability::ToolCalling,
            &state.http_client,
            &messages,
            Some(&tools_schema),
        ).await {
            Ok(res) => res,
            Err(err_msg) => {
                log::error!("Interaction model error: {}", err_msg);
                final_response = "I'm having trouble reaching my local AI provider. Please make sure Ollama is running.".to_string();
                has_error = true;
                LlmResult {
                    content: String::new(),
                    tool_call: None,
                }
            }
        };

        if !has_error {
            log::info!(
                "Model responded in {}ms: content='{}', tool_call={:?}",
                model_start.elapsed().as_millis(),
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
                }
            } else {
                final_response = clean_content;
            }
        }
    }

    if final_response.is_empty() {
        final_response = "I'm not sure how to help with that.".to_string();
    }

    // 6. Build structured assistant response and derive UI mood state cleanly
    let assistant_resp = AssistantResponse::simple_text(final_response, final_thinking);
    let response_text = assistant_resp.text.clone();
    let mood = assistant_resp.ui_state.mood.clone();

    if let Ok(mut m) = state.mood.lock() {
        *m = mood.clone();
    }

    let trans_lower = transcription.to_lowercase();
    let response_lower = response_text.to_lowercase();

    // Check if user is dismissing/closing the interaction
    let is_user_dismissal = trans_lower.contains("thank")
        || trans_lower.contains("nothing")
        || trans_lower.contains("that's all")
        || trans_lower.contains("that is all")
        || trans_lower.contains("that's it")
        || trans_lower.contains("that is it")
        || trans_lower.contains("nevermind")
        || trans_lower.contains("never mind")
        || trans_lower.contains("i'm good")
        || trans_lower.contains("im good")
        || trans_lower.contains("no need")
        || trans_lower == "ok"
        || trans_lower == "okay"
        || trans_lower == "alright"
        || trans_lower == "got it"
        || trans_lower == "cool";

    let is_farewell = response_lower.contains("goodbye")
        || response_lower.contains("bye")
        || response_lower.contains("see you")
        || response_lower.contains("farewell")
        || response_lower.contains("have a great day")
        || response_lower.contains("have a nice day")
        || response_lower.contains("have a good day")
        || response_lower.contains("take care")
        || response_lower.contains("happy coding")
        || response_lower.contains("you're welcome")
        || response_lower.contains("you are welcome")
        || response_lower.contains("anytime")
        || response_lower.contains("my pleasure")
        || response_lower.contains("no problem");

    let has_pending = state.pending_tool_call.lock().unwrap().is_some();
    let assistant_asked_question = response_text.trim().ends_with('?');

    if !has_error && !is_user_dismissal && !is_farewell {
        if has_pending {
            auto_listen = true;
        } else if !is_tool && assistant_asked_question {
            auto_listen = true;
        }
    }

    // Record turn in persistent memory
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

    // 7. TTS synthesis and audio playback
    let text_to_speak = response_text.clone();
    let window_clone = window.clone();
    let mood_clone = mood.clone();

    let _ = tokio::task::spawn_blocking(move || {
        let _ = window_clone.emit("speaking", &mood_clone);
        let speech = SpeechService::default();
        let _ = speech.speak(&text_to_speak, 1.0);
        let _ = window_clone.emit("processing", ());
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = window_clone.emit("speaking", "calm");
        if is_farewell {
            let _ = window_clone.hide();
        }
    }).await;

    state.is_interacting.store(false, Ordering::SeqCst);

    Ok(ProcessResult {
        transcription,
        thinking: assistant_resp.thinking,
        response: response_text,
        auto_listen,
    })
}
