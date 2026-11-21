/*
TGSpeechBox Rust port — cascade and parallel formant filter banks.
Ported from src/formantGenerator.h (MIT).
*/

use tgsb_frame::{Frame, FrameEx};

use crate::dsp_common::{
    bandwidth_for_sweep,
    K_PARALLEL_PF6_NYQUIST_SPREAD_RATIO, K_RESONATOR_NYQUIST_CLAMP_RATIO,
    K_SWEEP_BW_MAX, K_SWEEP_BW_MIN_F1, K_SWEEP_BW_MIN_F2, K_SWEEP_BW_MIN_F3,
    K_SWEEP_Q_MAX_F1, K_SWEEP_Q_MAX_F2, K_SWEEP_Q_MAX_F3,
};
use crate::resonator::{PitchSyncResonator, Resonator};
use crate::utils::calculate_value_at_fade_position;

pub struct CascadeFormantGenerator {
    sample_rate: i32,
    r1: PitchSyncResonator,
    r2: Resonator,
    r3: Resonator,
    r4: Resonator,
    r5: Resonator,
    r6: Resonator,
    r7: Resonator,
    r8: Resonator,
    r_n0: Resonator,
    r_np: Resonator,
    pitch_sync_f1_delta: f64,
    pitch_sync_b1_delta: f64,
    bw_scale: f64,
    f1_bw_offset: f64,
    nasal_bw_scale: f64,
    f4_freq_scale: f64,
    nasal_gain_scale: f64,
}

impl CascadeFormantGenerator {
    pub fn new(sr: i32) -> CascadeFormantGenerator {
        CascadeFormantGenerator {
            sample_rate: sr,
            r1: PitchSyncResonator::new(sr),
            r2: Resonator::new(sr, false),
            r3: Resonator::new(sr, false),
            r4: Resonator::new(sr, false),
            r5: Resonator::new(sr, false),
            r6: Resonator::new(sr, false),
            r7: Resonator::new(sr, false),
            r8: Resonator::new(sr, false),
            r_n0: Resonator::new(sr, true),
            r_np: Resonator::new(sr, false),
            pitch_sync_f1_delta: 0.0,
            pitch_sync_b1_delta: 0.0,
            bw_scale: 1.0,
            f1_bw_offset: 0.0,
            nasal_bw_scale: 1.0,
            f4_freq_scale: 1.0,
            nasal_gain_scale: 1.0,
        }
    }
    pub fn reset(&mut self) {
        self.r1.reset(); self.r2.reset(); self.r3.reset(); self.r4.reset();
        self.r5.reset(); self.r6.reset(); self.r7.reset(); self.r8.reset();
        self.r_n0.reset(); self.r_np.reset();
    }
    pub fn decay(&mut self, factor: f64) {
        self.r1.decay(factor); self.r2.decay(factor); self.r3.decay(factor);
        self.r4.decay(factor); self.r5.decay(factor); self.r6.decay(factor);
        self.r7.decay(factor); self.r8.decay(factor);
        self.r_n0.decay(factor); self.r_np.decay(factor);
    }
    pub fn set_pitch_sync_params(&mut self, f1: f64, b1: f64) {
        self.pitch_sync_f1_delta = f1;
        self.pitch_sync_b1_delta = b1;
        self.r1.set_pitch_sync_params(f1, b1);
    }
    pub fn set_cascade_bw_scale(&mut self, mut scale: f64) {
        if scale < 0.3 { scale = 0.3; }
        if scale > 2.0 { scale = 2.0; }
        self.bw_scale = scale;
    }
    pub fn set_nasal_bw_scale(&mut self, mut scale: f64) {
        if scale < 0.5 { scale = 0.5; }
        if scale > 2.0 { scale = 2.0; }
        self.nasal_bw_scale = scale;
    }
    pub fn set_f4_freq_scale(&mut self, mut scale: f64) {
        if scale < 0.7 { scale = 0.7; }
        if scale > 1.5 { scale = 1.5; }
        self.f4_freq_scale = scale;
    }
    pub fn set_nasal_gain_scale(&mut self, mut scale: f64) {
        if scale < 0.5 { scale = 0.5; }
        if scale > 1.5 { scale = 1.5; }
        self.nasal_gain_scale = scale;
    }
    pub fn set_f1_bw_offset(&mut self, offset: f64) {
        self.f1_bw_offset = offset;
    }

