# Byte

Byte is a Windows desktop voice assistant built with **Rust, Tauri, and HTML, CSS, and JavaScript**. Its floating window centers on an animated orb that reflects the assistant's current state. Click the orb or press **Ctrl+B** to start talking. Click again or press **Escape** to stop or cancel an interaction.

## How it works

1. Byte captures audio from the microphone and detects when you have finished speaking.
2. It transcribes the recording with a speech recognition engine.
3. It sends the text and relevant conversation context to the configured language model.
4. It speaks the response using the configured speech engine.

Some interactions can continue by listening for another request. The orb and status label show when Byte is listening, thinking, or speaking. Open **Type a message** to use text input and review recent replies.

## AI and speech

Byte's configuration is stored at `%APPDATA%\Byte\config\config.json`. The default AI configuration uses a local Ollama service. Speech recognition and speech output use local Whisper and Piper tools; install their executables and models separately if they are not already available. Set their paths with `BYTE_WHISPER_EXE`, `BYTE_WHISPER_MODEL`, `BYTE_PIPER_EXE`, and `BYTE_PIPER_MODEL`.

## Desktop capabilities

Depending on the request and available configuration, Byte can open applications, control windows, organize files, access the clipboard, control audio and media, read system information, show notifications, and analyze the screen. Screen analysis uses the configured Ollama vision model and asks for confirmation before capturing the screen. It can also keep selected preferences and conversation history.

The floating widget includes a **Wake word** switch. When enabled, Byte listens for its wake word in the background; switch it off to pause detection. Setup also shows component health and lets you configure AI and speech model paths. The setup view lets you review saved memory, clear conversation history, or reset all saved memory. The optional first-run spoken greeting is off by default; set `first_run_greeting_enabled` to `true` to enable it before Byte's first launch.

For screen analysis, install a vision model in Ollama (for example, `ollama pull llava`) and set `llm.vision_model` in the config file to that model's name.

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
