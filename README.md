# Byte

Byte is a Windows desktop voice assistant built with **Rust, Tauri, and HTML, CSS, and JavaScript**. Its floating window centers on an animated orb that reflects the assistant's current state. Click the orb or press **Ctrl+B** to start talking. Click again or press **Escape** to stop or cancel an interaction.

## How it works

1. Byte captures audio from the microphone and detects when you have finished speaking.
2. It transcribes the recording with a speech recognition engine.
3. It sends the request to the configured language model.
4. It speaks the response using the configured speech engine.

Some interactions can continue by listening for another request. The orb and status label show when Byte is listening, thinking, or speaking. Open **Type a message** to send a written request and hear Byte's reply aloud.

## Local AI and speech models

Byte uses [Ollama](https://ollama.com/download) locally for chat and screen vision. Install Ollama, then pull Byte's default models:

```powershell
ollama pull granite4.2:3b
ollama pull llava
```

Byte uses `granite4.2:3b` for chat and `llava` for vision by default. Granite 4.2 is text-only; if you prefer a Granite vision model, pull `granite3.2-vision:2b` and set `llm.vision_model` to that name. For a single model that can handle both text and images, Ollama offers [Ministral 3](https://ollama.com/library/ministral-3):

```powershell
ollama pull ministral-3:8b
```

Set both `llm.ollama_model` and `llm.vision_model` to `ministral-3:8b` in `%APPDATA%\Byte\config\config.json` to use it for both. This model is about 6 GB and its Ollama listing currently requires Ollama 0.13.1.

## Desktop capabilities

Depending on the request and available configuration, Byte can open applications, control windows, organize files, access the clipboard, control audio and media, read system information, show notifications, and analyze the screen. Screen analysis uses the configured Ollama vision model and asks for confirmation before capturing the screen. Byte only saves preferences when asked and does not automatically reuse past conversations.

The floating widget includes a **Wake word** switch. When enabled, Byte listens for its wake word in the background; switch it off to pause detection. AI and speech model paths are configured in the app config file. The optional first-run spoken greeting is off by default; set `first_run_greeting_enabled` to `true` to enable it before Byte's first launch.

For screen analysis, install a vision model in Ollama and set `llm.vision_model` to its name. Byte's default is `llava`; `ministral-3:8b` can handle both chat and vision when configured for both fields.

## Project structure

- `ui/` contains the floating window interface, styles, interaction logic, and orb animation.
- `src-tauri/src/` contains the Rust application, audio pipeline, AI providers, configuration, memory, and desktop tools.
- `src-tauri/tauri.conf.json` contains Tauri application and window settings.
- `src-tauri/capabilities/` contains permissions granted to the interface.

## Requirements

- Windows 10 or 11
- Rust toolchain
- Ollama with a configured model
- Optional: Whisper and Piper executables and model files for local speech processing

## Run

From the project folder:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

Byte is an in-progress project. Available features depend on the current implementation and on which AI and speech services are installed and configured.