    pub fn get_next(
        &mut self,
        frame: &Frame,
        frame_ex: Option<&FrameEx>,
        glottis_open: bool,
        input: f64,
    ) -> f64 {
        let input = input / 2.0;
        let n0_output = self.r_n0.resonate(
            input,
            frame.cf_n0,
            frame.cb_n0 * self.nasal_bw_scale,
            true,
        );
        let mut scaled_ca_np = frame.ca_np * self.nasal_gain_scale;
        if scaled_ca_np > 1.0 { scaled_ca_np = 1.0; }
        let mut output = calculate_value_at_fade_position(
            input,
            self.r_np.resonate(
                n0_output,
                frame.cf_np,
                frame.cb_np * self.nasal_bw_scale,
                true,
            ),
            scaled_ca_np,
        );

        let mut cb1 = frame.cb1;
        let mut cb2 = frame.cb2;
        let mut cb3 = frame.cb3;
        if let Some(ex) = frame_ex {
            if ex.end_cf1.is_finite() {
                cb1 = bandwidth_for_sweep(frame.cf1, cb1, K_SWEEP_Q_MAX_F1, K_SWEEP_BW_MIN_F1, K_SWEEP_BW_MAX);
            }
            if ex.end_cf2.is_finite() {
                cb2 = bandwidth_for_sweep(frame.cf2, cb2, K_SWEEP_Q_MAX_F2, K_SWEEP_BW_MIN_F2, K_SWEEP_BW_MAX);
            }
            if ex.end_cf3.is_finite() {
                cb3 = bandwidth_for_sweep(frame.cf3, cb3, K_SWEEP_Q_MAX_F3, K_SWEEP_BW_MIN_F3, K_SWEEP_BW_MAX);
            }
        }
        cb1 += self.f1_bw_offset;

        let cascade_bw_scale = self.bw_scale;
        let f1_bw_scale = cascade_bw_scale * 0.75;
        let f2_bw_scale = cascade_bw_scale * 0.88;
        cb1 *= f1_bw_scale;
        cb2 *= f2_bw_scale;
        cb3 *= cascade_bw_scale;
        let cb4 = frame.cb4 * cascade_bw_scale;
        let cb5 = frame.cb5 * cascade_bw_scale;
        let cb6 = frame.cb6 * cascade_bw_scale;

        let nyquist = 0.5 * self.sample_rate as f64;
        let cascade_fade = |cf: f64| -> f64 {
            if cf <= 0.0 || !cf.is_finite() { return 1.0; }
            let ratio = cf / nyquist;
            if ratio < 0.65 { 1.0 } else if ratio > 0.85 { 0.0 } else { 1.0 - (ratio - 0.65) / 0.20 }
        };
        let f4s = self.f4_freq_scale;
        let hs1 = if f4s != 1.0 { f4s.powf(0.2) } else { 1.0 };
        let hs2 = if f4s != 1.0 { f4s.powf(0.5) } else { 1.0 };
        let hs3 = if f4s != 1.0 { f4s.powf(0.8) } else { 1.0 };

        {
            let (default_cf7, default_cb7) = (6500.0, 720.0);
            let (default_cf8, default_cb8) = (7500.0, 1250.0);
            let mut fcf8 = frame_ex.map_or(default_cf8, |e| e.cf8);
            let mut fcb8 = frame_ex.map_or(default_cb8, |e| e.cb8);
            let mut fcf7 = frame_ex.map_or(default_cf7, |e| e.cf7);
            let mut fcb7 = frame_ex.map_or(default_cb7, |e| e.cb7);
            if !fcf8.is_finite() || fcf8 <= 0.0 { fcf8 = default_cf8; }
            if !fcb8.is_finite() || fcb8 <= 0.0 { fcb8 = default_cb8; }
            if !fcf7.is_finite() || fcf7 <= 0.0 { fcf7 = default_cf7; }
            if !fcb7.is_finite() || fcb7 <= 0.0 { fcb7 = default_cb7; }
            fcf8 *= f4s;
            fcf7 *= f4s;
            let pre_r8 = output;
            output = self.r8.resonate(output, fcf8, fcb8 * cascade_bw_scale, true);
            let fade8 = cascade_fade(fcf8);
            output = pre_r8 + fade8 * (output - pre_r8);
            let pre_r7 = output;
            output = self.r7.resonate(output, fcf7, fcb7 * cascade_bw_scale, true);
            let fade7 = cascade_fade(fcf7);
            output = pre_r7 + fade7 * (output - pre_r7);
        }

        let pre_r6 = output;
        output = self.r6.resonate(output, frame.cf6 * f4s, cb6, true);
        let fade6 = cascade_fade(frame.cf6 * f4s);
        output = pre_r6 + fade6 * (output - pre_r6);
        let pre_r5 = output;
        output = self.r5.resonate(output, frame.cf5 * f4s, cb5, true);
        let fade5 = cascade_fade(frame.cf5 * f4s);
        output = pre_r5 + fade5 * (output - pre_r5);
        let pre_r4 = output;
        output = self.r4.resonate(output, frame.cf4 * f4s, cb4, true);
        let fade4 = cascade_fade(frame.cf4 * f4s);
        output = pre_r4 + fade4 * (output - pre_r4);
        output = self.r3.resonate(output, frame.cf3 * hs3, cb3, true);
        output = self.r2.resonate(output, frame.cf2 * hs2, cb2, true);
        output = self.r1.resonate(output, frame.cf1 * hs1, cb1, glottis_open);
        output
    }
}

