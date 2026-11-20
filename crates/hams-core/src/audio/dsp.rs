//! Offline processing of a finished recording: resample to 16 kHz, clean up the
//! voice, find where speech is, and trim the silence around it.

use realfft::RealFftPlanner;
use rubato::{FftFixedIn, Resampler};

use super::{Captured, TARGET_RATE};

/// Speech quieter than this (dBFS, mean power of the voiced frames, measured in
/// the voice band) is treated as a whisper or a far-away voice.
const QUIET_DB: f32 = -42.0;
/// Normal recordings are boosted by at most 30 dB.
const MAX_GAIN: f32 = 31.6;
/// Quiet recordings may go up to 45 dB, as long as the room noise stays below
/// [`NOISE_CEILING_DB`] afterwards: boosting hiss makes Whisper invent words.
const MAX_GAIN_QUIET: f32 = 177.8;
const NOISE_CEILING_DB: f32 = -55.0;
/// A low-level recording (loudest frame under [`STEADY_NOISE_CEILING_DB`]) whose
/// loudest frame is within this many dB of its quietest tenth is steady noise
/// (a fan, a hum), not speech.
const MIN_SPREAD_DB: f32 = 3.0;
const STEADY_NOISE_CEILING_DB: f32 = -45.0;

/// Full pipeline from raw mono samples at `rate` to Whisper-ready audio.
///
/// Normal-level speech takes the plain path: trim, normalize. Speech that is
/// found to be very quiet (found by looking only at the 300 Hz to 5 kHz voice
/// band, where hum and rumble don't count) also gets room noise reduced and a
/// higher gain limit, so a whisper reaches Whisper at a usable level.
pub fn prepare(raw: &[f32], rate: u32) -> Captured {
    let duration_ms = if rate == 0 { 0 } else { raw.len() as u64 * 1000 / rate as u64 };
    let mut samples = resample(raw, rate, TARGET_RATE);
    remove_rumble(&mut samples);

    let band = band_pass(&samples);
    if let Some(vad) = analyze(&band, true).filter(|vad| vad.speech_db < QUIET_DB) {
        return prepare_quiet(&samples, &vad, duration_ms);
    }

    let Some((start, end)) = speech_bounds(&samples) else {
        return Captured { samples: Vec::new(), duration_ms, has_speech: false, quiet: false };
    };
    let mut speech = samples[start..end].to_vec();
    normalize(&mut speech);
    Captured { samples: pad_to_minimum(speech), duration_ms, has_speech: true, quiet: false }
}

/// whisper.cpp wants at least one second of audio.
fn pad_to_minimum(mut speech: Vec<f32>) -> Vec<f32> {
    let min_len = TARGET_RATE as usize + TARGET_RATE as usize / 4;
    if speech.len() < min_len {
        speech.resize(min_len, 0.0);
    }
    speech
}

/// The whisper path: reduce the room noise (profile taken from the parts of the
/// recording where nobody speaks), then lift the speech as far as the remaining
/// noise allows.
fn prepare_quiet(samples: &[f32], vad: &Vad, duration_ms: u64) -> Captured {
    let noise = noise_samples(samples, vad);
    let mut max_gain = MAX_GAIN;
    let mut cleaned = samples.to_vec();
    if let Some(profile) = noise_profile(&noise) {
        if let Some(gated) = spectral_gate(samples, &profile) {
            cleaned = gated;
            // How loud the room is after gating decides how much boost is safe.
            if let Some(gated_noise) = spectral_gate(&noise, &profile) {
                let power = gated_noise.iter().map(|s| s * s).sum::<f32>() / gated_noise.len().max(1) as f32;
                let noise_db = 10.0 * power.max(1e-12).log10();
                max_gain = 10f32.powf((NOISE_CEILING_DB - noise_db) / 20.0).clamp(MAX_GAIN, MAX_GAIN_QUIET);
            }
        }
    }
    let mut speech = cleaned[vad.start..vad.end].to_vec();
    normalize_with(&mut speech, max_gain);
    Captured { samples: pad_to_minimum(speech), duration_ms, has_speech: true, quiet: true }
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
    analyze(samples, false).map(|vad| (vad.start, vad.end))
}

