use std::io::Cursor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use hound::{WavSpec, WavWriter};
use serde::Serialize;

const WHISPER_SAMPLE_RATE: u32 = 16_000;
const MIN_RECORDING_SECONDS: f32 = 0.25;
const MIN_WHISPER_SECONDS: f32 = 0.8;
const RELEASE_CAPTURE_TAIL_MS: u64 = 160;
// -82 dBFS: retain very quiet headset whispers. Below this, the stream is effectively
// silent/disconnected for dictation purposes rather than merely a low-output microphone.
const SILENCE_RMS_THRESHOLD: f32 = 0.00008;
const CLIPPING_THRESHOLD: f32 = 0.98;

#[derive(Clone, Serialize, Debug)]
pub struct AudioDiagnostics {
    pub device_name: String,
    pub native_sample_rate: u32,
    pub channels: u16,
    pub recorded_duration_secs: f32,
    pub peak_amplitude: f32,
    pub rms_amplitude: f32,
    pub is_clipping: bool,
    pub is_recording: bool,
    pub message: Option<String>,
}

impl Default for AudioDiagnostics {
    fn default() -> Self {
        Self {
            device_name: "Detecting microphone…".into(),
            native_sample_rate: 0,
            channels: 0,
            recorded_duration_secs: 0.0,
            peak_amplitude: 0.0,
            rms_amplitude: 0.0,
            is_clipping: false,
            is_recording: false,
            message: None,
        }
    }
}

pub struct RecordedAudio {
    pub wav_bytes: Vec<u8>,
    pub diagnostics: AudioDiagnostics,
}

/// Captures the platform's default microphone and converts it to Whisper's 16 kHz mono WAV.
pub struct AudioRecorder {
    recording: Arc<AtomicBool>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    diagnostics: Arc<Mutex<AudioDiagnostics>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            diagnostics: Arc::new(Mutex::new(AudioDiagnostics::default())),
        }
    }

    pub fn diagnostics(&self) -> AudioDiagnostics {
        self.diagnostics.lock().unwrap().clone()
    }

    pub fn start_recording(&self) -> Result<(), String> {
        if self.recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            let message = microphone_help("No default input device was found");
            self.set_failure(&message);
            message
        })?;
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Default microphone".into());
        let supported_config = device.default_input_config().map_err(|e| {
            let message = format!("Could not read microphone configuration: {e}");
            self.set_failure(&message);
            message
        })?;
        let sample_format = supported_config.sample_format();
        let config: cpal::StreamConfig = supported_config.into();

        self.audio_data.lock().unwrap().clear();
        *self.diagnostics.lock().unwrap() = AudioDiagnostics {
            device_name,
            native_sample_rate: config.sample_rate.0,
            channels: config.channels,
            is_recording: true,
            ..AudioDiagnostics::default()
        };

        self.recording.store(true, Ordering::SeqCst);
        let recording = Arc::clone(&self.recording);
        let buffer = Arc::clone(&self.audio_data);
        let diagnostics = Arc::clone(&self.diagnostics);
        std::thread::spawn(move || {
            let stream = match build_stream(
                &device,
                &config,
                sample_format,
                recording.clone(),
                buffer,
                diagnostics.clone(),
            ) {
                Ok(stream) => stream,
                Err(error) => {
                    recording.store(false, Ordering::SeqCst);
                    diagnostics.lock().unwrap().message =
                        Some(format!("Could not open microphone stream: {error}"));
                    return;
                }
            };
            if let Err(error) = stream.play() {
                recording.store(false, Ordering::SeqCst);
                diagnostics.lock().unwrap().message =
                    Some(format!("Could not start microphone capture: {error}"));
                return;
            }
            while recording.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        });
        Ok(())
    }

    pub fn stop_recording_and_process(&self, gain_factor: f32) -> Result<RecordedAudio, String> {
        // Keep a tiny tail after the hotkey is released. WSLg/PulseAudio has buffering
        // latency, and ending at the exact key-up can cut off final consonants in short
        // words such as "hi".
        std::thread::sleep(std::time::Duration::from_millis(RELEASE_CAPTURE_TAIL_MS));
        self.recording.store(false, Ordering::SeqCst);
        // Let the capture thread receive the stop flag and flush its final callback.
        std::thread::sleep(std::time::Duration::from_millis(60));

        let raw_samples = self.audio_data.lock().unwrap().clone();
        let mut diagnostics = self.diagnostics();
        diagnostics.is_recording = false;

        if raw_samples.is_empty() {
            diagnostics.message = Some("No audio frames arrived from the microphone.".into());
            *self.diagnostics.lock().unwrap() = diagnostics.clone();
            return Err(
                microphone_help("No audio was captured"),
            );
        }
        if diagnostics.recorded_duration_secs < MIN_RECORDING_SECONDS {
            diagnostics.message = Some("Recording was too short to transcribe.".into());
            *self.diagnostics.lock().unwrap() = diagnostics.clone();
            return Err(
                "Recording was too short. Hold the hotkey and speak for at least one second."
                    .into(),
            );
        }
        if diagnostics.rms_amplitude < SILENCE_RMS_THRESHOLD {
            diagnostics.message = Some("The microphone captured near-silence.".into());
            *self.diagnostics.lock().unwrap() = diagnostics.clone();
            return Err(
                microphone_help("No voice was detected"),
            );
        }

        let mut whisper_samples = resample_to_16khz(&raw_samples, diagnostics.native_sample_rate);
        // Whisper's feature extractor is more stable with a short silent tail than a
        // sub-second buffer. Padding preserves the original speech while avoiding a
        // low-context decode for quick commands.
        let minimum_samples = (WHISPER_SAMPLE_RATE as f32 * MIN_WHISPER_SECONDS) as usize;
        if whisper_samples.len() < minimum_samples {
            whisper_samples.resize(minimum_samples, 0.0);
        }
        let raw_rms = diagnostics.rms_amplitude;
        let applied_gain = enhance_headset_voice(&mut whisper_samples, gain_factor);
        if diagnostics.is_clipping {
            diagnostics.message = Some(
                "Input clipped. Lower the microphone input level for clearer transcription."
                    .into(),
            );
        } else if raw_rms < 0.01 && applied_gain > 1.0 {
            diagnostics.message = Some(format!(
                "Quiet headset input was boosted {:.1}× for transcription.",
                applied_gain
            ));
        }
        *self.diagnostics.lock().unwrap() = diagnostics.clone();

        Ok(RecordedAudio {
            wav_bytes: encode_wav(&whisper_samples)?,
            diagnostics,
        })
    }

    /// Stops capture and discards the in-memory buffer without transcription or delivery.
    pub fn discard_recording(&self) {
        self.recording.store(false, Ordering::SeqCst);
        self.audio_data.lock().unwrap().clear();
        let mut diagnostics = self.diagnostics.lock().unwrap();
        diagnostics.is_recording = false;
        diagnostics.message = Some("Recording discarded.".into());
    }

    fn set_failure(&self, message: &str) {
        let mut diagnostics = self.diagnostics.lock().unwrap();
        diagnostics.is_recording = false;
        diagnostics.message = Some(message.to_string());
    }
}

