use std::sync::atomic::Ordering;
use tauri::{State, Emitter, Manager};
use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::audio::{self, SpeechService};
use crate::ai::{ChatMessage, LlmResult, ModelManager};
use crate::tools;
use crate::prompts;
use crate::core::{
    resolve_conversation_intent, ConversationIntent, InteractionContext, ByteError,
    CAPTURE_TIMEOUT_SECS, STT_TIMEOUT_SECS, LLM_TIMEOUT_SECS, TOOL_TIMEOUT_SECS, TTS_SYNTH_TIMEOUT_SECS, TTS_PLAY_TIMEOUT_SECS,
};
use crate::diagnostics::{PipelineTelemetry, StageTimer};
use crate::platform::get_active_window_title;
use crate::config::paths::BytePaths;

use crate::presence::PresenceState;

#[derive(Serialize)]
pub struct ProcessResult {
    pub transcription: String,
    pub response: String,
    pub auto_listen: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettings {
    pub provider: String,
    pub ollama_url: String,
    pub text_model: String,
    pub vision_model: String,
    pub cloud_url: String,
    pub cloud_model: String,
    pub api_key_configured: bool,
    pub setup_complete: bool,
}

/// What the settings form sends when saving.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsInput {
    pub provider: String,
    pub ollama_url: String,
    pub cloud_url: String,
    pub cloud_model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatus {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub reachable: bool,
    pub text_installed: bool,
    pub vision_installed: bool,
    pub message: String,
}

#[derive(PartialEq, Eq)]
enum ConfirmationIntent {
    Confirm,
    Cancel,
    Unclear,
}

fn confirmation_intent(transcription: &str) -> ConfirmationIntent {
    let lower = transcription.to_lowercase().replace(['\'', '’'], "");
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
    state.pending_tool_call.lock().unwrap().take();

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
    setup_panel: bool,
) -> Result<(), String> {
    let size = if expanded {
        tauri::LogicalSize::new(400.0, if setup_panel { 600.0 } else { 360.0 })
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
pub fn get_model_settings() -> Result<ModelSettings, String> {
    let config = crate::config::AppConfig::load();
    Ok(ModelSettings {
        provider: config.llm.provider,
        ollama_url: config.llm.ollama_url,
        text_model: config.llm.ollama_model,
        vision_model: config.llm.vision_model,
        cloud_url: config.llm.cloud_url,
        cloud_model: config.llm.cloud_model,
        api_key_configured: crate::ai::credentials::saved_cloud_api_key().is_some(),
        setup_complete: config.llm.setup_complete,
    })
}

fn valid_endpoint(url: &str, https_only: bool) -> bool {
    let scheme_ok = url.starts_with("https://") || (!https_only && url.starts_with("http://"));
    scheme_ok && !url.chars().any(char::is_whitespace) && url.len() <= 2048
}

fn valid_model_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
}

/// Accept a base URL (https://api.openai.com/v1) or the full chat completions URL.
fn normalize_cloud_endpoint(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.ends_with("/chat/completions") {
        url.to_string()
    } else {
        format!("{url}/chat/completions")
    }
}

#[tauri::command]
pub fn save_model_settings(settings: ModelSettingsInput, api_key: String) -> Result<ModelSettings, String> {
    let provider = settings.provider.trim();
    if provider != "ollama" && provider != "cloud" {
        return Err("Choose local or cloud models.".to_string());
    }
    let mut config = crate::config::AppConfig::load();

    let ollama_url = settings.ollama_url.trim();
    if !ollama_url.is_empty() {
        if !valid_endpoint(ollama_url, false) {
            return Err("Enter a valid Ollama endpoint, such as http://localhost:11434.".to_string());
        }
        config.llm.ollama_url = normalize_ollama_endpoint(ollama_url)?;
    }

    let cloud_url = settings.cloud_url.trim();
    let cloud_model = settings.cloud_model.trim();
    let api_key = api_key.trim();
    if provider == "cloud" {
        if !valid_endpoint(cloud_url, true) {
            return Err("Enter a valid HTTPS API endpoint.".to_string());
        }
        if !valid_model_name(cloud_model) {
            return Err("Enter a valid cloud model name, such as gpt-4.1-mini.".to_string());
        }
        if api_key.is_empty() && crate::ai::credentials::saved_cloud_api_key().is_none() {
            return Err("Paste your API key to use cloud models.".to_string());
        }
    }
    if api_key.len() > 2048 || api_key.chars().any(char::is_whitespace) {
        return Err("That API key doesn't look right.".to_string());
    }
    if !api_key.is_empty() {
        crate::ai::credentials::store_cloud_api_key(api_key)?;
    }
    if valid_endpoint(cloud_url, true) {
        config.llm.cloud_url = normalize_cloud_endpoint(cloud_url);
    }
    if valid_model_name(cloud_model) {
        config.llm.cloud_model = cloud_model.to_string();
    }

    config.llm.provider = provider.to_string();
    config.llm.setup_complete = true;
    config.save()?;
    get_model_settings()
}

#[tauri::command]
pub fn remove_cloud_api_key() -> Result<ModelSettings, String> {
    crate::ai::credentials::delete_cloud_api_key();
    let mut config = crate::config::AppConfig::load();
    if config.llm.is_cloud() {
        config.llm.provider = "ollama".to_string();
        config.save()?;
    }
    get_model_settings()
}

fn cloud_status(ok: bool, message: impl Into<String>) -> CloudStatus {
    CloudStatus { ok, message: message.into() }
}

/// Verify the API key and model by fetching the model's details.
/// Uses the typed key if given, otherwise the saved one.
#[tauri::command]
pub async fn check_cloud_connection(
    state: State<'_, AppState>,
    cloud_url: String,
    cloud_model: String,
    api_key: String,
) -> Result<CloudStatus, String> {
    let cloud_url = cloud_url.trim();
    let model = cloud_model.trim();
    if !valid_endpoint(cloud_url, true) {
        return Ok(cloud_status(false, "Enter a valid HTTPS API endpoint."));
    }
    if !valid_model_name(model) {
        return Ok(cloud_status(false, "Enter a valid model name."));
    }
    let typed = api_key.trim().to_string();
    let key = if typed.is_empty() { crate::ai::credentials::saved_cloud_api_key() } else { Some(typed) };
    let Some(key) = key else {
        return Ok(cloud_status(false, "Paste your API key first."));
    };
    let base = normalize_cloud_endpoint(cloud_url).trim_end_matches("/chat/completions").to_string();
    let res = state.http_client
        .get(format!("{base}/models/{model}"))
        .bearer_auth(key)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    let Ok(res) = res else {
        return Ok(cloud_status(false, "Couldn't reach the API. Check your internet connection."));
    };
    Ok(match res.status().as_u16() {
        200..=299 => cloud_status(true, format!("Connected. {model} will handle text, tools and vision.")),
        401 | 403 => cloud_status(false, "The API key was rejected."),
        404 => cloud_status(false, format!("{model} isn't available on this account.")),
        429 => cloud_status(false, "Rate limited or out of credit."),
        code => cloud_status(false, format!("The API returned an error ({code}).")),
    })
}

/// Accept either the Ollama root or the OpenAI-compatible chat URL and store the chat URL.
fn normalize_ollama_endpoint(endpoint: &str) -> Result<String, String> {
    Ok(format!("{}/v1/chat/completions", ollama_api_root(endpoint)?))
}

fn ollama_api_root(endpoint: &str) -> Result<String, String> {
    let mut root = endpoint.trim().trim_end_matches('/').to_string();
    if root.ends_with("/v1/chat/completions") {
        root.truncate(root.len() - "/v1/chat/completions".len());
    } else if root.ends_with("/v1") {
        root.truncate(root.len() - "/v1".len());
    }
    if root.starts_with("http://") || root.starts_with("https://") {
        Ok(root.trim_end_matches('/').to_string())
    } else {
        Err("Enter a valid Ollama endpoint first.".to_string())
    }
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

async fn list_ollama_models(client: &reqwest::Client, ollama_url: &str) -> Result<Vec<String>, String> {
    let root = ollama_api_root(ollama_url)?;
    let response = client.get(format!("{root}/api/tags"))
        .timeout(std::time::Duration::from_secs(5))
        .send().await
        .map_err(|_| format!("Ollama isn't responding at {root}. Make sure it's running."))?
        .error_for_status().map_err(|e| format!("Could not list Ollama models: {e}"))?;
    Ok(response.json::<OllamaTagsResponse>().await
        .map_err(|e| format!("Ollama returned an invalid model list: {e}"))?
        .models.into_iter().map(|model| model.name).collect())
}

fn model_installed(installed: &[String], model: &str) -> bool {
    installed.iter().any(|name| {
        name == model || (!model.contains(':') && name == &format!("{model}:latest"))
    })
}

#[tauri::command]
pub async fn get_model_status(state: State<'_, AppState>, ollama_url: String) -> Result<ModelStatus, String> {
    let config = crate::config::AppConfig::load();
    match list_ollama_models(&state.http_client, &ollama_url).await {
        Ok(installed) => {
            let text_installed = model_installed(&installed, &config.llm.ollama_model);
            let vision_installed = model_installed(&installed, &config.llm.vision_model);
            let message = match (text_installed, vision_installed) {
                (true, true) => "Connected. All models are ready.".to_string(),
                _ => "Connected. Some models still need to be downloaded.".to_string(),
            };
            Ok(ModelStatus { reachable: true, text_installed, vision_installed, message })
        }
        Err(message) => Ok(ModelStatus {
            reachable: false,
            text_installed: false,
            vision_installed: false,
            message,
        }),
    }
}

#[tauri::command]
pub async fn pull_ollama_model(state: State<'_, AppState>, ollama_url: String, model: String) -> Result<ModelStatus, String> {
    let config = crate::config::AppConfig::load();
    let model = model.trim();
    if model != config.llm.ollama_model && model != config.llm.vision_model {
        return Err("Only Byte's built-in models can be downloaded.".to_string());
    }
    let root = ollama_api_root(&ollama_url)?;
    let response = state.http_client.post(format!("{root}/api/pull"))
        .timeout(std::time::Duration::from_secs(2 * 60 * 60))
        .json(&serde_json::json!({ "name": model, "stream": false }))
        .send().await
        .map_err(|e| format!("Could not download {model}: {e}"))?
        .error_for_status().map_err(|e| format!("Ollama could not download {model}: {e}"))?;
    let body: serde_json::Value = response.json().await
        .map_err(|e| format!("Ollama returned an invalid download result: {e}"))?;
    if let Some(error) = body.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("Download failed: {error}"));
    }
    get_model_status(state, ollama_url).await
}

#[tauri::command]
pub fn reset_saved_memory(state: State<'_, AppState>) -> Result<(), String> {
    let mut memory = state.memory.lock().map_err(|_| "Saved memory is unavailable.".to_string())?;
    let reminders = std::mem::take(&mut memory.scheduled_reminders);
    *memory = crate::memory::ByteMemory { is_first_run: false, scheduled_reminders: reminders, ..Default::default() };
    memory.save()?;
    drop(memory);
    if let Ok(mut session) = state.session.lock() {
        session.clear();
    }
    Ok(())
}

#[tauri::command]
pub fn get_wake_word_enabled(state: State<'_, AppState>) -> bool {
    state.wake_word_enabled.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn set_wake_word_enabled(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    crate::tray::apply_wake_word(&app, enabled)?;
    Ok(enabled)
}

/// Open the folder that holds byte.log.
#[tauri::command]
pub fn open_log_folder(app: tauri::AppHandle) -> Result<(), String> {
    crate::tray::open_log_folder(&app)
}

/// Pending confirmations expire so a stray "yes" much later can't trigger them.
const CONFIRMATION_WINDOW: std::time::Duration = std::time::Duration::from_secs(90);

/// Determines whether transcription is empty, ambient noise, or a bracketed non-verbal artifact
pub fn is_empty_or_noise(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.is_empty()
        || (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || trimmed.chars().all(|c| !c.is_alphanumeric())
}

fn is_stop_command(text: &str) -> bool {
    matches!(text, "stop" | "cancel" | "never mind" | "nevermind")
        || ["stop speaking", "shut up", "be quiet"].iter().any(|p| text.contains(p))
}

fn reply(transcription: impl Into<String>, response: impl Into<String>, auto_listen: bool) -> ProcessResult {
    ProcessResult { transcription: transcription.into(), response: response.into(), auto_listen }
}

/// Is this interaction still the active one and not cancelled?
fn is_interaction_valid(ctx: &InteractionContext, state: &AppState) -> bool {
    !ctx.is_cancelled()
        && state.active_interaction.lock().ok()
            .and_then(|guard| guard.as_ref().map(|active| active.id == ctx.id && !active.is_cancelled()))
            .unwrap_or(false)
}

/// Release the interaction and leave the orb in `presence`.
fn finish(state: &AppState, window: &tauri::WebviewWindow, ctx: &InteractionContext, presence: PresenceState) {
    if let Ok(mut active) = state.active_interaction.lock() {
        if active.as_ref().is_some_and(|current| current.id == ctx.id) {
            active.take();
        }
    }
    state.is_recording.store(false, Ordering::SeqCst);
    state.is_interacting.store(false, Ordering::SeqCst);
    state.presence.set_state(window, presence);
}

fn cancelled(state: &AppState, window: &tauri::WebviewWindow, ctx: &InteractionContext, transcription: &str) -> Result<ProcessResult, String> {
    log::info!("Interaction {} cancelled.", ctx.id);
    finish(state, window, ctx, PresenceState::Cancelled);
    Ok(reply(transcription, "Cancelled.", false))
}

/// Query the text model, honoring cancellation and the stage timeout.
/// `Ok(None)` means the interaction was cancelled.
async fn ask_model(
    ctx: &InteractionContext,
    client: &reqwest::Client,
    messages: &[ChatMessage],
    tools: Option<&[serde_json::Value]>,
) -> Result<Option<LlmResult>, ByteError> {
    let manager = ModelManager::default();
    let cloud = manager.is_cloud();
    tokio::select! {
        res = tokio::time::timeout(std::time::Duration::from_secs(LLM_TIMEOUT_SECS), manager.chat(client, messages, tools)) => {
            match res {
                Ok(Ok(result)) => Ok(Some(result)),
                Ok(Err(msg)) if cloud => Err(ByteError::CloudError(msg)),
                Ok(Err(msg)) if msg.contains("Network request failed") => Err(ByteError::ModelUnreachable(msg)),
                Ok(Err(msg)) => Err(ByteError::ModelError(msg)),
                Err(_) => Err(ByteError::Timeout { stage: "AI generation".to_string(), duration_ms: LLM_TIMEOUT_SECS * 1000 }),
            }
        }
        _ = ctx.cancellation.cancelled() => Ok(None),
    }
}

/// Run a tool with a timeout. Tool errors are already phrased for the user.
async fn run_tool(
    registry: &tools::ToolRegistry,
    call: &tools::ToolCall,
    window: &tauri::WebviewWindow,
    state: &State<'_, AppState>,
    transcription: &str,
    confirmed: bool,
) -> Result<String, String> {
    let limit = registry.get_tool(&call.tool).map_or(TOOL_TIMEOUT_SECS, |t| t.timeout_secs());
    match tokio::time::timeout(
        std::time::Duration::from_secs(limit),
        registry.execute_tool(call, window, &state.http_client, transcription, state, confirmed),
    ).await {
        Ok(result) => result,
        Err(_) => Err(ByteError::Timeout { stage: call.tool.replace('_', " "), duration_ms: limit * 1000 }.to_user_friendly_message()),
    }
}

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

/// Record the microphone until silence (or the capture limit) and transcribe it.
/// `Ok(None)` means the interaction was cancelled.
async fn listen_and_transcribe(
    state: &State<'_, AppState>,
    window: &tauri::WebviewWindow,
    ctx: &InteractionContext,
    telemetry: &mut PipelineTelemetry,
) -> Result<Option<String>, ByteError> {
    state.presence.set_state(window, PresenceState::Listening);
    let capture_timer = StageTimer::start();
    audio::start_recording(state, window.clone()).map_err(ByteError::TranscriptionFailed)?;

    let capture_limit = std::time::Duration::from_secs(CAPTURE_TIMEOUT_SECS);
    let started = std::time::Instant::now();
    while state.is_recording.load(Ordering::SeqCst) {
        if ctx.is_cancelled() {
            return Ok(None);
        }
        if started.elapsed() > capture_limit {
            log::info!("Capture reached {}s limit; processing what was heard.", CAPTURE_TIMEOUT_SECS);
            state.is_recording.store(false, Ordering::SeqCst);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    telemetry.vad_capture_ms = capture_timer.elapsed_ms();
    let _ = window.emit("audio_level", 0.0f32);
    if ctx.is_cancelled() {
        return Ok(None);
    }

    state.presence.set_state(window, PresenceState::Thinking);
    let raw_samples = state.recorded_data.lock().map(|b| b.clone()).unwrap_or_default();
    if raw_samples.is_empty() {
        return Ok(Some(String::new()));
    }
    let sample_rate = *state.device_sample_rate.lock().unwrap();
    let resampled = audio::resample(&raw_samples, sample_rate, 16000);
    let temp_wav = BytePaths::temp_wav();
    audio::save_wav(&temp_wav, &resampled)
        .map_err(|e| ByteError::TranscriptionFailed(format!("Failed to save WAV: {e}")))?;

    let stt_timer = StageTimer::start();
    let wav = temp_wav.clone();
    let task = tokio::task::spawn_blocking(move || SpeechService::default().transcribe(&wav));
    let result = tokio::select! {
        res = tokio::time::timeout(std::time::Duration::from_secs(STT_TIMEOUT_SECS), task) => match res {
            Ok(Ok(text)) => text.map(Some).map_err(ByteError::TranscriptionFailed),
            Ok(Err(e)) => Err(ByteError::TranscriptionFailed(format!("Transcription task failed: {e}"))),
            Err(_) => Err(ByteError::Timeout { stage: "transcription".to_string(), duration_ms: STT_TIMEOUT_SECS * 1000 }),
        },
        _ = ctx.cancellation.cancelled() => Ok(None),
    };
    let _ = std::fs::remove_file(&temp_wav);
    telemetry.stt_ms = stt_timer.elapsed_ms();
    if let Ok(Some(text)) = &result {
        log::info!("Whisper finished in {}ms: '{}'", telemetry.stt_ms, text);
    }
    result
}

/// Decide what Byte says (and does) for one user turn. Returns the reply and
/// the tool that ran, if any. `Ok(None)` means the interaction was cancelled.
async fn respond(
    state: &State<'_, AppState>,
    window: &tauri::WebviewWindow,
    ctx: &InteractionContext,
    transcription: &str,
    telemetry: &mut PipelineTelemetry,
) -> Result<Option<(String, Option<String>)>, ByteError> {
    let registry = tools::ToolRegistry::new();

    // A pending confirmation is answered by this turn, if it is a yes/no.
    let pending = state.pending_tool_call.lock().unwrap().take()
        .filter(|(_, asked_at)| asked_at.elapsed() < CONFIRMATION_WINDOW);
    if let Some((call, _)) = pending {
        match confirmation_intent(transcription) {
            ConfirmationIntent::Confirm => {
                let timer = StageTimer::start();
                let output = run_tool(&registry, &call, window, state, transcription, true).await;
                telemetry.tool_ms = timer.elapsed_ms();
                return Ok(Some((output.unwrap_or_else(|e| e), Some(call.tool))));
            }
            ConfirmationIntent::Cancel => return Ok(Some(("Okay, I won't.".to_string(), None))),
            ConfirmationIntent::Unclear => log::info!("Confirmation not given; treating as a new request."),
        }
    }

    let (user_name, saved_preferences) = {
        let memory = state.memory.lock().unwrap();
        let lower = transcription.to_lowercase();
        let wants_preferences = ["my usual", "my favorite", "my favourite", "my preference", "remember about me"]
            .iter().any(|p| lower.contains(p));
        let prefs = wants_preferences.then(|| {
            memory.user_profile.preferences.iter()
                .map(|(k, v)| format!("- {}: {}", k.replace('_', " "), v))
                .collect::<Vec<_>>()
                .join("\n")
        });
        (memory.user_profile.name.clone(), prefs)
    };
    let is_greeting = prompts::is_simple_greeting(transcription);
    let active_window = if is_greeting { String::new() } else { get_active_window_title() };
    let system_prompt = prompts::build_system_prompt(
        &chrono::Local::now().format("%A, %B %-d, %Y, %-I:%M %p").to_string(),
        &active_window,
        Some(user_name.as_str()),
        saved_preferences.as_deref(),
    );

    let mut messages = vec![ChatMessage::new("system", system_prompt)];
    messages.extend(state.session.lock().unwrap().history());
    messages.push(ChatMessage::new("user", transcription));

    let tools_schema = registry.get_openai_tools();
    let llm_timer = StageTimer::start();
    let Some(result) = ask_model(ctx, &state.http_client, &messages, Some(&tools_schema)).await? else {
        return Ok(None);
    };
    telemetry.llm_ms = llm_timer.elapsed_ms();
    log::info!("Model replied in {}ms: content='{}', tool_call={:?}", telemetry.llm_ms, result.content, result.tool_call);

    let Some(call) = result.tool_call else {
        let (_, text) = tools::extract_think(&result.content);
        return Ok(Some((text, None)));
    };
    let Some(tool) = registry.get_tool(&call.tool) else {
        log::warn!("Model requested unknown tool '{}'", call.tool);
        return Ok(Some(("I can't do that yet.".to_string(), None)));
    };
    if !is_interaction_valid(ctx, state) {
        return Ok(None);
    }

    if tool.requires_confirmation() {
        let question = tool.confirmation_prompt(&call.parameters);
        let config = crate::config::AppConfig::load();
        if call.tool == "analyze_screen" && !config.llm.is_cloud() {
            // Load the local vision model while the user answers the question.
            crate::ai::VisionService::warm_up(state.http_client.clone(), config.llm.ollama_url, config.llm.vision_model);
        }
        *state.pending_tool_call.lock().unwrap() = Some((call, std::time::Instant::now()));
        return Ok(Some((question, None)));
    }

    let tool_name = call.tool.clone();
    let timer = StageTimer::start();
    let output = run_tool(&registry, &call, window, state, transcription, false).await;
    telemetry.tool_ms = timer.elapsed_ms();
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            log::warn!("Tool {} failed: {}", tool_name, error);
            return Ok(Some((error, Some(tool_name))));
        }
    };
    log::info!("Tool {} result: {}", tool_name, output);

    // Data tools: let the model answer the actual question from the result.
    if tool.answer_from_result() && is_interaction_valid(ctx, state) {
        messages.push(ChatMessage::new("system", prompts::tool_result_instructions(&tool_name, &output)));
        match ask_model(ctx, &state.http_client, &messages, None).await {
            Ok(Some(answer)) => {
                let (_, text) = tools::extract_think(&answer.content);
                if !text.trim().is_empty() && answer.tool_call.is_none() {
                    return Ok(Some((text, Some(tool_name))));
                }
            }
            Ok(None) => return Ok(None),
            Err(e) => log::warn!("Follow-up answer failed, reading tool output instead: {e}"),
        }
    }
    Ok(Some((output, Some(tool_name))))
}

async fn run_interaction(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
    text_input: Option<String>,
) -> Result<ProcessResult, String> {
    // Pre-empt any interaction still in flight.
    if let Ok(mut active) = state.active_interaction.lock() {
        if let Some(prev) = active.take() {
            log::info!("Pre-empting previous interaction {}", prev.id);
            prev.cancel();
        }
    }
    audio::stop_audio();
    audio::reset_cancellation();

    let ctx = InteractionContext::new();
    if let Ok(mut active) = state.active_interaction.lock() {
        *active = Some(ctx.clone());
    }
    state.is_interacting.store(true, Ordering::SeqCst);
    let pipeline_start = StageTimer::start();
    let mut telemetry = PipelineTelemetry::new();
    let voice_mode = text_input.is_none();

    // 1. Get the user's words.
    let transcription = match text_input {
        Some(text) => {
            state.presence.set_state(&window, PresenceState::Thinking);
            text
        }
        None => match listen_and_transcribe(&state, &window, &ctx, &mut telemetry).await {
            Ok(Some(text)) => text,
            Ok(None) => return cancelled(&state, &window, &ctx, ""),
            Err(err) => {
                log::error!("Voice input failed: {err}");
                finish(&state, &window, &ctx, PresenceState::Idle);
                return Err(err.to_user_friendly_message());
            }
        },
    };
    if ctx.is_cancelled() {
        return cancelled(&state, &window, &ctx, &transcription);
    }

    let lower = transcription.trim().trim_end_matches(['.', '!']).to_lowercase();
    if is_stop_command(&lower) {
        audio::stop_audio();
        state.session.lock().unwrap().clear();
        state.pending_tool_call.lock().unwrap().take();
        return cancelled(&state, &window, &ctx, &transcription);
    }
    if is_empty_or_noise(&transcription) {
        log::info!("Ignoring empty or noise transcription: '{}'", transcription);
        finish(&state, &window, &ctx, PresenceState::Idle);
        return Ok(reply(transcription, "", false));
    }

    // 2. Think and act.
    let (mut response, tool_used, has_error) = match respond(&state, &window, &ctx, &transcription, &mut telemetry).await {
        Ok(Some((text, tool))) => (text, tool, false),
        Ok(None) => return cancelled(&state, &window, &ctx, &transcription),
        Err(err) => {
            log::error!("Model stage failed: {err}");
            (err.to_user_friendly_message(), None, true)
        }
    };
    if response.trim().is_empty() {
        response = "Sorry, I'm not sure how to help with that.".to_string();
    }
    if !is_interaction_valid(&ctx, &state) {
        return cancelled(&state, &window, &ctx, &transcription);
    }

    // 3. Decide whether to keep listening, and remember the exchange for follow-ups.
    let has_pending = state.pending_tool_call.lock().unwrap().is_some();
    let intent = resolve_conversation_intent(&transcription, &response, tool_used.is_some(), has_pending, has_error);
    // Typed messages never switch the microphone on by themselves.
    let auto_listen = voice_mode && intent.should_auto_listen();
    let is_farewell = intent == ConversationIntent::Farewell;
    {
        let mut session = state.session.lock().unwrap();
        if is_farewell || has_error {
            session.clear();
        } else {
            session.record(&transcription, &response);
        }
    }
    log::info!("Intent {:?} (auto_listen: {})", intent, auto_listen);

    // 4. Speak.
    let speed = crate::config::AppConfig::load().audio.voice_speed;
    let synth_timer = StageTimer::start();
    let text = response.clone();
    let synth_task = tokio::task::spawn_blocking(move || SpeechService::default().synthesize(&text, speed));
    let wav = tokio::select! {
        res = tokio::time::timeout(std::time::Duration::from_secs(TTS_SYNTH_TIMEOUT_SECS), synth_task) => match res {
            Ok(Ok(Ok(path))) => Some(path),
            Ok(Ok(Err(e))) => { log::error!("{}", ByteError::SynthesisFailed(e)); None }
            Ok(Err(e)) => { log::error!("TTS task failed: {e}"); None }
            Err(_) => { log::warn!("TTS synthesis timed out after {}s", TTS_SYNTH_TIMEOUT_SECS); None }
        },
        _ = ctx.cancellation.cancelled() => return cancelled(&state, &window, &ctx, &transcription),
    };
    telemetry.tts_synthesis_ms = synth_timer.elapsed_ms();
    telemetry.ttfa_ms = telemetry.stt_ms + telemetry.llm_ms + telemetry.tool_ms + telemetry.tts_synthesis_ms;

    if let Some(wav_path) = wav {
        if !is_interaction_valid(&ctx, &state) {
            let _ = std::fs::remove_file(&wav_path);
            return cancelled(&state, &window, &ctx, &transcription);
        }
        state.presence.set_state(&window, PresenceState::Speaking);
        let playback_timer = StageTimer::start();
        let play_task = tokio::task::spawn_blocking(move || SpeechService::default().play(&wav_path));
        tokio::select! {
            res = tokio::time::timeout(std::time::Duration::from_secs(TTS_PLAY_TIMEOUT_SECS), play_task) => {
                if res.is_err() {
                    log::warn!("Playback exceeded {}s; stopping audio.", TTS_PLAY_TIMEOUT_SECS);
                    audio::stop_audio();
                }
            }
            _ = ctx.cancellation.cancelled() => {
                audio::stop_audio();
                return cancelled(&state, &window, &ctx, &transcription);
            }
        }
        telemetry.playback_ms = playback_timer.elapsed_ms();
    }
    telemetry.total_pipeline_ms = pipeline_start.elapsed_ms();
    telemetry.print_summary(&transcription, tool_used.as_deref());

    finish(&state, &window, &ctx, PresenceState::Idle);
    if is_farewell {
        let _ = window.hide();
    }
    Ok(reply(transcription, response, auto_listen))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirmation_intent() {
        assert!(confirmation_intent("Yes.") == ConfirmationIntent::Confirm);
        assert!(confirmation_intent("go ahead") == ConfirmationIntent::Confirm);
        assert!(confirmation_intent("No, don't") == ConfirmationIntent::Cancel);
        assert!(confirmation_intent("yes, but open chrome first") == ConfirmationIntent::Unclear);
    }

    #[test]
    fn test_stop_commands() {
        assert!(is_stop_command("stop"));
        assert!(is_stop_command("please stop speaking"));
        assert!(!is_stop_command("stop the music"));
    }

    #[test]
    fn test_byte_error_friendly_messages() {
        let unreachable_err = ByteError::ModelUnreachable("Connection refused (os error 10061)".to_string());
        assert!(unreachable_err.to_user_friendly_message().contains("Ollama is running"));

        let timeout_err = ByteError::Timeout { stage: "transcription".to_string(), duration_ms: 10000 };
        assert!(timeout_err.to_user_friendly_message().contains("timed out"));
    }

    #[test]
    fn test_is_empty_or_noise() {
        for noise in ["", "   ", "\n\t  ", "[BLANK_AUDIO]", "[silence]", "(sigh)", "...", "???", " . . . "] {
            assert!(is_empty_or_noise(noise), "{noise:?} should be noise");
        }
        for speech in ["Hello", "What is my CPU usage?", "Open Chrome", "yes", "123"] {
            assert!(!is_empty_or_noise(speech), "{speech:?} should be speech");
        }
    }
}