/// Where the speech is, and how loud it is.
struct Vad {
    /// Sample range of the speech, padded.
    start: usize,
    end: usize,
    /// Per 30 ms frame: is it speech?
    voiced: Vec<bool>,
    /// Frame levels in dB, same indexing as `voiced`.
    levels: Vec<f32>,
    /// Mean power of the voiced frames, in dBFS.
    speech_db: f32,
}

/// The voice activity detector behind [`speech_bounds`]. With `adaptive` the
/// margin over the noise floor is 6 dB when speech clearly stands out (15 dB or
/// more) and 9 dB otherwise; without it, always 9 dB.
fn analyze(samples: &[f32], adaptive: bool) -> Option<Vad> {
    let levels: Vec<f32> = samples.chunks(FRAME).map(frame_db).collect();
    if levels.is_empty() {
        return None;
    }
    let mut sorted = levels.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let noise_floor = sorted[sorted.len() / 10];
    let loudest = sorted[sorted.len() - 1];
    if loudest < -75.0 {
        return None;
    }
    // Steady noise has no peaks; speech always does. Only judged at low levels,
    // so a normal-level recording is never rejected by this.
    let spread = loudest - noise_floor;
    if loudest < STEADY_NOISE_CEILING_DB && spread < MIN_SPREAD_DB {
        return None;
    }
    let margin = if adaptive && spread >= 15.0 { 6.0 } else { 9.0 };
    // Speech must stand out from the floor and not be digital silence.
    let threshold = (noise_floor + margin).max(-75.0).min(loudest - 3.0);
    let voiced: Vec<bool> = levels.iter().map(|&db| db > threshold).collect();
    // Require ~150 ms of voiced frames in total, so clicks and bumps don't count.
    let count = voiced.iter().filter(|&&v| v).count();
    if count < 5 {
        return None;
    }
    let first = voiced.iter().position(|&v| v)?;
    let last = voiced.iter().rposition(|&v| v)?;
    let pad_before = 200 * TARGET_RATE as usize / 1000;
    let pad_after = 350 * TARGET_RATE as usize / 1000;
    let start = (first * FRAME).saturating_sub(pad_before);
    let end = ((last + 1) * FRAME + pad_after).min(samples.len());
    let power: f32 = levels.iter().zip(&voiced).filter(|(_, &v)| v).map(|(&db, _)| 10f32.powf(db / 10.0)).sum();
    let speech_db = 10.0 * (power / count as f32).max(1e-12).log10();
    Some(Vad { start, end, voiced, levels, speech_db })
}

/// One second-order filter section (RBJ cookbook, Butterworth Q).
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    z: [f32; 2],
}

