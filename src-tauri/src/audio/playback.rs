use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::Source;

static CANCEL_PLAYBACK: AtomicBool = AtomicBool::new(false);

pub fn stop_audio() {
    CANCEL_PLAYBACK.store(true, Ordering::SeqCst);
}

pub fn play_audio_file<P: AsRef<Path>>(file_path: P) {
    let path_ref = file_path.as_ref();
    if let Ok(file) = File::open(path_ref) {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                if let Ok(source) = Decoder::new(BufReader::new(file)) {
                    CANCEL_PLAYBACK.store(false, Ordering::SeqCst);
                    sink.append(source);
                    while !sink.empty() {
                        if CANCEL_PLAYBACK.load(Ordering::SeqCst) {
                            sink.stop();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
        }
    }
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
