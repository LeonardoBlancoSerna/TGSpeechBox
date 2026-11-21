/*
TGSpeechBox Rust port — all-pole resonator and pitch-synchronous F1 resonator.
Ported from src/resonator.h (MIT).
*/

use crate::dsp_common::clamp_resonator_to_nyquist;

pub struct Resonator {
    sample_rate: i32,
    frequency: f64,
    bandwidth: f64,
    anti: bool,
    set_once: bool,
    y1: f64,
    y2: f64,
    df_b0: f64,
    df_fb1: f64,
    df_fb2: f64,
    fir_a: f64,
    fir_b: f64,
    fir_c: f64,
    z1: f64,
    z2: f64,
    disabled: bool,
}

impl Resonator {
    pub fn new(sample_rate: i32, anti: bool) -> Resonator {
        Resonator {
            sample_rate,
            frequency: 0.0,
            bandwidth: 0.0,
            anti,
            set_once: false,
            y1: 0.0,
            y2: 0.0,
            df_b0: 0.0,
            df_fb1: 0.0,
            df_fb2: 0.0,
            fir_a: 1.0,
            fir_b: 0.0,
            fir_c: 0.0,
            z1: 0.0,
            z2: 0.0,
            disabled: true,
        }
    }

    pub fn set_params(&mut self, mut frequency: f64, mut bandwidth: f64) {
        if !self.set_once || frequency != self.frequency || bandwidth != self.bandwidth {
            self.frequency = frequency;
            self.bandwidth = bandwidth;
            clamp_resonator_to_nyquist(&mut frequency, &mut bandwidth, self.sample_rate);
            let nyquist = 0.5 * self.sample_rate as f64;
            let invalid = !frequency.is_finite() || !bandwidth.is_finite();
            let off = frequency <= 0.0 || bandwidth <= 0.0 || frequency >= nyquist;
            if invalid || off {
                self.disabled = true;
                if self.anti {
                    self.fir_a = 1.0;
                    self.fir_b = 0.0;
                    self.fir_c = 0.0;
                } else {
                    self.df_b0 = 0.0;
                    self.df_fb1 = 0.0;
                    self.df_fb2 = 0.0;
                }
                self.set_once = true;
                return;
            }
            self.disabled = false;
            if self.anti {
                let r = (-std::f64::consts::PI / self.sample_rate as f64 * bandwidth).exp();
                let cos_theta = (2.0 * std::f64::consts::PI * frequency / self.sample_rate as f64).cos();
                let res_a = 1.0 - 2.0 * r * cos_theta + r * r;
                if !res_a.is_finite() || res_a.abs() < 1e-12 {
                    self.fir_a = 1.0;
                    self.fir_b = 0.0;
                    self.fir_c = 0.0;
                } else {
                    let inv_a = 1.0 / res_a;
                    self.fir_a = inv_a;
                    self.fir_b = -2.0 * r * cos_theta * inv_a;
                    self.fir_c = r * r * inv_a;
                }
            } else {
                let g = (std::f64::consts::PI * frequency / self.sample_rate as f64).tan();
                let g2 = g * g;
                let rr = (-2.0 * std::f64::consts::PI * bandwidth / self.sample_rate as f64).exp();
                let k = (1.0 - rr) * (1.0 + g2) / (g * (1.0 + rr));
                let d = 1.0 + k * g + g2;
                self.df_b0 = 4.0 * g2 / d;
                self.df_fb1 = 2.0 * (1.0 - g2) / d;
                self.df_fb2 = -(1.0 - k * g + g2) / d;
            }
        }
        self.set_once = true;
    }

    pub fn resonate(&mut self, input: f64, frequency: f64, bandwidth: f64, allow_update: bool) -> f64 {
        if allow_update {
            self.set_params(frequency, bandwidth);
        }
        if self.disabled {
            return input;
        }
        if self.anti {
            let out = self.fir_a * input + self.fir_b * self.z1 + self.fir_c * self.z2;
            self.z2 = self.z1;
            self.z1 = input;
            out
        } else {
            let out = self.df_b0 * input + self.df_fb1 * self.y1 + self.df_fb2 * self.y2;
            self.y2 = self.y1;
            self.y1 = out;
            out
        }
    }

    pub fn reset(&mut self) {
        self.y1 = 0.0;
        self.y2 = 0.0;
        self.z1 = 0.0;
        self.z2 = 0.0;
        self.set_once = false;
    }

