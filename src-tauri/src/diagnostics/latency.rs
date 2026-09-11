use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Default)]
pub struct PipelineTelemetry {
    pub vad_capture_ms: u128,
    pub stt_ms: u128,
    pub llm_ms: u128,
    pub tool_ms: u128,
    pub tts_synth_ms: u128,
    pub total_ms: u128,
}

impl PipelineTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn print_summary(&self, user_text: &str, tool_called: Option<&str>) {
        let tool_desc = match tool_called {
            Some(name) => format!("{} ({} ms)", name, self.tool_ms),
            None => "None (0 ms)".to_string(),
        };

        log::info!(
            "\n┌──────────────────────────────────────────────────┐\n\
             │             BYTE PIPELINE TELEMETRY              │\n\
             ├──────────────────────────────────────────────────┤\n\
             │ User Utterance: {:<32} │\n\
             │ Tool Invoked:   {:<32} │\n\
             ├──────────────────────────────────────────────────┤\n\
             │ ├─ Audio Capture & VAD: {:>6} ms                │\n\
             │ ├─ STT (Whisper):       {:>6} ms                │\n\
             │ ├─ LLM Inference:       {:>6} ms                │\n\
             │ ├─ Tool Execution:      {:>6} ms                │\n\
             │ ├─ TTS Synthesis:       {:>6} ms                │\n\
             │ └─ Total Latency:       {:>6} ms                │\n\
             └──────────────────────────────────────────────────┘",
            truncate_str(user_text, 32),
            truncate_str(&tool_desc, 32),
            self.vad_capture_ms,
            self.stt_ms,
            self.llm_ms,
            self.tool_ms,
            self.tts_synth_ms,
            self.total_ms
        );
    }
}

pub struct StageTimer {
    start: Instant,
}

impl StageTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() > max_len {
        let mut truncated: String = trimmed.chars().take(max_len - 3).collect();
        truncated.push_str("...");
        truncated
    } else {
        trimmed.to_string()
    }
}
