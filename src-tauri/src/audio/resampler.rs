use std::path::Path;

/// Resamples an f32 audio buffer from from_hz to to_hz using linear interpolation
pub fn resample(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || input.is_empty() {
        return input.to_vec();
    }
    let factor = to_hz as f64 / from_hz as f64;
    let new_len = (input.len() as f64 * factor) as usize;
    let mut output = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let idx = i as f64 / factor;
        let low = idx.floor() as usize;
        let high = idx.ceil() as usize;
        if high >= input.len() {
            output.push(input[low]);
            continue;
        }
        let diff = idx - low as f64;
        let val = input[low] * (1.0 - diff as f32) + input[high] * diff as f32;
        output.push(val);
    }
    output
}

/// Encodes raw f32 samples to mono 16-bit PCM WAV at 16000Hz
pub fn save_wav<P: AsRef<Path>>(path: P, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    if let Some(parent) = path.as_ref().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut writer = hound::WavWriter::create(path.as_ref(), spec)?;
    for &sample in samples {
        let scaled = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        writer.write_sample(scaled)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_identity() {
        let input = vec![0.1, 0.5, -0.2, 0.8];
        let output = resample(&input, 16000, 16000);
        assert_eq!(input, output);
    }

    #[test]
    fn test_resample_downsampling() {
        let input = vec![0.0; 48000];
        let output = resample(&input, 48000, 16000);
        assert_eq!(output.len(), 16000);
    }
}