    pub fn decay(&mut self, factor: f64) {
        self.y1 *= factor;
        self.y2 *= factor;
    }
}

pub struct PitchSyncResonator {
    sample_rate: i32,
    y1: f64,
    y2: f64,
    df_b0: f64,
    df_fb1: f64,
    df_fb2: f64,
    disabled: bool,
    set_once: bool,
    delta_freq: f64,
    delta_bw: f64,
    last_target_freq: f64,
    last_target_bw: f64,
    smooth_freq: f64,
    smooth_bw: f64,
    smooth_alpha: f64,
}

impl PitchSyncResonator {
    pub fn new(sr: i32) -> PitchSyncResonator {
        let smooth_ms = 2.0;
        PitchSyncResonator {
            sample_rate: sr,
            y1: 0.0,
            y2: 0.0,
            df_b0: 0.0,
            df_fb1: 0.0,
            df_fb2: 0.0,
            disabled: true,
            set_once: false,
            delta_freq: 0.0,
            delta_bw: 0.0,
            last_target_freq: 0.0,
            last_target_bw: 0.0,
            smooth_freq: 0.0,
            smooth_bw: 0.0,
            smooth_alpha: 1.0 - (-1.0 / (sr as f64 * smooth_ms * 0.001)).exp(),
        }
    }

    fn compute_coeffs(&mut self, mut freq: f64, mut bw: f64) {
        clamp_resonator_to_nyquist(&mut freq, &mut bw, self.sample_rate);
        let nyquist = 0.5 * self.sample_rate as f64;
        if !freq.is_finite() || !bw.is_finite() || freq <= 0.0 || bw <= 0.0 || freq >= nyquist {
            self.disabled = true;
            self.df_b0 = 0.0;
            self.df_fb1 = 0.0;
            self.df_fb2 = 0.0;
            return;
        }
        self.disabled = false;
        let g = (std::f64::consts::PI * freq / self.sample_rate as f64).tan();
        let g2 = g * g;
        let rr = (-2.0 * std::f64::consts::PI * bw / self.sample_rate as f64).exp();
        let k = (1.0 - rr) * (1.0 + g2) / (g * (1.0 + rr));
        let d = 1.0 + k * g + g2;
        self.df_b0 = 4.0 * g2 / d;
        self.df_fb1 = 2.0 * (1.0 - g2) / d;
        self.df_fb2 = -(1.0 - k * g + g2) / d;
    }

    pub fn reset(&mut self) {
        self.y1 = 0.0;
        self.y2 = 0.0;
        self.set_once = false;
        self.smooth_freq = 0.0;
        self.smooth_bw = 0.0;
    }

    pub fn decay(&mut self, factor: f64) {
        self.y1 *= factor;
        self.y2 *= factor;
    }

    pub fn set_pitch_sync_params(&mut self, d_f1: f64, d_b1: f64) {
        self.delta_freq = d_f1;
        self.delta_bw = d_b1;
    }

    pub fn resonate(&mut self, input: f64, freq: f64, bw: f64, glottis_open: bool) -> f64 {
        let (mut target_freq, mut target_bw) = (freq, bw);
        if self.delta_freq != 0.0 || self.delta_bw != 0.0 {
            if glottis_open {
                target_freq = freq + self.delta_freq;
                target_bw = bw + self.delta_bw;
            }
            if self.smooth_freq == 0.0 { self.smooth_freq = target_freq; }
            if self.smooth_bw == 0.0 { self.smooth_bw = target_bw; }
            self.smooth_freq += (target_freq - self.smooth_freq) * self.smooth_alpha;
            self.smooth_bw += (target_bw - self.smooth_bw) * self.smooth_alpha;
            target_freq = self.smooth_freq;
            target_bw = self.smooth_bw;
        }
        if !self.set_once || target_freq != self.last_target_freq || target_bw != self.last_target_bw {
            self.last_target_freq = target_freq;
            self.last_target_bw = target_bw;
            self.compute_coeffs(target_freq, target_bw);
            self.set_once = true;
        }
        if self.disabled {
            return input;
        }
        let out = self.df_b0 * input + self.df_fb1 * self.y1 + self.df_fb2 * self.y2;
        self.y2 = self.y1;
        self.y1 = out;
        out
    }
}
