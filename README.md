# Byte

Byte is a hands-free AI assistant for Windows, built with **Rust, Tauri, HTML, CSS, and JavaScript**. Talk to Byte with the wake word, click the orb, or press **Ctrl+B**. The floating orb shows when Byte is listening, thinking, or speaking; its controls appear when you move the pointer over it.

## How it works

1. Byte captures audio from the microphone and detects when you have finished speaking.
2. It transcribes the recording with a speech recognition engine.
3. It sends the request to the configured language model.
4. It speaks the response using the configured speech engine.

Some voice interactions continue by listening for another request. You can also open the compact message input and hear Byte's reply aloud.

## Local and cloud AI

On first launch, choose a local Ollama model or an OpenAI-compatible cloud API. The choice can be changed later from the small settings icon shown when you hover over the orb. Cloud API keys are stored in Windows Credential Manager; requests are sent to the selected provider.

For local use, install [Ollama](https://ollama.com/download) and pull Byte's default models:

```powershell
ollama pull granite4.2:3b
ollama pull llava
```

Byte uses `granite4.2:3b` for chat and `llava` for vision by default. Granite 4.2 is text-only; `granite3.2-vision:2b` is an alternative vision model. [Ministral 3](https://ollama.com/library/ministral-3) can handle both text and images with one model:

```powershell
ollama pull ministral-3:8b
```

Choose `ministral-3:8b` for both chat and vision in Byte's model settings to use it for both. The model download is about 6 GB; its Ollama listing currently requires Ollama 0.13.1. For cloud use, enter the provider's HTTPS endpoint, model name, and API key during setup.

## Desktop capabilities

Depending on the request and available configuration, Byte can open applications, control windows, organize files, access the clipboard, control audio and media, read system information, show notifications, and analyze the screen. Screen analysis asks for confirmation before capturing the screen and sending it to the selected model provider. Byte only saves preferences when asked and does not automatically reuse past conversations.

The wake word switch appears on hover and can pause background listening. The optional first-run spoken greeting is off by default; set `first_run_greeting_enabled` to `true` to enable it.

## Project structure

- `ui/` contains the floating window interface, styles, interaction logic, and orb animation.
- `src-tauri/src/` contains the Rust application, audio pipeline, AI providers, configuration, memory, and desktop tools.
- `src-tauri/tauri.conf.json` contains Tauri application and window settings.
- `src-tauri/capabilities/` contains permissions granted to the interface.

## Requirements

- Windows 10 or 11
- Rust toolchain
- Ollama with a configured model for local AI, or an OpenAI-compatible cloud API key
- Optional: Whisper and Piper executables and model files for local speech processing

## Run

From the project folder:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

Byte is an in-progress project. Available features depend on the current implementation and on which AI and speech services are installed and configured.