impl Biquad {
    fn new(low_pass: bool, cutoff: f32, rate: f32) -> Self {
        let w = 2.0 * std::f32::consts::PI * cutoff / rate;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let b = if low_pass {
            [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0]
        } else {
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0]
        };
        Self { b: b.map(|v| v / a0), a: [-2.0 * cos / a0, (1.0 - alpha) / a0], z: [0.0; 2] }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// A copy of the audio limited to the voice band (300 Hz to 5 kHz). Whispers
/// are all breath and consonants in that band; fans, hum and rumble are not.
fn band_pass(samples: &[f32]) -> Vec<f32> {
    let rate = TARGET_RATE as f32;
    let (mut high, mut low) = (Biquad::new(false, 300.0, rate), Biquad::new(true, 5_000.0, rate));
    samples.iter().map(|&s| low.process(high.process(s))).collect()
}

/// The parts of the recording where nobody is speaking, joined together.
/// Falls back to the quietest fifth of the recording if the speaker barely
/// paused.
fn noise_samples(samples: &[f32], vad: &Vad) -> Vec<f32> {
    let first = vad.voiced.iter().position(|&v| v).unwrap_or(0);
    let last = vad.voiced.iter().rposition(|&v| v).unwrap_or(0);
    let mut frames: Vec<usize> = (0..vad.voiced.len())
        // Keep clear of the speech: a word's onset and its decay aren't noise.
        .filter(|&i| !vad.voiced[i] && (i + 4 <= first || i >= last + 8))
        .collect();
    if frames.len() < 8 {
        let mut by_level: Vec<usize> = (0..vad.levels.len()).collect();
        by_level.sort_by(|&a, &b| vad.levels[a].total_cmp(&vad.levels[b]));
        frames = by_level.into_iter().take((vad.levels.len() / 5).max(4)).collect();
    }
    frames.iter().flat_map(|&i| samples[i * FRAME..((i + 1) * FRAME).min(samples.len())].iter().copied()).collect()
}

const GATE_N: usize = 512;
const GATE_HOP: usize = GATE_N / 2;
/// Reduction is capped at 12 dB: enough to quiet a room, too little to chew speech.
const GATE_FLOOR: f32 = 0.25;
/// How much louder than the average noise a bin must be to count as signal.
const GATE_OVERSUBTRACT: f32 = 1.3;

fn gate_window() -> Vec<f32> {
    // sqrt of a Hann window, used on the way in and out: at 50% overlap the two
    // together add up to exactly one.
    (0..GATE_N).map(|i| (std::f32::consts::PI * i as f32 / GATE_N as f32).sin()).collect()
}

/// Average spectrum (magnitude per bin) of the noise.
fn noise_profile(noise: &[f32]) -> Option<Vec<f32>> {
    if noise.len() < GATE_N {
        return None;
    }
    let fft = RealFftPlanner::<f32>::new().plan_fft_forward(GATE_N);
    let window = gate_window();
    let (mut input, mut spectrum) = (fft.make_input_vec(), fft.make_output_vec());
    let mut profile = vec![0.0_f32; spectrum.len()];
    let mut frames = 0;
    let mut position = 0;
    while position + GATE_N <= noise.len() {
        for i in 0..GATE_N {
            input[i] = noise[position + i] * window[i];
        }
        fft.process(&mut input, &mut spectrum).ok()?;
        for (sum, bin) in profile.iter_mut().zip(&spectrum) {
            *sum += bin.norm();
        }
        frames += 1;
        position += GATE_HOP;
    }
    profile.iter_mut().for_each(|v| *v /= frames as f32);
    Some(profile)
}

/// Gentle spectral noise gate: every bin is turned down by how much of it looks
/// like the noise profile, never by more than 12 dB, and it opens fast and
/// closes slowly so word onsets survive and there's no "musical noise".
/// (RNNoise-style suppressors are trained to keep *voiced* speech; they treat a
/// whisper as noise.)
fn spectral_gate(samples: &[f32], profile: &[f32]) -> Option<Vec<f32>> {
    let mut planner = RealFftPlanner::<f32>::new();
    let (forward, inverse) = (planner.plan_fft_forward(GATE_N), planner.plan_fft_inverse(GATE_N));
    let window = gate_window();
    // Silence in front and behind, so the first and last samples get a full overlap.
    let mut padded = vec![0.0_f32; GATE_N];
    padded.extend_from_slice(samples);
    padded.extend(std::iter::repeat_n(0.0, GATE_N * 2));
    let mut out = vec![0.0_f32; padded.len()];
    let (mut input, mut spectrum, mut time) = (forward.make_input_vec(), forward.make_output_vec(), inverse.make_output_vec());
    let bins = spectrum.len();
    let mut previous = vec![1.0_f32; bins];
    let mut magnitude = vec![0.0_f32; bins];
    let mut position = 0;
    while position + GATE_N <= padded.len() {
        for i in 0..GATE_N {
            input[i] = padded[position + i] * window[i];
        }
        forward.process(&mut input, &mut spectrum).ok()?;
        for (m, bin) in magnitude.iter_mut().zip(&spectrum) {
            *m = bin.norm();
        }
        for k in 0..bins {
            let heard = (magnitude[k.saturating_sub(1)] + magnitude[k] + magnitude[(k + 1).min(bins - 1)]) / 3.0;
            let target = if heard > 1e-9 {
                let ratio = profile[k] / heard;
                (1.0 - GATE_OVERSUBTRACT * ratio * ratio).max(0.0).sqrt().clamp(GATE_FLOOR, 1.0)
            } else {
                GATE_FLOOR
            };
            let gain = if target > previous[k] { target } else { 0.6 * previous[k] + 0.4 * target };
            previous[k] = gain;
            spectrum[k] *= gain;
        }
        inverse.process(&mut spectrum, &mut time).ok()?;
        for i in 0..GATE_N {
            out[position + i] += time[i] / GATE_N as f32 * window[i];
        }
        position += GATE_HOP;
    }
    Some(out[GATE_N..GATE_N + samples.len()].to_vec())
}

/// Bring speech to a consistent level without clipping. Gain is capped at 30 dB
/// (v1 allowed 1200x, which mostly amplified background noise).
pub fn normalize(samples: &mut [f32]) {
    normalize_with(samples, MAX_GAIN);
}

fn normalize_with(samples: &mut [f32], max_gain: f32) {
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
    let gain = (0.1 / rms).clamp(1.0, max_gain).min(0.95 / peak * 1.5);
    for sample in samples.iter_mut() {
        let boosted = *sample * gain;
        // Soft limiter keeps loud consonants natural instead of hard clipping.
        *sample = (boosted / (1.0 + 0.12 * boosted.abs())).clamp(-0.98, 0.98);
    }
}

/// The meter reads 0 at this level and below (dBFS)...
const METER_FLOOR_DB: f32 = -70.0;
/// ...and 1 at this level and above.
const METER_CEILING_DB: f32 = -10.0;

/// Live meter value in 0..1 for one callback buffer, on a perceptual (dB) scale.
/// The floor is low enough that a whisper (around -55 dBFS) visibly moves the
/// Mac pill and the phone's dot wave; normal speech (-30 to -15 dBFS) still sits
/// in the upper half. The meter shows what the microphone hears, before any gain.
pub fn meter_level(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let db = frame_db(frame);
    ((db - METER_FLOOR_DB) / (METER_CEILING_DB - METER_FLOOR_DB)).clamp(0.0, 1.0)
}

/// Notes mode: once `buffer` holds at least `min_seconds`, remove and return
/// its front, cut at the quietest moment of the last 3 seconds so no word is
/// split between chunks.
pub fn take_chunk(buffer: &mut Vec<f32>, rate: u32, min_seconds: u32) -> Option<Vec<f32>> {
    if buffer.len() < rate as usize * min_seconds as usize {
        return None;
    }
    let cut = quiet_split(buffer, rate, rate as usize * 3);
    Some(buffer.drain(..cut).collect())
}

/// Where to cut a long recording into chunks, searching the last `search`
/// samples: the quietest stretch of about 200 ms, cut at its quietest 10 ms.
/// Looking at a stretch rather than a single frame matters: the closure of a
/// "p" or "t" inside a word can be as silent as a pause, but it's far shorter.
pub fn quiet_split(samples: &[f32], rate: u32, search: usize) -> usize {
    const FRAME_MS: usize = 10;
    const STRETCH_FRAMES: usize = 20;
    let frame = (rate as usize * FRAME_MS / 1000).max(1);
    let start = samples.len().saturating_sub(search);
    let energies: Vec<f32> =
        samples[start..].chunks_exact(frame).map(|f| f.iter().map(|s| s * s).sum::<f32>()).collect();
    if energies.is_empty() {
        return samples.len();
    }
    let width = STRETCH_FRAMES.min(energies.len());
    let mut sum: f32 = energies[..width].iter().sum();
    let mut best = (sum, 0);
    for i in width..energies.len() {
        sum += energies[i] - energies[i - width];
        if sum < best.0 {
            best = (sum, i + 1 - width);
        }
    }
    let quietest = (best.1..best.1 + width)
        .min_by(|&a, &b| energies[a].total_cmp(&energies[b]))
        .unwrap_or(best.1);
    start + quietest * frame + frame / 2
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
        assert!((32_000..40_000).contains(&end), "end = {end}");
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

    // ---- Whispers and rooms ------------------------------------------------

    fn white(seed: &mut u32) -> f32 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 17;
        *seed ^= *seed << 5;
        *seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    fn rms_db(samples: &[f32]) -> f32 {
        10.0 * (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).max(1e-12).log10()
    }

    fn set_rms_db(samples: &mut [f32], dbfs: f32) {
        let gain = 10f32.powf((dbfs - rms_db(samples)) / 20.0);
        samples.iter_mut().for_each(|s| *s *= gain);
    }

    /// Steady room noise, a bit rumbly (one-pole low-passed white noise).
    fn room(seconds: f32, dbfs: f32) -> Vec<f32> {
        let mut seed = 0x1234_5678;
        let mut low = 0.0;
        let mut out: Vec<f32> = (0..(16_000.0 * seconds) as usize)
            .map(|_| {
                low += 0.15 * (white(&mut seed) - low);
                low
            })
            .collect();
        set_rms_db(&mut out, dbfs);
        out
    }

    /// Three breathy "syllables" (noise in the voice band, 250 ms each) at
    /// `dbfs` while they sound: nothing like a tone, a lot like a whisper.
    fn whisper(dbfs: f32) -> Vec<f32> {
        let mut seed = 0xCAFE_F00D;
        let mut out = Vec::new();
        for _ in 0..3 {
            let burst: Vec<f32> = (0..4_000).map(|_| white(&mut seed)).collect();
            out.extend(burst);
            out.extend(vec![0.0; 2_400]);
        }
        let mut shaped = band_pass(&out);
        let active: Vec<f32> = shaped.iter().copied().filter(|s| s.abs() > 1e-9).collect();
        let gain = 10f32.powf((dbfs - rms_db(&active)) / 20.0);
        shaped.iter_mut().for_each(|s| *s *= gain);
        shaped
    }

    /// `lead` s of room, the whisper, `lead` s of room, all over the room noise.
    fn recording(lead: f32, speech: &[f32], room_dbfs: f32) -> Vec<f32> {
        let lead_samples = (16_000.0 * lead) as usize;
        let mut audio = vec![0.0; lead_samples];
        audio.extend_from_slice(speech);
        audio.extend(vec![0.0; lead_samples]);
        let noise = room(audio.len() as f32 / 16_000.0, room_dbfs);
        for (sample, noise) in audio.iter_mut().zip(noise) {
            *sample += noise;
        }
        audio
    }

    #[test]
    fn the_meter_moves_for_a_whisper_and_keeps_room_for_loud_speech() {
        let at = |dbfs: f32| {
            let mut tone = vec![0.0; 800];
            for (i, s) in tone.iter_mut().enumerate() {
                *s = (i as f32 * 0.3).sin();
            }
            set_rms_db(&mut tone, dbfs);
            meter_level(&tone)
        };
        assert_eq!(meter_level(&[]), 0.0);
        assert_eq!(at(-90.0), 0.0, "a silent room reads zero");
        assert!(at(-55.0) > 0.2, "a whisper at -55 dBFS reads {}", at(-55.0));
        assert!(at(-65.0) < at(-55.0), "the meter keeps ordering quiet sounds");
        assert!(at(-20.0) > at(-30.0) && at(-30.0) > at(-55.0), "louder reads higher");
        assert!(at(-20.0) < 1.0, "normal speech leaves headroom");
        assert_eq!(at(-5.0), 1.0, "and it still tops out");
    }

    #[test]
    fn a_whisper_over_room_noise_is_speech() {
        let audio = recording(1.0, &whisper(-60.0), -70.0);
        let captured = prepare(&audio, 16_000);
        assert!(captured.has_speech);
        assert!(captured.quiet);
    }

    #[test]
    fn steady_room_noise_alone_is_not_speech() {
        for level in [-80.0, -65.0, -55.0] {
            assert!(!prepare(&room(3.0, level), 16_000).has_speech, "room noise at {level} dBFS");
        }
    }

    #[test]
    fn normal_speech_takes_the_plain_path() {
        let mut speech = whisper(-20.0);
        speech.extend(whisper(-18.0));
        let audio = recording(0.6, &speech, -60.0);
        let captured = prepare(&audio, 16_000);
        assert!(captured.has_speech && !captured.quiet);

        // Exactly what the pipeline produced before quiet handling existed.
        let mut plain = audio.clone();
        remove_rumble(&mut plain);
        let (start, end) = speech_bounds(&plain).unwrap();
        let mut expected = plain[start..end].to_vec();
        normalize(&mut expected);
        assert_eq!(captured.samples, expected);
    }

    #[test]
    fn quiet_speech_is_lifted_to_a_usable_level() {
        let audio = recording(1.0, &whisper(-52.0), -80.0);
        let captured = prepare(&audio, 16_000);
        assert!(captured.quiet && captured.has_speech);

        let mut loud: Vec<f32> = captured.samples.chunks(FRAME).map(rms_db).collect();
        loud.sort_by(|a, b| b.total_cmp(a));
        let speech = loud[..loud.len() / 2].iter().sum::<f32>() / (loud.len() / 2) as f32;
        assert!((-24.0..=-16.0).contains(&speech), "speech at {speech:.1} dBFS");
        let floor = {
            let mut quiet_first = loud.clone();
            quiet_first.reverse();
            quiet_first[quiet_first.len() / 10]
        };
        assert!(floor < -55.0, "room noise left at {floor:.1} dBFS");
    }

    #[test]
    fn the_gate_quiets_the_room_and_keeps_the_speech() {
        let burst = whisper(-45.0);
        let audio = recording(1.0, &burst, -60.0);
        let profile = noise_profile(&audio[..12_000]).unwrap();
        let gated = spectral_gate(&audio, &profile).unwrap();
        assert_eq!(gated.len(), audio.len());

        let room_before = rms_db(&audio[2_000..12_000]);
        let room_after = rms_db(&gated[2_000..12_000]);
        assert!(room_before - room_after > 6.0, "room {room_before:.1} → {room_after:.1} dBFS");
        let speech = 16_000..16_000 + burst.len();
        let lost = rms_db(&audio[speech.clone()]) - rms_db(&gated[speech]);
        assert!(lost < 3.0, "the gate took {lost:.1} dB off the speech");
    }

    #[test]
    fn quiet_split_finds_the_pause() {
        let rate = 1_000;
        // 2 s of "speech", a 100 ms pause at 2.40-2.50 s, then more speech.
        let mut samples: Vec<f32> = (0..3_000).map(|i| ((i as f32) * 0.3).sin() * 0.5).collect();
        for s in &mut samples[2_400..2_500] {
            *s = 0.0;
        }
        let cut = quiet_split(&samples, rate, 1_000);
        assert!((2_400..2_500).contains(&cut), "cut at {cut}");
        assert_eq!(quiet_split(&[], rate, 1_000), 0);
    }

    #[test]
    fn quiet_split_prefers_a_pause_over_a_silent_consonant() {
        let rate = 16_000;
        let speech = |n: usize| -> Vec<f32> { (0..n).map(|i| ((i as f32) * 0.2).sin() * 0.4).collect() };
        // Speech, a 40 ms dead-silent stop consonant, speech, a 250 ms pause
        // with a little room noise, then speech.
        let mut samples = speech(16_000);
        samples.extend(std::iter::repeat_n(0.0, 640));
        samples.extend(speech(8_000));
        let pause_start = samples.len();
        samples.extend((0..4_000).map(|i| ((i as f32) * 1.3).sin() * 0.01));
        samples.extend(speech(8_000));
        let cut = quiet_split(&samples, rate, samples.len());
        assert!((pause_start..pause_start + 4_000).contains(&cut), "cut at {cut}, pause at {pause_start}");
    }
}
