// stt.rs
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Constants
const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Hybrid STT: uses local Whisper transcription
pub async fn hybrid_transcribe_audio(
    audio_samples: &[f32],
    whisper_ctx: &WhisperContext,
) -> Result<String, Box<dyn std::error::Error>> {
    transcribe_audio(audio_samples, whisper_ctx)
}

/// Initialize Whisper context once at startup
pub fn init_whisper_context() -> Result<WhisperContext, Box<dyn std::error::Error>> {
    suppress_whisper_output();

    // Try base model first (better accuracy), fallback to quantized model
    let model_paths = [
        "pkg/models/ggml-base.bin",        // Original (better accuracy)
        "pkg/models/ggml-base-q8_0.bin",  // Quantized (fallback)
    ];
    
    let model_path = model_paths.iter()
        .find(|p| Path::new(p).exists())
        .ok_or("Whisper model not found. Please run setup first.")?;

    println!("[STT] Loading Whisper model: {}", model_path);
    
    let params = WhisperContextParameters::default();
    let ctx = WhisperContext::new_with_params(model_path, params)?;
    
    // Pre-warm: create one state to allocate buffers upfront
    println!("[STT] Pre-warming Whisper state...");
    let _ = ctx.create_state();
    println!("[STT] Whisper ready");
    
    Ok(ctx)
}

/// Record audio, force convert to Mono, and RESAMPLE to 16kHz
pub   fn record_audio(duration_secs: u64) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device available")?;
    let config = device.default_input_config()?;
    
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    println!("ðŸŽ¤ Input Device Sample Rate: {} Hz (Resampling to 16000 Hz...)", sample_rate);

    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let audio_buffer_clone = audio_buffer.clone();

    let err_fn = |err| eprintln!("Error in audio stream: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                let mut buffer = audio_buffer_clone.lock().unwrap();
                if channels == 2 {
                    // Convert Stereo to Mono
                    for chunk in data.chunks(2) {
                        buffer.push((chunk[0] + chunk[1]) / 2.0);
                    }
                } else {
                    buffer.extend_from_slice(data);
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &_| {
                let mut buffer = audio_buffer_clone.lock().unwrap();
                if channels == 2 {
                    for chunk in data.chunks(2) {
                        let sample = (chunk[0] as f32 + chunk[1] as f32) / 2.0 / 32768.0;
                        buffer.push(sample);
                    }
                } else {
                    for &s in data {
                        buffer.push(s as f32 / 32768.0);
                    }
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _: &_| {
                let mut buffer = audio_buffer_clone.lock().unwrap();
                if channels == 2 {
                    for chunk in data.chunks(2) {
                        let s0 = chunk[0] as f32 / 65535.0 - 0.5;
                        let s1 = chunk[1] as f32 / 65535.0 - 0.5;
                        buffer.push((s0 + s1) / 2.0);
                    }
                } else {
                    for &s in data {
                        buffer.push(s as f32 / 65535.0 - 0.5);
                    }
                }
            },
            err_fn,
            None,
        )?,
        other => return Err(format!("Unsupported sample format: {:?}", other).into()),
    };

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(duration_secs));
    drop(stream);

    let recorded_data = audio_buffer.lock().unwrap().clone();

    // CRITICAL FIX: Resample to 16kHz before returning
    let resampled_data = resample_linear(&recorded_data, sample_rate, WHISPER_SAMPLE_RATE);

    Ok(resampled_data)
}

/// Helper function to save audio to a single WAV file (overwriting it)
// pub fn save_to_wav(path: &str, samples: &[f32]) -> std::io::Result<()> {
//     // 1. Create file (truncates existing file, so no duplicates)
//     let file = File::create(path)?;
//     let mut writer = BufWriter::new(file);

//     let sample_rate = 16000u32;
//     let num_channels = 1u16;
//     let bits_per_sample = 16u16;
//     let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
//     let block_align = num_channels * bits_per_sample / 8;
//     let data_len = samples.len() as u32 * 2; // 2 bytes per sample
//     let total_len = 36 + data_len;

