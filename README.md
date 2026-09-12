# Byte: Desktop Voice Assistant & AI Companion

**Byte** is a premium, local-first personal desktop AI voice companion and OS automation copilot for Windows built using **Rust (Tauri v2)** and **Vanilla Web Technologies (HTML/CSS/JS Canvas)**. It combines local hardware voice processing (Whisper STT and Piper TTS) with flexible local/cloud LLM intelligence (Granite 3B / Qwen 3 4B via Ollama / OpenRouter / Nvidia NIM) to execute system actions, control windows, play music, analyze your screen, organize files, and engage in continuous voice conversation.

---

## 🔮 Visual Identity: The "Energy Core"

Byte departs from generic circular buttons and amoeba-like biological blobs in favor of a **living quantum energy core**:

> **80% Sphere + 20% Living Energy**  
> An intelligent floating energy nucleus with a subtle breathing field, conveying machine consciousness and high energy density rather than cellular fluid.

```text
    Previous Concept (Blob/Amoeba)             Byte Energy Core (AI Nucleus)

             ╭───╮                                      ╭────╮
          ╭──╯   ╰╮                                   ╭─╯    ╰─╮
         ╱         ╲                                 │    ◉     │
        │    ◉      │             ───►               │          │
         ╲       ╭─╯                                  ╰─╮    ╭─╯
          ╰──╮───╯                                      ╰────╯
             ╰╯
       Organic / Irregular                      Near-Spherical & Breathing
```

### Visual Engineering Highlights
- **Near-Spherical Geometry (90–95% Circular)**: Subtly deformed perimeter ($\pm 3\text{--}6\%$ of base radius) avoiding both sterile static circles and uncontrolled amoebic bulges.
- **8–12 Harmonic Micro-Waves**: High-harmonic perimeter wave synthesis produces a silky, continuous breathing shimmer.
- **Radiant Intelligence Nucleus**: An intense high-lumen pure white core (`#ffffff`) surrounded by concentrated violet/cyan/emerald energy, communicating active AI cognition.
- **Dual-Layer Atmospheric Aura**: Inner corona tightly hugs the nucleus while an independent outer ethereal aura breathes and pulses with presence state changes.
- **Gravitational Orbital Particles**: Ambient embers travel in 3 distinct elliptical planetary tracks with front/back 3D depth occlusion rather than random debris.
- **State-Driven Physical Behavior**:
  - **Idle**: Smooth circular silhouette, gentle 3-second breathing pulse, calm celestial blue/violet aura.
  - **Listening**: Real-time microphone RMS audio reactivity producing subtle acoustic micro-ripples across the surface.
  - **Thinking**: Core contracts and densifies; radiant white center intensifies; a focused perimeter traveling wave sweeps around; the outer aura expands in a rhythmic 1.5-second cognitive pulse; and orbital particles accelerate in tighter tracks.
  - **Speaking**: Aura expands outward rhythmically with voice synthesis; surface ripples with warm radiant energy.
  - **Cancelled**: Smooth contraction and gentle settling back to idle.
- **High-DPI Razor Sharpness**: Automatic scaling across 4K, 125%, 150%, and 175% Windows desktop display scaling.

---

## Key Features

### 🎙️ Conversational Voice Pipeline
- **Local STT (Speech-to-Text)**: Automatically transcribes voice queries using local `whisper-cli.exe` running the high-performance `ggml-tiny.en.bin` model (~200ms latency).
- **Local TTS (Text-to-Speech)**: Plays natural spoken answers using `piper.exe` with standard high-quality `.onnx` voices (`en_US-lessac-medium.onnx`).
- **Room-Noise Calibrated VAD**: Automatically calibrates ambient sound (200ms) and detects speech termination (1.5s trailing silence), ending recordings hands-free.
- **Greeting Context Isolation**: Trivial greetings (*"hello"*, *"good morning"*) automatically bypass prior conversation history, ensuring the model never hallucinates or repeats stale task baggage.
- **Strict History Sanitization**: Enforces strict `user` $\leftrightarrow$ `assistant` alternation, pairing completed turns and purging orphan user queries.
- **Continuous Auto-Listen Followup**: Triggers the microphone automatically after non-destructive responses, enabling natural back-and-forth dialogue.
- **Immediate Voice Pre-emption**: Stop commands (*"stop"*, *"cancel"*, *"be quiet"*, *"shut up"*) immediately cancel active LLM streams, flush audio sinks, and dismiss operations.
- **Voice Memory Reset**: Verbal reset commands (*"clear memory"*, *"clear history"*, *"reset conversation"*) wipe the active conversational sliding window on demand.