pub struct ParallelFormantGenerator {
    sample_rate: i32,
    r1: Resonator,
    r2: Resonator,
    r3: Resonator,
    r4: Resonator,
    r5: Resonator,
    r6: Resonator,
    r7: Resonator,
    r8: Resonator,
    f4_freq_scale: f64,
}

impl ParallelFormantGenerator {
    pub fn new(sr: i32) -> ParallelFormantGenerator {
        ParallelFormantGenerator {
            sample_rate: sr,
            r1: Resonator::new(sr, false),
            r2: Resonator::new(sr, false),
            r3: Resonator::new(sr, false),
            r4: Resonator::new(sr, false),
            r5: Resonator::new(sr, false),
            r6: Resonator::new(sr, false),
            r7: Resonator::new(sr, false),
            r8: Resonator::new(sr, false),
            f4_freq_scale: 1.0,
        }
    }
    pub fn set_f4_freq_scale(&mut self, mut scale: f64) {
        if scale < 0.7 { scale = 0.7; }
        if scale > 1.5 { scale = 1.5; }
        self.f4_freq_scale = scale;
    }
    pub fn reset(&mut self) {
        self.r1.reset(); self.r2.reset(); self.r3.reset(); self.r4.reset();
        self.r5.reset(); self.r6.reset(); self.r7.reset(); self.r8.reset();
    }
    pub fn decay(&mut self, factor: f64) {
        self.r1.decay(factor); self.r2.decay(factor); self.r3.decay(factor);
        self.r4.decay(factor); self.r5.decay(factor); self.r6.decay(factor);
        self.r7.decay(factor); self.r8.decay(factor);
    }