//     // 2. Write WAV Header
//     writer.write_all(b"RIFF")?;
//     writer.write_all(&total_len.to_le_bytes())?;
//     writer.write_all(b"WAVE")?;
//     writer.write_all(b"fmt ")?;
//     writer.write_all(&16u32.to_le_bytes())?; // Chunk size
//     writer.write_all(&1u16.to_le_bytes())?;  // Audio format (1 = PCM)
//     writer.write_all(&num_channels.to_le_bytes())?;
//     writer.write_all(&sample_rate.to_le_bytes())?;
//     writer.write_all(&byte_rate.to_le_bytes())?;
//     writer.write_all(&block_align.to_le_bytes())?;
//     writer.write_all(&bits_per_sample.to_le_bytes())?;
//     writer.write_all(b"data")?;
//     writer.write_all(&data_len.to_le_bytes())?;

//     // 3. Write Data (Convert f32 -> i16)
//     for &sample in samples {
//         let s = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
//         writer.write_all(&s.to_le_bytes())?;
//     }
    
//     writer.flush()?;
//     Ok(())
// }

/// Simple linear interpolation to resample audio
fn resample_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz {
        return input.to_vec();
    }

    let ratio = from_hz as f32 / to_hz as f32;
    let new_len = (input.len() as f32 / ratio) as usize;
    let mut output = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let original_idx_float = i as f32 * ratio;
        let original_idx = original_idx_float as usize;

        if original_idx >= input.len() - 1 {
            break;
        }

        let fraction = original_idx_float - original_idx as f32;
        let a = input[original_idx];
        let b = input[original_idx + 1];

        // Linear interpolation
        output.push(a + (b - a) * fraction);
    }
    output
}

pub fn transcribe_audio(
    audio_samples: &[f32],
    ctx: &WhisperContext,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_language(Some("en"));
    
    // Use ALL available cores for maximum speed
    let thread_count = std::cmp::max(1, num_cpus::get()) as i32;
    params.set_n_threads(thread_count);
    
    params.set_suppress_non_speech_tokens(true);
    params.set_translate(false);
    
    // Aggressive speed optimizations
    params.set_no_context(true);      // Don't use previous context
    params.set_single_segment(true);  // Single segment for commands
    params.set_token_timestamps(false); // Skip token timestamps
    
    // Reduce beam search overhead
    params.set_suppress_blank(true);  // Skip blank tokens

    let mut state = ctx.create_state()?;
    state.full(params, audio_samples)?;

    let num_segments = state.full_n_segments()?;
    let mut transcription = String::new();

    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)?;
        transcription.push_str(&segment);
        transcription.push(' ');
    }

    Ok(transcription.trim().to_string())
}

#[cfg(target_os = "windows")]
fn suppress_whisper_output() {
    // On Windows, redirect stderr to null to suppress Whisper debug output
    use std::ptr;
    unsafe {
        let null_handle = winapi::um::fileapi::CreateFileA(
            b"NUL\0".as_ptr() as *const i8,
            winapi::um::winnt::GENERIC_WRITE,
            winapi::um::winnt::FILE_SHARE_READ | winapi::um::winnt::FILE_SHARE_WRITE,
            ptr::null_mut(),
            winapi::um::fileapi::OPEN_EXISTING,
            0,
            ptr::null_mut(),
        );
        if null_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
            winapi::um::processenv::SetStdHandle(winapi::um::winbase::STD_ERROR_HANDLE, null_handle);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn suppress_whisper_output() {
    unsafe {
        use std::fs::OpenOptions;
        use std::os::unix::io::AsRawFd;
        if let Ok(null) = OpenOptions::new().write(true).open("/dev/null") {
            libc::dup2(null.as_raw_fd(), 2);
        }
    }
}