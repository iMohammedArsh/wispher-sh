//! Offline processing of a finished recording: resample to 16 kHz, clean up the
//! voice, find where speech is, and trim the silence around it.

use rubato::{FftFixedIn, Resampler};

use super::{Captured, TARGET_RATE};

/// Full pipeline from raw mono samples at `rate` to Whisper-ready audio.
pub fn prepare(raw: &[f32], rate: u32) -> Captured {
    let duration_ms = if rate == 0 { 0 } else { raw.len() as u64 * 1000 / rate as u64 };
    let mut samples = resample(raw, rate, TARGET_RATE);
    remove_rumble(&mut samples);
    let Some((start, end)) = speech_bounds(&samples) else {
        return Captured { samples: Vec::new(), duration_ms, has_speech: false };
    };
    let mut speech = samples[start..end].to_vec();
    normalize(&mut speech);
    // whisper.cpp wants at least one second of audio.
    let min_len = TARGET_RATE as usize + TARGET_RATE as usize / 4;
    if speech.len() < min_len {
        speech.resize(min_len, 0.0);
    }
    Captured { samples: speech, duration_ms, has_speech: true }
}

/// High-quality FFT resampling. Falls back to a copy if rates match or input is tiny.
pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || input.len() < 64 {
        return input.to_vec();
    }
    let expected = (input.len() as u64 * to as u64 / from as u64) as usize;
    let Ok(mut resampler) = FftFixedIn::<f32>::new(from as usize, to as usize, 1024, 2, 1) else {
        return linear_resample(input, from, to);
    };
    let delay = resampler.output_delay();
    let mut out = Vec::with_capacity(expected + delay + 2048);
    let mut position = 0;
    loop {
        let needed = resampler.input_frames_next();
        if position + needed > input.len() {
            break;
        }
        match resampler.process(&[&input[position..position + needed]], None) {
            Ok(chunk) => out.extend_from_slice(&chunk[0]),
            Err(_) => return linear_resample(input, from, to),
        }
        position += needed;
    }
    if position < input.len() {
        if let Ok(chunk) = resampler.process_partial(Some(&[&input[position..]]), None) {
            out.extend_from_slice(&chunk[0]);
        }
    }
    // Flush the filter so the tail of the last word isn't cut.
    while out.len() < expected + delay {
        match resampler.process_partial(None::<&[&[f32]]>, None) {
            Ok(chunk) if !chunk[0].is_empty() => out.extend_from_slice(&chunk[0]),
            _ => break,
        }
    }
    out.drain(..delay.min(out.len()));
    out.truncate(expected);
    out
}

fn linear_resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    let expected = (input.len() as u64 * to as u64 / from as u64) as usize;
    let ratio = from as f64 / to as f64;
    (0..expected)
        .map(|i| {
            let position = i as f64 * ratio;
            let left = position.floor() as usize;
            let right = (left + 1).min(input.len() - 1);
            let t = (position - left as f64) as f32;
            input[left] + (input[right] - input[left]) * t
        })
        .collect()
}

/// Remove DC offset and low-frequency rumble (~80 Hz one-pole high-pass at 16 kHz).
/// Ported from Hams v1 (`enhance_headset_voice`).
pub fn remove_rumble(samples: &mut [f32]) {
    if samples.is_empty() {
        return;
    }
    let dc = samples.iter().sum::<f32>() / samples.len() as f32;
    let (mut previous_in, mut previous_out) = (0.0_f32, 0.0_f32);
    for sample in samples.iter_mut() {
        let input = *sample - dc;
        let output = input - previous_in + 0.969 * previous_out;
        previous_in = input;
        previous_out = output;
        *sample = output;
    }
}

const FRAME: usize = 480; // 30 ms at 16 kHz

fn frame_db(frame: &[f32]) -> f32 {
    let energy = frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32;
    10.0 * energy.max(1e-12).log10()
}

/// Energy-based voice activity detection. Returns the sample range that contains
/// speech (with padding), or None if there's no speech at all.
///
/// The threshold adapts to the recording's own noise floor, so a quiet headset
/// whisper still counts while a silent room does not.
pub fn speech_bounds(samples: &[f32]) -> Option<(usize, usize)> {
    let levels: Vec<f32> = samples.chunks(FRAME).map(frame_db).collect();
    if levels.is_empty() {
        return None;
    }
    let mut sorted = levels.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let noise_floor = sorted[sorted.len() / 10];
    let loudest = sorted[sorted.len() - 1];
    // Speech must stand out from the floor and not be digital silence.
    let threshold = (noise_floor + 9.0).max(-75.0).min(loudest - 3.0);
    if loudest < -75.0 {
        return None;
    }
    let voiced: Vec<bool> = levels.iter().map(|&db| db > threshold).collect();
    // Require ~150 ms of voiced frames in total, so clicks and bumps don't count.
    if voiced.iter().filter(|&&v| v).count() < 5 {
        return None;
    }
    let first = voiced.iter().position(|&v| v)?;
    let last = voiced.iter().rposition(|&v| v)?;
    let pad_before = 200 * TARGET_RATE as usize / 1000;
    let pad_after = 350 * TARGET_RATE as usize / 1000;
    let start = (first * FRAME).saturating_sub(pad_before);
    let end = ((last + 1) * FRAME + pad_after).min(samples.len());
    Some((start, end))
}

