use std::sync::atomic::{AtomicUsize, Ordering};

pub struct AdaptiveVad {
    samples_collected: usize,
    rms_sum: f32,
    count: usize,
    pub threshold: f32,
    silence_counter: AtomicUsize,
}

impl AdaptiveVad {
    pub fn new() -> Self {
        Self {
            samples_collected: 0,
            rms_sum: 0.0,
            count: 0,
            threshold: 0.015,
            silence_counter: AtomicUsize::new(0),
        }
    }

    /// Process a new frame of audio samples and return whether silence condition is met
    pub fn process_frame(
        &mut self,
        data: &[f32],
        sample_rate: u32,
        channels: u16,
        total_recorded_samples: usize,
        silence_limit_ms: u64,
        min_recording_ms: u64,
    ) -> bool {
        if data.is_empty() {
            return false;
        }

        // Calculate Root Mean Square (RMS) energy
        let sum_sq: f32 = data.iter().map(|&x| x * x).sum();
        let rms = (sum_sq / data.len() as f32).sqrt();

        // Initial 200ms adaptive calibration
        let limit_calibration_samples = (sample_rate as f32 * 0.2) as usize;
        let frame_samples = data.len() / (channels as usize).max(1);
        if self.samples_collected < limit_calibration_samples {
            self.samples_collected += frame_samples;
            self.rms_sum += rms;
            self.count += 1;
            if self.samples_collected >= limit_calibration_samples && self.count > 0 {
                let avg_rms = self.rms_sum / self.count as f32;
                self.threshold = (avg_rms * 1.5).clamp(0.01, 0.04);
                log::info!("VAD calibrated adaptive silence threshold: {:.4}", self.threshold);
            }
        }

        if rms < self.threshold {
            let current_silence = self.silence_counter.fetch_add(frame_samples, Ordering::SeqCst) + frame_samples;
            let silence_limit_samples = ((sample_rate as f32) * (silence_limit_ms as f32 / 1000.0)) as usize;
            let min_recorded_samples = ((sample_rate as f32) * (min_recording_ms as f32 / 1000.0)) as usize;

            if current_silence > silence_limit_samples && total_recorded_samples > min_recorded_samples {
                return true; // Silence triggered
            }
        } else {
            // Sound detected above noise threshold
            self.silence_counter.store(0, Ordering::SeqCst);
        }

        false
    }

    pub fn reset(&mut self) {
        self.samples_collected = 0;
        self.rms_sum = 0.0;
        self.count = 0;
        self.threshold = 0.015;
        self.silence_counter.store(0, Ordering::SeqCst);
    }
}
