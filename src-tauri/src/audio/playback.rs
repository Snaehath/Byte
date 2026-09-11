use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::Source;

static CANCEL_PLAYBACK: AtomicBool = AtomicBool::new(false);
static ACTIVE_SINK: Mutex<Option<Arc<Sink>>> = Mutex::new(None);

pub fn reset_cancellation() {
    CANCEL_PLAYBACK.store(false, Ordering::SeqCst);
}

pub fn stop_audio() {
    CANCEL_PLAYBACK.store(true, Ordering::SeqCst);
    if let Ok(mut guard) = ACTIVE_SINK.lock() {
        if let Some(sink) = guard.take() {
            sink.stop();
        }
    }
}

pub fn play_audio_file<P: AsRef<Path>>(file_path: P) {
    let path_ref = file_path.as_ref();
    // Fresh playback turn: reset cancellation flag
    CANCEL_PLAYBACK.store(false, Ordering::SeqCst);

    log::info!("play_audio_file: opening audio file '{}'", path_ref.display());
    let file = match File::open(path_ref) {
        Ok(f) => f,
        Err(e) => {
            log::error!("play_audio_file: failed to open WAV file '{}': {}", path_ref.display(), e);
            return;
        }
    };

    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(res) => res,
        Err(e) => {
            log::error!("play_audio_file: failed to obtain default audio OutputStream: {}", e);
            return;
        }
    };

    let sink = match Sink::try_new(&stream_handle) {
        Ok(s) => s,
        Err(e) => {
            log::error!("play_audio_file: failed to create audio Sink: {}", e);
            return;
        }
    };

    let source = match Decoder::new(BufReader::new(file)) {
        Ok(src) => src,
        Err(e) => {
            log::error!("play_audio_file: failed to decode audio: {}", e);
            return;
        }
    };

    let sink = Arc::new(sink);
    if let Ok(mut guard) = ACTIVE_SINK.lock() {
        *guard = Some(Arc::clone(&sink));
    }

    log::info!("play_audio_file: starting audio playback...");
    sink.append(source);
    sink.sleep_until_end();

    if let Ok(mut guard) = ACTIVE_SINK.lock() {
        *guard = None;
    }
    log::info!("play_audio_file: audio playback finished.");
}

pub fn play_listening_chime() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let note1 = rodio::source::SineWave::new(523.25) // C5
                    .take_duration(std::time::Duration::from_millis(80))
                    .amplify(0.15);
                let note2 = rodio::source::SineWave::new(659.25) // E5
                    .take_duration(std::time::Duration::from_millis(80))
                    .amplify(0.15);
                let note3 = rodio::source::SineWave::new(783.99) // G5
                    .take_duration(std::time::Duration::from_millis(150))
                    .amplify(0.15);

                sink.append(note1);
                sink.append(note2);
                sink.append(note3);
                sink.sleep_until_end();
            }
        }
    });
}

pub fn play_processing_chime() {
    std::thread::spawn(|| {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let note1 = rodio::source::SineWave::new(783.99) // G5
                    .take_duration(std::time::Duration::from_millis(60))
                    .amplify(0.12);
                let note2 = rodio::source::SineWave::new(659.25) // E5
                    .take_duration(std::time::Duration::from_millis(100))
                    .amplify(0.12);

                sink.append(note1);
                sink.append(note2);
                sink.sleep_until_end();
            }
        }
    });
}
