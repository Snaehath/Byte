//! System tray icon: keeps Byte reachable while it runs in the background.

use std::sync::atomic::Ordering;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use crate::presence::PresenceState;
use crate::AppState;

/// Menu items whose state must follow changes made elsewhere in the app.
struct TrayItems {
    wake_word: CheckMenuItem<Wry>,
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let wake_enabled = app.state::<AppState>().wake_word_enabled.load(Ordering::SeqCst);

    let show = MenuItem::with_id(app, "show", "Show Byte", true, None::<&str>)?;
    let talk = MenuItem::with_id(app, "talk", "Talk to Byte (Ctrl+B)", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let wake_word = CheckMenuItem::with_id(app, "wake_word", "Listen for wake word", true, wake_enabled, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Byte", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[
        &show,
        &talk,
        &settings,
        &PredefinedMenuItem::separator(app)?,
        &wake_word,
        &PredefinedMenuItem::separator(app)?,
        &quit,
    ])?;

    let mut tray = TrayIconBuilder::with_id("byte")
        .tooltip("Byte")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => reveal(app),
            "talk" => {
                reveal(app);
                let _ = app.emit("wakeup", ());
            }
            "settings" => {
                reveal(app);
                let _ = app.emit("open_settings", ());
            }
            "wake_word" => {
                let enabled = app
                    .try_state::<TrayItems>()
                    .and_then(|items| items.wake_word.is_checked().ok())
                    .unwrap_or(false);
                if let Err(e) = apply_wake_word(app, enabled) {
                    log::error!("Tray: couldn't change wake word: {e}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                reveal(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    app.manage(TrayItems { wake_word });
    Ok(())
}

/// Bring the orb back on screen without moving it.
pub fn reveal(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    let state = app.state::<AppState>();
    if state.presence.get_state() == PresenceState::Hidden {
        state.presence.set_state(&window, PresenceState::Idle);
    }
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// Persist the wake word setting and keep the tray and UI in sync.
pub fn apply_wake_word(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let mut config = crate::config::AppConfig::load();
    config.wake_word_enabled = enabled;
    config.save()?;
    app.state::<AppState>().wake_word_enabled.store(enabled, Ordering::SeqCst);
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.wake_word.set_checked(enabled);
    }
    let _ = app.emit("wake_word_changed", enabled);
    Ok(())
}
