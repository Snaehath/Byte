# Byte

Byte is a Windows desktop voice assistant built with Rust and Tauri. It can answer questions, use voice input and speech, and perform desktop tasks such as opening apps and managing files.

## Requirements

- Windows 10 or 11
- Rust toolchain
- For local AI, Ollama with a supported model
- Optional: Whisper and Piper binaries and models for local speech recognition and speech output

## Run

From the project folder:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

## Configuration

Byte can use a local Ollama model or a cloud provider. Set the provider's API key in your environment or configure it in Byte's config file. To use local speech, install Whisper and Piper and set their executable and model paths with the `BYTE_WHISPER_EXE`, `BYTE_WHISPER_MODEL`, `BYTE_PIPER_EXE`, and `BYTE_PIPER_MODEL` environment variables.

Use **Ctrl+B** to toggle Byte.