/// Bring speech to a consistent level without clipping. Gain is capped at 30 dB
/// (v1 allowed 1200x, which mostly amplified background noise).
pub fn normalize(samples: &mut [f32]) {
    if samples.is_empty() {
        return;
    }
    // Measure the RMS of the loudest half of frames: that's the speech, not the pauses.
    let mut frame_rms: Vec<f32> = samples
        .chunks(FRAME)
        .map(|f| (f.iter().map(|s| s * s).sum::<f32>() / f.len() as f32).sqrt())
        .collect();
    frame_rms.sort_by(|a, b| b.total_cmp(a));
    let speech = &frame_rms[..frame_rms.len().div_ceil(2)];
    let rms = speech.iter().sum::<f32>() / speech.len() as f32;
    let peak = samples.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
    if rms < 1e-6 || peak < 1e-6 {
        return;
    }
    let gain = (0.1 / rms).clamp(1.0, 31.6).min(0.95 / peak * 1.5);
    for sample in samples.iter_mut() {
        let boosted = *sample * gain;
        // Soft limiter keeps loud consonants natural instead of hard clipping.
        *sample = (boosted / (1.0 + 0.12 * boosted.abs())).clamp(-0.98, 0.98);
    }
}

/// Live meter value in 0..1 for one callback buffer, on a perceptual (dB) scale.
pub fn meter_level(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let db = frame_db(frame);
    ((db + 60.0) / 50.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(rate: u32, seconds: f32, freq: f32, amplitude: f32) -> Vec<f32> {
        (0..(rate as f32 * seconds) as usize)
            .map(|i| amplitude * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    #[test]
    fn resample_keeps_duration_and_frequency() {
        let input = tone(48_000, 2.0, 440.0, 0.5);
        let out = resample(&input, 48_000, 16_000);
        assert_eq!(out.len(), 32_000);
        // Count zero crossings: 440 Hz for 2 s ≈ 1760 crossings.
        let crossings = out.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
        assert!((1700..1820).contains(&crossings), "crossings = {crossings}");
    }

    #[test]
    fn resample_handles_odd_rates() {
        let input = tone(44_100, 1.0, 300.0, 0.5);
        assert_eq!(resample(&input, 44_100, 16_000).len(), 16_000);
    }

    #[test]
    fn silence_has_no_speech() {
        let silence = vec![0.0; 32_000];
        assert!(speech_bounds(&silence).is_none());
        let hiss: Vec<f32> = (0..32_000).map(|i| ((i * 7919) % 13) as f32 * 1e-5).collect();
        assert!(speech_bounds(&hiss).is_none());
        assert!(!prepare(&silence, 16_000).has_speech);
    }

    #[test]
    fn speech_is_found_and_trimmed() {
        let mut audio = vec![0.0005_f32; 16_000]; // 1 s quiet room
        audio.extend(tone(16_000, 1.0, 220.0, 0.2)); // 1 s "speech"
        audio.extend(vec![0.0005_f32; 32_000]); // 2 s quiet
        let (start, end) = speech_bounds(&audio).unwrap();
        assert!(start > 12_000 && start <= 16_000, "start = {start}");
        assert!(end >= 32_000 && end < 40_000, "end = {end}");
    }

    #[test]
    fn quiet_whisper_still_counts_as_speech() {
        let mut audio = vec![0.00002_f32; 16_000];
        audio.extend(tone(16_000, 0.8, 200.0, 0.004)); // about -51 dBFS
        audio.extend(vec![0.00002_f32; 16_000]);
        assert!(prepare(&audio, 16_000).has_speech);
    }

    #[test]
    fn normalize_boosts_quiet_audio_without_clipping() {
        let mut quiet = tone(16_000, 1.0, 200.0, 0.01);
        normalize(&mut quiet);
        let peak = quiet.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
        assert!(peak > 0.1 && peak < 0.99, "peak = {peak}");

        let mut loud = tone(16_000, 1.0, 200.0, 0.9);
        normalize(&mut loud);
        assert!(loud.iter().all(|s| s.abs() <= 0.98));
    }

    #[test]
    fn short_speech_is_padded_for_whisper() {
        let mut audio = vec![0.0_f32; 4_000];
        audio.extend(tone(16_000, 0.3, 220.0, 0.3));
        audio.extend(vec![0.0_f32; 4_000]);
        let captured = prepare(&audio, 16_000);
        assert!(captured.has_speech);
        assert!(captured.samples.len() >= 16_000);
    }
}
