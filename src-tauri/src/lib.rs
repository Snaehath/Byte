use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::process::Command as StdCommand;
use tauri::{Emitter, Manager};

pub struct Command;

impl Command {
    pub fn new<S: AsRef<std::ffi::OsStr>>(program: S) -> StdCommand {
        let mut cmd = StdCommand::new(program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd
    }
}

pub mod config;
pub mod prompts;
pub mod audio;
pub mod ai;
pub mod memory;
pub mod core;
pub mod tools;
pub mod platform;
pub mod diagnostics;
pub mod commands;

// Re-exports for backwards-compatibility
pub use memory::{ByteMemory, UserProfile, Reminder, ConversationTurn};

// Managed state for Tauri to share recording, interactions, memory and mood safely across threads
pub struct AppState {
    pub is_recording: Arc<AtomicBool>,
    pub recorded_data: Arc<Mutex<Vec<f32>>>,
    pub device_sample_rate: Mutex<u32>,
    pub is_interacting: Arc<AtomicBool>,
    pub http_client: reqwest::Client,
    pub memory: Arc<Mutex<ByteMemory>>,
    pub mood: Arc<Mutex<String>>,
    pub pending_tool_call: Mutex<Option<tools::ToolCall>>,
    pub pending_tool_transcription: Mutex<Option<String>>,
    pub active_interaction: Arc<Mutex<Option<crate::core::InteractionContext>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let is_interacting = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let is_interacting_clone = Arc::clone(&is_interacting);
    let is_interacting_for_wake = Arc::clone(&is_interacting);

    let is_recording = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let is_recording_clone = Arc::clone(&is_recording);
    let is_recording_for_wake = Arc::clone(&is_recording);

    let memory = Arc::new(Mutex::new(ByteMemory::load()));
    let memory_clone = Arc::clone(&memory);
    let mood = Arc::new(Mutex::new("calm".to_string()));

    tauri::Builder::default()
        .manage(AppState {
            is_recording: is_recording_clone,
            recorded_data: Arc::new(Mutex::new(Vec::new())),
            device_sample_rate: Mutex::new(0),
            is_interacting: is_interacting_clone,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .unwrap(),
            memory,
            mood,
            pending_tool_call: Mutex::new(None),
            pending_tool_transcription: Mutex::new(None),
            active_interaction: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_interaction,
            commands::stop_action,
            commands::get_system_health
        ])
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Spawn proactive background cognitive loop for scheduled reminders
            let app_handle_for_loop = app.handle().clone();
            let memory_for_loop = Arc::clone(&memory_clone);
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let mut to_trigger = Vec::new();
                    {
                        if let Ok(mut mem) = memory_for_loop.lock() {
                            let before_len = mem.scheduled_reminders.len();
                            mem.scheduled_reminders.retain(|r| {
                                if now_ms >= r.target_time_ms {
                                    to_trigger.push(r.clone());
                                    false
                                } else {
                                    true
                                }
                            });
                            if mem.scheduled_reminders.len() != before_len {
                                let _ = mem.save();
                            }
                        }
                    }

                    for reminder in to_trigger {
                        log::info!("Proactive background loop: Triggering reminder: {}", reminder.label);
                        let text = format!("Reminder: {}", reminder.label);
                        let window_opt = app_handle_for_loop.get_webview_window("main");

                        std::thread::spawn(move || {
                            if let Some(ref w) = window_opt {
                                let _ = w.emit::<String>("speaking", "excited".to_string());
                            }
                            audio::speak_text(&text);
                            if let Some(ref w) = window_opt {
                                let _ = w.emit::<String>("speaking", "calm".to_string());
                            }
                        });
                    }
                }
            });

            // Register global shortcut Ctrl+B
            #[cfg(desktop)]
            {
                use tauri::Manager;
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
                let ctrl_b = Shortcut::new(Some(Modifiers::CONTROL), Code::KeyB);
                let ctrl_b_clone = ctrl_b.clone();
                let app_handle = app.handle().clone();
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
                                if shortcut == &ctrl_b_clone {
                                    log::info!("Global shortcut Ctrl+B pressed!");
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        if window.is_visible().unwrap_or(false) {
                                            let _ = window.hide();
                                        } else {
                                            let _ = window.show();
                                            let _ = window.set_focus();
                                            let _ = app_handle.emit("wakeup", ());
                                        }
                                    }
                                }
                            }
                        })
                        .build()
                )?;
                app.global_shortcut().register(ctrl_b)?;
            }

            // Start background voice wake word detector
            let app_handle_for_wake = app.handle().clone();
            audio::start_wake_word_detector(is_recording_for_wake, is_interacting_for_wake, app_handle_for_wake);

            // Greet on first run only
            let memory_for_hello = Arc::clone(&memory_clone);
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let mut should_greet = false;
                if let Ok(mut mem) = memory_for_hello.lock() {
                    if mem.is_first_run {
                        should_greet = true;
                        mem.is_first_run = false;
                        let _ = mem.save();
                    }
                }
                if should_greet {
                    audio::speak_text("Hello! I am Byte, your desktop assistant. I am ready to help.");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