fn microphone_help(problem: &str) -> String {
    if cfg!(target_os = "macos") {
        format!(
            "{problem}. Check System Settings → Privacy & Security → Microphone and allow Hams."
        )
    } else {
        format!("{problem}. Check the selected input device and its microphone level.")
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: SampleFormat,
    recording: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    diagnostics: Arc<Mutex<AudioDiagnostics>>,
) -> Result<Stream, cpal::BuildStreamError> {
    let channels = config.channels as usize;
    let err_diagnostics = Arc::clone(&diagnostics);
    let err_fn = move |err| {
        eprintln!("[Audio] Input stream error: {err}");
        err_diagnostics.lock().unwrap().message = Some(format!("Microphone stream error: {err}"));
    };
    match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _| {
                append_samples(
                    data,
                    channels,
                    &recording,
                    &buffer,
                    &diagnostics,
                    |sample| sample,
                )
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| {
                append_samples(
                    data,
                    channels,
                    &recording,
                    &buffer,
                    &diagnostics,
                    |sample| sample as f32 / i16::MAX as f32,
                )
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| {
                append_samples(
                    data,
                    channels,
                    &recording,
                    &buffer,
                    &diagnostics,
                    |sample| (sample as f32 - 32768.0) / 32768.0,
                )
            },
            err_fn,
            None,
        ),
        other => {
            eprintln!("[Audio] Unsupported microphone sample format: {other:?}");
            Err(cpal::BuildStreamError::StreamConfigNotSupported)
        }
    }
}

fn append_samples<T: Copy>(
    data: &[T],
    channels: usize,
    recording: &AtomicBool,
    buffer: &Mutex<Vec<f32>>,
    diagnostics: &Mutex<AudioDiagnostics>,
    to_f32: impl Fn(T) -> f32,
) {
    if !recording.load(Ordering::SeqCst) || channels == 0 {
        return;
    }
    let mut mono = Vec::with_capacity(data.len() / channels);
    for frame in data.chunks(channels) {
        let value = frame.iter().copied().map(&to_f32).sum::<f32>() / frame.len() as f32;
        mono.push(value.clamp(-1.0, 1.0));
    }
    if mono.is_empty() {
        return;
    }
    let peak = mono
        .iter()
        .fold(0.0_f32, |current, sample| current.max(sample.abs()));
    let sum_squares = mono.iter().map(|sample| sample * sample).sum::<f32>();
    let count = {
        let mut captured = buffer.lock().unwrap();
        captured.extend_from_slice(&mono);
        captured.len()
    };
    let mut info = diagnostics.lock().unwrap();
    let instant_rms = (sum_squares / mono.len() as f32).sqrt();
    info.peak_amplitude = info.peak_amplitude.max(peak);
    info.rms_amplitude = if info.rms_amplitude == 0.0 {
        instant_rms
    } else {
        info.rms_amplitude * 0.35 + instant_rms * 0.65
    };
    info.recorded_duration_secs = count as f32 / info.native_sample_rate.max(1) as f32;
    info.is_clipping |= peak >= CLIPPING_THRESHOLD;
}

fn resample_to_16khz(input: &[f32], input_rate: u32) -> Vec<f32> {
    if input_rate == WHISPER_SAMPLE_RATE || input.len() < 2 {
        return input.to_vec();
    }
    let output_len =
        ((input.len() as u64 * WHISPER_SAMPLE_RATE as u64) / input_rate.max(1) as u64) as usize;
    let ratio = input_rate as f32 / WHISPER_SAMPLE_RATE as f32;
    (0..output_len)
        .map(|index| {
            let position = index as f32 * ratio;
            let left = position.floor() as usize;
            let right = (left + 1).min(input.len() - 1);
            input[left] + (input[right] - input[left]) * (position - left as f32)
        })
        .collect()
}

/// Make quiet headset speech consistently audible to Whisper without allowing peaks to clip.
/// This is deliberately a light voice processor: it removes DC/low-frequency rumble, then
/// normalizes toward a speech-level RMS target with a peak safety ceiling.
fn enhance_headset_voice(samples: &mut [f32], gain_factor: f32) -> f32 {
    if samples.is_empty() {
        return 1.0;
    }

    let dc_offset = samples.iter().sum::<f32>() / samples.len() as f32;
    let mut previous_input = 0.0_f32;
    let mut previous_output = 0.0_f32;
    for sample in samples.iter_mut() {
        let input = *sample - dc_offset;
        // Gentle ~80 Hz high-pass filter to remove headset rumble and DC bias.
        let output = input - previous_input + 0.969 * previous_output;
        previous_input = input;
        previous_output = output;
        *sample = output;
    }

    let rms =
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt();
    let peak = samples
        .iter()
        .fold(0.0_f32, |current, sample| current.max(sample.abs()));
    // This is deliberately below the raw silence floor: a real whisper loses some
    // energy in the high-pass stage, but must still reach normalization.
    if rms <= 0.00002 || peak <= 0.00002 {
        return 1.0;
    }

    // Ultra-quiet speech needs substantially more headroom than normal speech. The
    // peak ceiling and soft limiter below remain authoritative, preventing this from
    // turning a loud input into a clipped one.
    let maximum_gain = (gain_factor.clamp(1.0, 8.0) * 320.0).min(1200.0);
    let requested_gain = (0.12 / rms).clamp(1.0, maximum_gain);
    let safe_gain = (0.92 / peak).min(requested_gain);
    for sample in samples {
        let boosted = *sample * safe_gain;
        // Soft limiting preserves loud consonants more naturally than a hard clip.
        *sample = (boosted / (1.0 + 0.12 * boosted.abs())).clamp(-0.95, 0.95);
    }
    safe_gain
}

fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: WHISPER_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(|e| format!("WAV writer initialization failed: {e}"))?;
        for sample in samples {
            writer
                .write_sample((sample * i16::MAX as f32) as i16)
                .map_err(|e| format!("WAV write failed: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("WAV finalization failed: {e}"))?;
    }
    Ok(cursor.into_inner())
}
