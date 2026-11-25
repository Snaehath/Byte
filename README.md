<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Byte app icon" width="128" height="128" />
</p>

<h1 align="center">Byte</h1>

Byte is a hands-free AI assistant for Windows, built with **Rust, Tauri, HTML, CSS, and JavaScript**. Talk to Byte with the wake word, click the orb, or press **Ctrl+B**. The floating orb shows when Byte is listening, thinking, or speaking; its controls appear when you move the pointer over it.

## How it works

1. Byte captures audio from the microphone and detects when you have finished speaking.
2. It transcribes the recording with a speech recognition engine.
3. It sends the request to the configured language model.
4. It speaks the response using the configured speech engine.

Some voice interactions continue by listening for another request. You can also open the compact message input and hear Byte's reply aloud.

## AI models

### Local (default)

Byte runs on your device through [Ollama](https://ollama.com/download) and works with any Ollama model. Choose models in **Settings → Models → Local**: one model for conversation and tool calling, and a vision model for screen questions, or turn on **Use it for vision too** to run a single model that handles text, tools, and images. Install models with `ollama pull`; Settings lists what is installed.

Suggested models:

| Purpose | Model |
| --- | --- |
| Conversation and tool calling | `granite4.2:3b` |
| Screen understanding (vision) | `minicpm-v4.6:latest` |

```powershell
ollama pull granite4.2:3b
ollama pull minicpm-v4.6:latest
```

For desktop actions, pick a conversation model that supports tool calling.

### Cloud (optional)

Switch **Models** to **Cloud** and paste an OpenAI API key. OpenAI is the only cloud provider supported for now. One model (default `gpt-4.1-mini`) handles conversation, tool calling, and screen analysis. The key is stored in Windows Credential Manager, never in the config file. In cloud mode your messages, and screenshots when you ask Byte to look at your screen, are sent to OpenAI; speech recognition and the voice still run locally.

### Speech

Byte currently supports [Whisper](https://github.com/ggml-org/whisper.cpp) (`whisper-cli.exe` with a `ggml-*.bin` model) for speech recognition and [Piper](https://github.com/rhasspy/piper) with a voice such as `en_US-lessac-high` for speech. Place them in `%APPDATA%\Byte\models`, or set their paths under `speech_tools` in `%APPDATA%\Byte\config\config.json`.

## Desktop capabilities

Byte can open apps (including Microsoft Store apps), websites and folders, play music or videos on YouTube, search the web, minimize/maximize/close the window you're working in, change volume, brightness and media playback, read or copy clipboard text, show notifications, set spoken timers, report CPU/RAM/uptime, and answer questions about your screen. Looking at the screen always asks for a spoken "yes" first.

Within one conversation Byte remembers the last few exchanges so follow-up questions work. This short-term context lives only in memory and resets after three idle minutes or a goodbye. Byte only saves your name and preferences when you ask it to remember them.

## Running in the background

Byte lives in the system tray. Left-click the tray icon to show the orb; right-click for Talk, Settings, the wake word switch, and Quit. Closing or hiding the orb (Ctrl+B) keeps Byte running in the tray, and launching Byte again brings the running copy forward. The optional first-run spoken greeting is off by default; set `first_run_greeting_enabled` to `true` to enable it.

## Project structure

- `ui/` contains the floating window interface, styles, interaction logic, and orb animation.
- `src-tauri/src/` contains the Rust application, audio pipeline, AI providers, configuration, memory, and desktop tools.
- `src-tauri/tauri.conf.json` contains Tauri application and window settings.
- `src-tauri/capabilities/` contains permissions granted to the interface.

## Requirements

- Windows 10 or 11
- Rust toolchain
- Ollama with at least one model installed, or an OpenAI API key
- Whisper and Piper with their model files (the supported speech engines)

## Run

From the project folder:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

## Build

Create a Windows installer from the project folder:

```powershell
Set-Location src-tauri
cargo tauri build
```

The app is named **Byte**. Its source icon is `src-tauri/icons/icon.png`; Tauri uses the `.ico` and platform-specific icon files in `src-tauri/icons/` when packaging the app.

Byte is an in-progress project. Available features depend on the current implementation and on which AI and speech services are installed and configured.