### 🛠️ Advanced Desktop Tools & OS Bridge
Byte safely mediates system actions through strongly typed Rust tool handlers:

- **Dynamic App Launcher (`open_application`)**: Scans Windows Start Menu shortcuts (`.lnk`) across both All Users and AppData directories to launch any installed program.
- **Screen Vision (`analyze_screen`)**: Captures primary monitor screenshots via GDI+ and queries vision LLMs (Llama 3.2 Vision / Gemini Flash) to explain errors, code, or interfaces.
- **Active Window Controller (`window_control`)**: Minimizes, maximizes, or closes foreground windows using Win32 API calls (`ShowWindow`, `PostMessage`).
- **YouTube Autoplayer (`play_music`)**: Scrapes YouTube search results to locate matching video IDs and launches playback in your default browser.
- **Hardware Telemetry (`get_system_stats`)**: Reads live CPU load percentage, RAM usage, and uptime via Windows Management Instrumentation (WMI/CIM).
- **File Organizer (`organize_folder`)**: Safely sorts cluttered folders into categorized subdirectories (`Images`, `Documents`, `Media`, `Installers`, `Archives`) with confirmation gating.
- **Desktop Wallpaper Changer (`change_wallpaper`)**: Rotates wallpapers from local folders or Picsum with an integrated non-destructive pixel-blending watermark eraser.
- **System Control (`system_control`)**: Adjusts volume, mutes audio, controls media track playback, and changes monitor brightness via WMI.
- **Persistent Memory (`update_memory`)**: Stores user preferences, name, habits, and past conversation sliding history in `byte_memory.json`.
- **Utilities & Information**: Real-time timer (`set_timer`), clipboard manager (`clipboard_access`), system notifications (`show_notification`), Wikipedia reader (`wikipedia_search`), currency converter (`convert_currency`), and Google News reader (`read_news`).

---

## Tech Stack

- **Frontend**: HTML5, Vanilla CSS3 (Glassmorphic Floating Widget), Vanilla JavaScript 60+ FPS Canvas Engine (`ByteLivingOrb`), Tauri Event API.
- **Backend**: Rust (Tauri v2, Tokio async runtime, CPAL audio capture, Hound, Rodio, Image, Windows Win32 / WMI API).
- **Speech-to-Text**: Whisper.cpp (`whisper-cli.exe` + `ggml-tiny.en.bin`).
- **Text-to-Speech**: Piper TTS (`piper.exe` + `en_US-lessac-medium.onnx`).
- **LLM Engine**: Local Ollama (`granite4.2:3b` / `qwen3-4b`) / OpenRouter / Nvidia NIM Vision.

---

## Configuration & Setup

### Requirements
- **OS**: Windows 10 / 11 (64-bit)
- **Local STT / TTS**: 
  - Place `whisper-cli.exe` and `ggml-tiny.en.bin` in `%APPDATA%\Byte\models\` or set `BYTE_WHISPER_EXE` and `BYTE_WHISPER_MODEL`.
  - Place `piper.exe` and `en_US-lessac-medium.onnx` in `%APPDATA%\Byte\models\` or set `BYTE_PIPER_EXE` and `BYTE_PIPER_MODEL`.
- **LLM Engine**:
  - **Local**: Ollama running locally (`ollama run granite4.2:3b` or `ollama run qwen3-4b` on port `11434`).
  - **Cloud**: Set `OPENROUTER_API_KEY` or configure in `%APPDATA%\Byte\config\config.json`.

### Running the App

1. Run directly via Cargo:
   ```bash
   cd src-tauri
   cargo run
   ```

2. Or run via Tauri CLI:
   ```bash
   cargo tauri dev
   ```

3. Toggle Byte presence anytime using the global hotkey: **`Ctrl + B`** or click the floating orb.
