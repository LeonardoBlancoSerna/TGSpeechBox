/*
TGSpeechBox Rust port — DSP tuning constants, PRNG and filters.
Ported from src/dspCommon.h (MIT).
*/

use std::f64::consts::PI;

pub const PITWO: f64 = PI * 2.0;

pub const K_RESONATOR_NYQUIST_CLAMP_RATIO: f64 = 0.475;
pub const K_PARALLEL_PF6_NYQUIST_SPREAD_RATIO: f64 = 0.435;

pub const K_SWEEP_Q_MAX_F1: f64 = 10.0;
pub const K_SWEEP_Q_MAX_F2: f64 = 18.0;
pub const K_SWEEP_Q_MAX_F3: f64 = 20.0;
pub const K_SWEEP_BW_MIN_F1: f64 = 30.0;
pub const K_SWEEP_BW_MIN_F2: f64 = 40.0;
pub const K_SWEEP_BW_MIN_F3: f64 = 60.0;
pub const K_SWEEP_BW_MAX: f64 = 1000.0;

pub const K_RADIATION_DERIV_GAIN_BASE: f64 = 5.0;
pub const K_RADIATION_DERIV_GAIN_REF_SR: f64 = 22050.0;
pub const K_TURBULENCE_FLOW_POWER: f64 = 1.5;
pub const K_FRIC_NOISE_SCALE: f64 = 0.175;
pub const K_FRIC_SOFT_CLIP_K: f64 = 0.18;
pub const K_BYPASS_MIN_GAIN: f64 = 0.70;
pub const K_BYPASS_VOICED_DUCK: f64 = 0.20;
pub const K_VOICED_FRIC_DUCK: f64 = 0.18;
pub const K_VOICED_FRIC_DUCK_POWER: f64 = 1.0;
pub const K_VOICED_FRIC_MOD_DEPTH: f64 = 0.5;
pub const K_VOICED_FRIC_PRODUCT_THRESH: f64 = 0.35;
pub const K_CONSONANT_CLARITY_BOOST: f64 = 0.55;

pub const K_FRIC_BURST_FC_11K: f64 = 3500.0;
pub const K_FRIC_SUSTAIN_FC_11K: f64 = 3500.0;
pub const K_FRIC_BURST_FC_16K: f64 = 4500.0;
pub const K_FRIC_SUSTAIN_FC_16K: f64 = 6500.0;
pub const K_FRIC_BURST_FC_22K: f64 = 5000.0;
pub const K_FRIC_SUSTAIN_FC_22K: f64 = 8500.0;
pub const K_FRIC_BURST_FC_44K: f64 = 5000.0;
pub const K_FRIC_SUSTAIN_FC_44K: f64 = 8500.0;
pub const K_ASP_BURST_FC_11K: f64 = 2400.0;
pub const K_ASP_BURST_FC_16K: f64 = 3200.0;
pub const K_ASP_BURST_FC_22K: f64 = 3800.0;
pub const K_ASP_BURST_FC_44K: f64 = 4000.0;
pub const K_BURSTINESS_SCALE: f64 = 25.0;

pub const K_BREATHINESS_TILT_MAX_DB: f64 = 6.0;
pub const K_BREATHINESS_ASP_TILT_MAX_DB: f64 = -8.0;
pub const K_BREATHINESS_TILT_SMOOTH_MS: f64 = 8.0;

pub fn clamp_double(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

pub fn clamp_resonator_to_nyquist(freq_hz: &mut f64, bw_hz: &mut f64, sample_rate: i32) {
    let f_max = K_RESONATOR_NYQUIST_CLAMP_RATIO * sample_rate as f64;
    if *freq_hz > f_max {
        *bw_hz *= *freq_hz / f_max;
        *freq_hz = f_max;
    }
}

pub fn bandwidth_for_sweep(
    freq_hz: f64,
    base_bw_hz: f64,
    q_max: f64,
    bw_min_hz: f64,
    bw_max_hz: f64,
) -> f64 {
    if !freq_hz.is_finite() || !base_bw_hz.is_finite() || freq_hz <= 0.0 || base_bw_hz <= 0.0 {
        return base_bw_hz;
    }
    let mut bw = base_bw_hz;
    let bw_from_q = freq_hz / q_max;
    if bw_from_q > bw {
        bw = bw_from_q;
    }
    clamp_double(bw, bw_min_hz, bw_max_hz)
}

pub fn calculate_value_at_fade_position(old: f64, new: f64, ratio: f64) -> f64 {
    if new.is_nan() { old } else { old + (new - old) * ratio }
}

pub fn cosine_smooth(t: f64) -> f64 {
    0.5 * (1.0 - (PI * t).cos())
}

pub fn calculate_freq_at_fade_position(old: f64, new: f64, ratio: f64) -> f64 {
    if new.is_nan() { return old; }
    if old <= 0.0 || new <= 0.0 {
        return old + (new - old) * ratio;
    }
    let log_old = old.ln();
    let log_new = new.ln();
    (log_old + (log_new - log_old) * ratio).exp()
}

#[derive(Clone)]
pub struct FastRandom {
    seed: u32,
}

impl FastRandom {
    pub fn new(seed: u32) -> FastRandom {
        FastRandom { seed }
    }
    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }
    pub fn next_double(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 1) as f64 * (1.0 / 2147483648.0)
    }
    pub fn next_bipolar(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed as i32) as f64 * (1.0 / 2147483648.0)
    }
}

#[derive(Clone)]
pub struct NoiseGenerator {
    last_value: f64,
    rng: FastRandom,
}

impl NoiseGenerator {
    pub fn new() -> NoiseGenerator {
        NoiseGenerator { last_value: 0.0, rng: FastRandom::new(54321) }
    }
    pub fn reset(&mut self) {
        self.last_value = 0.0;
    }
    pub fn get_next(&mut self) -> f64 {
        self.last_value = (self.rng.next_double() - 0.5) + 0.75 * self.last_value;
        self.last_value
    }
    pub fn white(&mut self) -> f64 {
        self.rng.next_bipolar()
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        NoiseGenerator::new()
    }
}

pub struct FrequencyGenerator {
    sample_rate: i32,
    last_cycle_pos: f64,
}

impl FrequencyGenerator {
    pub fn new(sr: i32) -> FrequencyGenerator {
        FrequencyGenerator { sample_rate: sr, last_cycle_pos: 0.0 }
    }
    pub fn reset(&mut self) {
        self.last_cycle_pos = 0.0;
    }
    pub fn get_next(&mut self, frequency: f64) -> f64 {
        let pos = (frequency / self.sample_rate as f64 + self.last_cycle_pos) % 1.0;
        self.last_cycle_pos = pos;
        pos
    }
}

pub struct OnePoleLowpass {
    sample_rate: i32,
    alpha: f64,
    z: f64,
}

impl OnePoleLowpass {
    pub fn new(sr: i32) -> OnePoleLowpass {
        OnePoleLowpass { sample_rate: sr, alpha: 0.0, z: 0.0 }
    }
    pub fn set_cutoff_hz(&mut self, mut fc_hz: f64) {
        if fc_hz < 10.0 { fc_hz = 10.0; }
        let nyq = 0.5 * self.sample_rate as f64;
        if fc_hz > nyq * 0.95 { fc_hz = nyq * 0.95; }
        self.alpha = (-PITWO * fc_hz / self.sample_rate as f64).exp();
    }
    pub fn process(&mut self, x: f64) -> f64 {
        self.z = (1.0 - self.alpha) * x + self.alpha * self.z;
        self.z
    }
    pub fn reset(&mut self) {
        self.z = 0.0;
    }
}
