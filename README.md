# Byte

Byte is a Windows desktop voice assistant built with Rust, Tauri, and JavaScript. Talk to Byte to ask questions or request help with tasks on your computer. It can use local or cloud AI services and supports local speech recognition and speech output.

## What it can do

- Listen to voice requests and respond with speech.
- Answer questions using a configured AI provider.
- Open applications, manage files, control windows, and perform other desktop tasks.
- Remember selected preferences and conversation context.

Byte asks for confirmation before carrying out actions that may change files or system settings.

## Requirements

- Windows 10 or 11 and the Rust toolchain.
- An AI provider: Ollama running locally, or a supported cloud provider configured with an API key.
- For local voice processing, Whisper and Piper with their model files. These are optional if you use another supported setup.

## Run

From the project folder, start the development app with:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

Use **Ctrl+B** to toggle Byte.

## Configuration

Configure your AI provider in Byte's configuration file or set its API key in your environment. For local speech, set paths to the Whisper and Piper executables and models using `BYTE_WHISPER_EXE`, `BYTE_WHISPER_MODEL`, `BYTE_PIPER_EXE`, and `BYTE_PIPER_MODEL`.

The app stores its configuration and memory in your Windows user data folder.
