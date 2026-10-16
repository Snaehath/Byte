# Byte

Byte is a Windows desktop voice assistant built with **Rust, Tauri, and HTML, CSS, and JavaScript**. Its floating window centers on an animated orb that reflects the assistant's current state. Click the orb or press **Ctrl+B** to start talking. Click again or press **Escape** to stop or cancel an interaction.

## How it works

1. Byte captures audio from the microphone and detects when you have finished speaking.
2. It transcribes the recording with a speech recognition engine.
3. It sends the text and relevant conversation context to the configured language model.
4. It speaks the response using the configured speech engine.

Some interactions can continue by listening for another request. The orb and status label show when Byte is listening, thinking, or speaking.

## AI and speech

Byte's configuration is stored in a JSON file in your Windows user data folder. The default AI configuration uses a local Ollama service. Speech recognition and speech output can use local Whisper and Piper tools. You may need to install these tools and their model files separately; their paths can be set with `BYTE_WHISPER_EXE`, `BYTE_WHISPER_MODEL`, `BYTE_PIPER_EXE`, and `BYTE_PIPER_MODEL`.

## Desktop capabilities

Depending on the request and available configuration, Byte can open applications, control windows, organize files, access the clipboard, control audio and media, read system information, show notifications, and analyze the screen. It can also keep selected preferences and conversation history. Some actions ask for confirmation before proceeding.

## Project structure

- `ui/` contains the floating window interface, styles, interaction logic, and orb animation.
- `src-tauri/src/` contains the Rust application, audio pipeline, AI providers, configuration, memory, and desktop tools.
- `src-tauri/tauri.conf.json` contains Tauri application and window settings.
- `src-tauri/capabilities/` contains permissions granted to the interface.

The root `src/` and `Cargo.toml` are separate from the Tauri desktop app. Use the Tauri manifest to run the desktop application.

## Requirements

- Windows 10 or 11
- Rust toolchain
- Ollama with a configured model, or a supported cloud AI provider
- Optional: Whisper and Piper executables and model files for local speech processing

## Run

From the project folder:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

Byte is an in-progress project. Available features depend on the current implementation and on which AI and speech services are installed and configured.