    pub fn get_next(
        &mut self,
        frame: &Frame,
        frame_ex: Option<&FrameEx>,
        _glottis_open: bool,
        input: f64,
    ) -> f64 {
        let input = input / 2.0;
        let mut output = 0.0;
        let mut pb1 = frame.pb1;
        let mut pb2 = frame.pb2;
        let mut pb3 = frame.pb3;
        if let Some(ex) = frame_ex {
            if ex.end_pf1.is_finite() {
                pb1 = bandwidth_for_sweep(frame.pf1, pb1, K_SWEEP_Q_MAX_F1, K_SWEEP_BW_MIN_F1, K_SWEEP_BW_MAX);
            }
            if ex.end_pf2.is_finite() {
                pb2 = bandwidth_for_sweep(frame.pf2, pb2, K_SWEEP_Q_MAX_F2, K_SWEEP_BW_MIN_F2, K_SWEEP_BW_MAX);
            }
            if ex.end_pf3.is_finite() {
                pb3 = bandwidth_for_sweep(frame.pf3, pb3, K_SWEEP_Q_MAX_F3, K_SWEEP_BW_MIN_F3, K_SWEEP_BW_MAX);
            }
        }
        let f4s = self.f4_freq_scale;
        let hs1 = if f4s != 1.0 { f4s.powf(0.2) } else { 1.0 };
        let hs2 = if f4s != 1.0 { f4s.powf(0.5) } else { 1.0 };
        let hs3 = if f4s != 1.0 { f4s.powf(0.8) } else { 1.0 };

        let mut tilt = frame_ex.map_or(0.0, |e| e.frication_tilt_db);
        if !tilt.is_finite() {
            tilt = 0.0;
        } else if tilt < -24.0 {
            tilt = -24.0;
        } else if tilt > 24.0 {
            tilt = 24.0;
        }
        let tilt_scale = |f_hz: f64| -> f64 {
            if tilt == 0.0 { return 1.0; }
            let excess = f_hz - 1500.0;
            if excess <= 0.0 { return 1.0; }
            10f64.powf(tilt * excess / (20.0 * 3000.0))
        };

        let pf1e = frame.pf1 * hs1;
        let pf2e = frame.pf2 * hs2;
        let pf3e = frame.pf3 * hs3;
        let pf4e = frame.pf4 * f4s;
        let pf5e = frame.pf5 * f4s;
        let mut pf6e = frame.pf6 * f4s;
        if pf6e >= K_RESONATOR_NYQUIST_CLAMP_RATIO * self.sample_rate as f64 {
            pf6e = K_PARALLEL_PF6_NYQUIST_SPREAD_RATIO * self.sample_rate as f64;
        }
        output += (self.r1.resonate(input, pf1e, pb1, true) - input) * frame.pa1 * tilt_scale(pf1e);
        output += (self.r2.resonate(input, pf2e, pb2, true) - input) * frame.pa2 * tilt_scale(pf2e);
        output += (self.r3.resonate(input, pf3e, pb3, true) - input) * frame.pa3 * tilt_scale(pf3e);
        output += (self.r4.resonate(input, pf4e, frame.pb4, true) - input) * frame.pa4 * tilt_scale(pf4e);
        output += (self.r5.resonate(input, pf5e, frame.pb5, true) - input) * frame.pa5 * tilt_scale(pf5e);
        output += (self.r6.resonate(input, pf6e, frame.pb6, true) - input) * frame.pa6 * tilt_scale(pf6e);

        if let Some(ex) = frame_ex {
            let (k_par_f7_amp, k_par_f8_amp) = (0.15, 0.07);
            let (default_cf7, default_cb7) = (6500.0, 720.0);
            let (default_cf8, default_cb8) = (7500.0, 1250.0);
            let mut pf7 = ex.cf7;
            let mut pb7v = ex.cb7;
            let mut pf8 = ex.cf8;
            let mut pb8v = ex.cb8;
            if !pf7.is_finite() || pf7 <= 0.0 { pf7 = default_cf7; }
            if !pb7v.is_finite() || pb7v <= 0.0 { pb7v = default_cb7; }
            if !pf8.is_finite() || pf8 <= 0.0 { pf8 = default_cf8; }
            if !pb8v.is_finite() || pb8v <= 0.0 { pb8v = default_cb8; }
            pf7 *= f4s;
            pf8 *= f4s;
            let nyq = 0.5 * self.sample_rate as f64;
            let par_fade = |cf: f64| -> f64 {
                let ratio = cf / nyq;
                if ratio < 0.65 { 1.0 } else if ratio > 0.85 { 0.0 } else { 1.0 - (ratio - 0.65) / 0.20 }
            };
            let fade7 = par_fade(pf7);
            let fade8 = par_fade(pf8);
            if fade7 > 0.0 {
                output += (self.r7.resonate(input, pf7, pb7v, true) - input) * k_par_f7_amp * fade7;
            }
            if fade8 > 0.0 {
                output += (self.r8.resonate(input, pf8, pb8v, true) - input) * k_par_f8_amp * fade8;
            }
        }
        calculate_value_at_fade_position(output, input, frame.parallel_bypass)
    }
}
