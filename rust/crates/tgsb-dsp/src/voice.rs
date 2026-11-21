/*
TGSpeechBox Rust port — LF glottal source with tilt, breathiness and tremor.
Ported from src/voiceGenerator.h (MIT).
*/

use tgsb_frame::{Frame, FrameEx, VoicingTone};

use crate::dsp_common::{
    clamp_double, FastRandom, FrequencyGenerator, NoiseGenerator, OnePoleLowpass, PITWO,
    K_BREATHINESS_ASP_TILT_MAX_DB, K_BREATHINESS_TILT_MAX_DB, K_BREATHINESS_TILT_SMOOTH_MS,
    K_RADIATION_DERIV_GAIN_BASE, K_RADIATION_DERIV_GAIN_REF_SR, K_TURBULENCE_FLOW_POWER,
};
use crate::pitch_model::FujisakiBartmanPitch;

pub struct VoiceGenerator {
    sample_rate: i32,
    pitch_gen: FrequencyGenerator,
    vibrato_gen: FrequencyGenerator,
    tremor_gen: FrequencyGenerator,
    aspiration_gen: NoiseGenerator,
    fujisaki_pitch: FujisakiBartmanPitch,
    fujisaki_was_enabled: bool,
    last_fujisaki_reset: f64,
    last_fujisaki_phrase_amp: f64,
    last_fujisaki_accent_amp: f64,
    last_flow: f64,
    last_voiced_in: f64,
    last_voiced_out: f64,
    last_voiced_src: f64,
    last_asp_out: f64,
    noise_glottal_mod_depth: f64,
    last_noise_mod: f64,
    flow_norm: f64,
    flow_peak_env: f64,
    flow_norm_mean: f64,
    tremor_depth: f64,
    tremor_depth_smooth: f64,
    last_tremor_sin: f64,
    smooth_asp_amp: f64,
    smooth_asp_amp_init: bool,
    asp_attack_coeff: f64,
    asp_release_coeff: f64,
    voiced_anti_alias_lp1: OnePoleLowpass,
    voiced_anti_alias_lp2: OnePoleLowpass,
    voiced_anti_alias_active: bool,
    last_cycle_pos: f64,
    jitter_mul: f64,
    shimmer_mul: f64,
    jitter_shimmer_rng: FastRandom,
    voicing_peak_pos: f64,
    voiced_pre_emph_a: f64,
    voiced_pre_emph_mix: f64,
    speed_quotient: f64,
    chorus_depth: f64,
    chorus_detune_hz: f64,
    chorus_phase: f64,
    tilt_target_tl_db: f64,
    tilt_tl_db: f64,
    tilt_pole: f64,
    tilt_pole_target: f64,
    tilt_state: f64,
    tilt_tl_alpha: f64,
    tilt_pole_alpha: f64,
    tilt_ref_hz: f64,
    tilt_last_tl_for_targets: f64,
    per_frame_tilt_offset: f64,
    per_frame_tilt_offset_target: f64,
    per_frame_tilt_offset_alpha: f64,
    asp_tilt_target_db: f64,
    asp_tilt_smoothed_db: f64,
    asp_tilt_smooth_alpha: f64,
    asp_lp_state: f64,
    fric_lp_state: f64,
    per_frame_asp_tilt_offset: f64,
    per_frame_asp_tilt_offset_target: f64,
    per_frame_asp_tilt_offset_alpha: f64,
    radiation_deriv_gain: f64,
    radiation_mix: f64,
    noise_amplitude_scale: f64,
    source_oversample_active: bool,
    os_base_sharpness: f64,
    os_lf_blend_base: f64,
    os_lf_cap: f64,
    output_base_sharpness: f64,
    output_lf_blend_base: f64,
    output_lf_cap: f64,
    pub glottis_open: bool,
    pub cycle_just_wrapped: bool,
}

impl VoiceGenerator {
    pub fn new(sr: i32) -> VoiceGenerator {
        let mut v = VoiceGenerator {
            sample_rate: sr,
            pitch_gen: FrequencyGenerator::new(sr),
            vibrato_gen: FrequencyGenerator::new(sr),
            tremor_gen: FrequencyGenerator::new(sr),
            aspiration_gen: NoiseGenerator::new(),
            fujisaki_pitch: FujisakiBartmanPitch::new(sr),
            fujisaki_was_enabled: false,
            last_fujisaki_reset: 0.0,
            last_fujisaki_phrase_amp: 0.0,
            last_fujisaki_accent_amp: 0.0,
            last_flow: 0.0,
            last_voiced_in: 0.0,
            last_voiced_out: 0.0,
            last_voiced_src: 0.0,
            last_asp_out: 0.0,
            noise_glottal_mod_depth: 0.0,
            last_noise_mod: 1.0,
            flow_norm: 0.0,
            flow_peak_env: 0.0,
            flow_norm_mean: 0.0,
            tremor_depth: 0.0,
            tremor_depth_smooth: 0.0,
            last_tremor_sin: 0.0,
            smooth_asp_amp: 0.0,
            smooth_asp_amp_init: false,
            asp_attack_coeff: 0.0,
            asp_release_coeff: 0.0,
            voiced_anti_alias_lp1: OnePoleLowpass::new(sr),
            voiced_anti_alias_lp2: OnePoleLowpass::new(sr),
            voiced_anti_alias_active: false,
            last_cycle_pos: 0.0,
            jitter_mul: 1.0,
            shimmer_mul: 1.0,
            jitter_shimmer_rng: FastRandom::new(98765),
            voicing_peak_pos: 0.91,
            voiced_pre_emph_a: 0.92,
            voiced_pre_emph_mix: 0.35,
            speed_quotient: 2.0,
            chorus_depth: 0.0,
            chorus_detune_hz: 2.0,
            chorus_phase: 0.0,
            tilt_target_tl_db: 0.0,
            tilt_tl_db: 0.0,
            tilt_pole: 0.0,
            tilt_pole_target: 0.0,
            tilt_state: 0.0,
            tilt_tl_alpha: 0.0,
            tilt_pole_alpha: 0.0,
            tilt_ref_hz: 3000.0,
            tilt_last_tl_for_targets: 1e9,
            per_frame_tilt_offset: 0.0,
            per_frame_tilt_offset_target: 0.0,
            per_frame_tilt_offset_alpha: 0.0,
            asp_tilt_target_db: 0.0,
            asp_tilt_smoothed_db: 0.0,
            asp_tilt_smooth_alpha: 0.0,
            asp_lp_state: 0.0,
            fric_lp_state: 0.0,
            per_frame_asp_tilt_offset: 0.0,
            per_frame_asp_tilt_offset_target: 0.0,
            per_frame_asp_tilt_offset_alpha: 0.0,
            radiation_deriv_gain: 1.0,
            radiation_mix: 0.0,
            noise_amplitude_scale: 0.0,
            source_oversample_active: false,
            os_base_sharpness: 0.0,
            os_lf_blend_base: 0.0,
            os_lf_cap: 0.0,
            output_base_sharpness: 0.0,
            output_lf_blend_base: 0.0,
            output_lf_cap: 0.0,
            glottis_open: false,
            cycle_just_wrapped: false,
        };
        let tl_smooth_ms = 8.0;
        let pole_smooth_ms = 5.0;
        v.tilt_tl_alpha = 1.0 - (-1.0 / (sr as f64 * (tl_smooth_ms * 0.001))).exp();
        v.tilt_pole_alpha = 1.0 - (-1.0 / (sr as f64 * (pole_smooth_ms * 0.001))).exp();
        v.per_frame_tilt_offset_alpha =
            1.0 - (-1.0 / (sr as f64 * (K_BREATHINESS_TILT_SMOOTH_MS * 0.001))).exp();
        v.per_frame_asp_tilt_offset_alpha =
            1.0 - (-1.0 / (sr as f64 * (K_BREATHINESS_TILT_SMOOTH_MS * 0.001))).exp();
        let asp_tilt_smooth_ms = 10.0;
        v.asp_tilt_smooth_alpha = 1.0 - (-1.0 / (sr as f64 * (asp_tilt_smooth_ms * 0.001))).exp();
        v.asp_attack_coeff = 1.0 - (-1.0 / (0.001 * 1.0 * sr as f64)).exp();
        v.asp_release_coeff = 1.0 - (-1.0 / (0.001 * 12.0 * sr as f64)).exp();

        if sr < 22050 {
            v.voiced_anti_alias_active = true;
            let aa_fc;
            if sr <= 11025 {
                aa_fc = 4000.0;
            } else if sr <= 16000 {
                let t = (sr - 11025) as f64 / (16000.0 - 11025.0);
                aa_fc = 4000.0 + t * 1000.0;
            } else {
                let t = (sr - 16000) as f64 / (22050.0 - 16000.0);
                aa_fc = 5000.0 + t * 1500.0;
            }
            v.voiced_anti_alias_lp1.set_cutoff_hz(aa_fc);
            v.voiced_anti_alias_lp2.set_cutoff_hz(aa_fc);
        }
        let nyq = 0.5 * sr as f64;
        if v.tilt_ref_hz > nyq * 0.95 { v.tilt_ref_hz = nyq * 0.95; }
        if v.tilt_ref_hz < 500.0 { v.tilt_ref_hz = 500.0; }
        v.radiation_deriv_gain = K_RADIATION_DERIV_GAIN_BASE * (sr as f64 / K_RADIATION_DERIV_GAIN_REF_SR);
        v.noise_amplitude_scale = (sr as f64 / 22050.0).powf(0.25);

        v.output_base_sharpness = if sr >= 44100 { 6.0 }
            else if sr >= 32000 { 4.5 }
            else if sr >= 22050 { 3.0 }
            else if sr >= 16000 { 1.5 }
            else { 1.3 };
        if sr <= 11025 {
            v.output_lf_blend_base = 0.30; v.output_lf_cap = 0.35;
        } else if sr >= 16000 {
            v.output_lf_blend_base = 1.0; v.output_lf_cap = 1.0;
        } else {
            v.output_lf_blend_base = 0.30 + 0.70 * (sr - 11025) as f64 / (16000.0 - 11025.0);
            v.output_lf_cap = 0.85;
        }
        if sr < 22050 {
            v.source_oversample_active = true;
            let esr = 2 * sr;
            v.os_base_sharpness = if esr >= 44100 { 4.5 }
                else if esr >= 32000 { 3.5 }
                else if esr >= 22050 { 3.5 }
                else if esr >= 16000 { 1.5 }
                else { 1.3 };
            if esr <= 11025 {
                v.os_lf_blend_base = 0.30; v.os_lf_cap = 0.35;
            } else if esr >= 16000 {
                v.os_lf_blend_base = 1.0; v.os_lf_cap = 1.0;
            } else {
                v.os_lf_blend_base = 0.30 + 0.70 * (esr - 11025) as f64 / (16000.0 - 11025.0);
                v.os_lf_cap = 0.85;
            }
        } else {
            v.source_oversample_active = false;
            v.os_base_sharpness = v.output_base_sharpness;
            v.os_lf_blend_base = v.output_lf_blend_base;
            v.os_lf_cap = v.output_lf_cap;
        }

        let defaults = VoicingTone::defaults();
        v.voicing_peak_pos = defaults.voicing_peak_pos;
        v.voiced_pre_emph_a = defaults.voiced_pre_emph_a;
        v.voiced_pre_emph_mix = defaults.voiced_pre_emph_mix;
        v.noise_glottal_mod_depth = clamp_double(defaults.noise_glottal_mod_depth, 0.0, 1.0);
        v.speed_quotient = clamp_double(defaults.speed_quotient, 0.5, 4.0);
        v.chorus_depth = clamp_double(defaults.chorus_depth, 0.0, 1.0);
        v.chorus_detune_hz = clamp_double(defaults.chorus_detune_hz, 0.5, 5.0);
        v.set_tilt_db_per_oct(defaults.voiced_tilt_db_per_oct);
        v.set_aspiration_tilt_db_per_oct(defaults.aspiration_tilt_db_per_oct);
        v.tilt_tl_db = v.tilt_target_tl_db;
        v.update_tilt_targets(v.tilt_tl_db);
        v.tilt_pole = v.tilt_pole_target;
        v.tilt_last_tl_for_targets = v.tilt_tl_db;
        v
    }

    pub fn reset(&mut self) {
        self.pitch_gen.reset();
        self.vibrato_gen.reset();
        self.aspiration_gen.reset();
        self.fujisaki_pitch.reset_past();
        self.fujisaki_was_enabled = false;
        self.last_fujisaki_reset = 0.0;
        self.last_fujisaki_phrase_amp = 0.0;
        self.last_fujisaki_accent_amp = 0.0;
        self.last_flow = 0.0;
        self.last_voiced_in = 0.0;
        self.last_voiced_out = 0.0;
        self.last_voiced_src = 0.0;
        self.last_asp_out = 0.0;
        self.last_noise_mod = 1.0;
        self.flow_norm = 0.0;
        self.flow_peak_env = 0.0;
        self.flow_norm_mean = 0.0;
        self.smooth_asp_amp = 0.0;
        self.smooth_asp_amp_init = false;
        self.last_cycle_pos = 0.0;
        self.jitter_mul = 1.0;
        self.shimmer_mul = 1.0;
        self.glottis_open = false;
        self.chorus_phase = 0.0;
        self.asp_lp_state = 0.0;
        self.fric_lp_state = 0.0;
        self.voiced_anti_alias_lp1.reset();
        self.voiced_anti_alias_lp2.reset();
        self.asp_tilt_smoothed_db = self.asp_tilt_target_db;
        self.tilt_state = 0.0;
        self.per_frame_tilt_offset = 0.0;
        self.per_frame_tilt_offset_target = 0.0;
        self.per_frame_asp_tilt_offset = 0.0;
        self.per_frame_asp_tilt_offset_target = 0.0;
    }

    pub fn set_tilt_db_per_oct(&mut self, v: f64) {
        self.tilt_target_tl_db = clamp_double(v, -24.0, 24.0);
    }
    pub fn set_voicing_params(
        &mut self,
        peak: f64,
        pre_a: f64,
        pre_mix: f64,
        tilt: f64,
        noise_mod: f64,
        sq: f64,
        asp_tilt: f64,
    ) {
        self.voicing_peak_pos = peak;
        self.voiced_pre_emph_a = pre_a;
        self.voiced_pre_emph_mix = pre_mix;
        self.noise_glottal_mod_depth = clamp_double(noise_mod, 0.0, 1.0);
        self.speed_quotient = clamp_double(sq, 0.5, 4.0);
        self.set_tilt_db_per_oct(tilt);
        self.set_aspiration_tilt_db_per_oct(asp_tilt);
    }
    pub fn set_tremor_depth(&mut self, depth: f64) {
        self.tremor_depth = clamp_double(depth, 0.0, 0.5);
    }
    pub fn set_chorus_params(&mut self, depth: f64, detune: f64) {
        self.chorus_depth = clamp_double(depth, 0.0, 1.0);
        self.chorus_detune_hz = clamp_double(detune, 0.5, 5.0);
    }
    pub fn set_aspiration_tilt_db_per_oct(&mut self, tilt_db: f64) {
        self.asp_tilt_target_db = clamp_double(tilt_db, -24.0, 24.0);
    }
    pub fn get_last_asp_out(&self) -> f64 { self.last_asp_out }
    pub fn get_last_noise_mod(&self) -> f64 { self.last_noise_mod }
    pub fn get_flow_norm_ac(&self) -> f64 { self.flow_norm - self.flow_norm_mean }

    fn calc_pole_for_tilt_db(&self, mut ref_hz: f64, tilt_db: f64) -> f64 {
        if tilt_db.abs() < 1e-5 { return 0.0; }
        if tilt_db > 0.0 {
            let nyq = 0.5 * self.sample_rate as f64;
            if ref_hz < 1.0 { ref_hz = 1.0; }
            if ref_hz > nyq * 0.95 { ref_hz = nyq * 0.95; }
            let g = 10f64.powf(-tilt_db / 20.0);
            let g2 = g * g;
            let w = PITWO * ref_hz / self.sample_rate as f64;
            let cosw = w.cos();
            let aa = g2 - 1.0;
            let b = 2.0 * (1.0 - g2 * cosw);
            let disc = b * b - 4.0 * aa * aa;
            if disc < 0.0 { return 0.0; }
            let sqrt_disc = disc.sqrt();
            let denom = 2.0 * aa;
            if denom.abs() < 1e-18 { return 0.0; }
            let a1 = (-b + sqrt_disc) / denom;
            let a2 = (-b - sqrt_disc) / denom;
            let ok1 = a1 >= 0.0 && a1 < 1.0;
            let ok2 = a2 >= 0.0 && a2 < 1.0;
            let a = if ok1 && ok2 { a1.min(a2) } else if ok1 { a1 } else if ok2 { a2 } else { a1 };
            clamp_double(a, 0.0, 0.9999)
        } else {
            let target_gain = 10f64.powf(-tilt_db / 20.0);
            let a = (1.0 - target_gain) / (1.0 + target_gain);
            clamp_double(a, -0.9, -0.0001)
        }
    }

    fn update_tilt_targets(&mut self, tl_db_now: f64) {
        let tl = clamp_double(tl_db_now, -24.0, 24.0);
        self.tilt_pole_target = self.calc_pole_for_tilt_db(self.tilt_ref_hz, tl);
        let k_base_radiation_mix_max = 0.30;
        let base_mix = k_base_radiation_mix_max * (self.sample_rate as f64 / 16000.0).min(1.0);
        if tl < 0.0 {
            let bright = -tl / 10.0;
            self.radiation_mix = clamp_double(
                base_mix + bright * (1.0 - base_mix),
                base_mix,
                1.0,
            );
        } else {
            self.radiation_mix = clamp_double(base_mix * (1.0 - tl / 12.0), 0.0, base_mix);
        }
    }

    fn apply_tilt(&mut self, input: f64) -> f64 {
        self.per_frame_tilt_offset +=
            (self.per_frame_tilt_offset_target - self.per_frame_tilt_offset) * self.per_frame_tilt_offset_alpha;
        let effective_tilt = self.tilt_target_tl_db + self.per_frame_tilt_offset;
        self.tilt_tl_db += (effective_tilt - self.tilt_tl_db) * self.tilt_tl_alpha;
        if (self.tilt_tl_db - self.tilt_last_tl_for_targets).abs() > 0.01 {
            self.update_tilt_targets(self.tilt_tl_db);
            self.tilt_last_tl_for_targets = self.tilt_tl_db;
        }
        self.tilt_pole += (self.tilt_pole_target - self.tilt_pole) * self.tilt_pole_alpha;
        let out = (1.0 - self.tilt_pole) * input + self.tilt_pole * self.tilt_state;
        self.tilt_state = out;
        out
    }

    fn one_pole_alpha_from_fc(&self, fc_hz: f64) -> f64 {
        let mut fc = fc_hz;
        let nyq = 0.5 * self.sample_rate as f64;
        if fc < 20.0 { fc = 20.0; }
        if fc > nyq * 0.95 { fc = nyq * 0.95; }
        (-PITWO * fc / self.sample_rate as f64).exp()
    }

    fn flow_at_phase(
        &self,
        cycle_pos: f64,
        effective_oq: f64,
        peak_pos: f64,
        pitch_hz: f64,
        frame_ex_sharpness: f64,
        base_sharp: f64,
        lf_blend_b: f64,
        lf_cap_v: f64,
    ) -> f64 {
        if cycle_pos < effective_oq { return 0.0; }
        let mut open_len = 1.0 - effective_oq;
        if open_len < 0.0001 { open_len = 0.0001; }
        let dt = if pitch_hz > 0.0 { pitch_hz / self.sample_rate as f64 } else { 0.0 };
        let mut denom = open_len - dt;
        if denom < 0.0001 { denom = 0.0001; }
        let mut phase = (cycle_pos - effective_oq) / denom;
        if phase < 0.0 { phase = 0.0; }
        if phase > 1.0 { phase = 1.0; }

        let flow_cosine = if phase < peak_pos {
            0.5 * (1.0 - (phase * std::f64::consts::PI / peak_pos).cos())
        } else {
            0.5 * (1.0 + ((phase - peak_pos) * std::f64::consts::PI / (1.0 - peak_pos)).cos())
        };

        let flow_lf = if phase < peak_pos {
            let t = phase / peak_pos;
            let mut open_power = 2.0 + (self.speed_quotient - 2.0) * 0.5;
            if open_power < 1.0 { open_power = 1.0; }
            if open_power > 4.0 { open_power = 4.0; }
            t.powf(open_power) * (3.0 - 2.0 * t)
        } else {
            let t = (phase - peak_pos) / (1.0 - peak_pos);
            let mut bs = base_sharp;
            if frame_ex_sharpness > 0.0 {
                bs *= frame_ex_sharpness;
                if bs < 1.0 { bs = 1.0; }
                if bs > 15.0 { bs = 15.0; }
            }
            let mut sq_factor = 0.4 + (self.speed_quotient - 0.5) * (0.6 / 1.5);
            if sq_factor < 0.3 { sq_factor = 0.3; }
            if sq_factor > 2.0 { sq_factor = 2.0; }
            (1.0 - t).powf(bs * sq_factor)
        };

        let sharp_mul = if frame_ex_sharpness > 0.0 { frame_ex_sharpness } else { 1.0 };
        let lf_scale = clamp_double(sharp_mul, 0.25, 3.0).powf(0.25);
        let lf_blend = clamp_double(lf_blend_b * lf_scale, 0.0, lf_cap_v);
        (1.0 - lf_blend) * flow_cosine + lf_blend * flow_lf
    }

    fn apply_aspiration_tilt(&mut self, x: f64) -> f64 {
        self.per_frame_asp_tilt_offset += (self.per_frame_asp_tilt_offset_target
            - self.per_frame_asp_tilt_offset) * self.per_frame_asp_tilt_offset_alpha;
        self.asp_tilt_smoothed_db +=
            (self.asp_tilt_target_db - self.asp_tilt_smoothed_db) * self.asp_tilt_smooth_alpha;
        let t = self.asp_tilt_smoothed_db + self.per_frame_asp_tilt_offset;
        let mut amt = clamp_double(t.abs() / 18.0, 0.0, 1.0);
        amt = amt.powf(0.65);
        let fc = 6000.0 - 4500.0 * amt;
        let a = self.one_pole_alpha_from_fc(fc);
        self.asp_lp_state = (1.0 - a) * x + a * self.asp_lp_state;
        let lp = self.asp_lp_state;
        let hp = x - lp;
        let bright_amt = if t > 0.0 { amt } else { 0.0 };
        let dark_amt = if t < 0.0 { amt } else { 0.0 };
        x + hp * (1.25 * bright_amt - dark_amt)
    }

    pub fn apply_frication_tilt(&mut self, x: f64) -> f64 {
        let t = self.asp_tilt_smoothed_db;
        let mut amt = clamp_double(t.abs() / 18.0, 0.0, 1.0);
        amt = amt.powf(0.65);
        let fc = 6000.0 - 4500.0 * amt;
        let a = self.one_pole_alpha_from_fc(fc);
        self.fric_lp_state = (1.0 - a) * x + a * self.fric_lp_state;
        let lp = self.fric_lp_state;
        let hp = x - lp;
        let bright_amt = if t > 0.0 { amt } else { 0.0 };
        let dark_amt = if t < 0.0 { amt } else { 0.0 };
        x + hp * (1.25 * bright_amt - dark_amt)
    }

    pub fn get_next(&mut self, frame: &Frame, frame_ex: Option<&FrameEx>) -> f64 {
        let mut creakiness = 0.0;
        let mut breathiness = 0.0;
        let mut jitter = 0.0;
        let mut shimmer = 0.0;
        let mut frame_ex_sharpness = 0.0;
        if let Some(ex) = frame_ex {
            creakiness = ex.creakiness;
            breathiness = ex.breathiness;
            jitter = ex.jitter;
            shimmer = ex.shimmer;
            frame_ex_sharpness = ex.sharpness;
            if !creakiness.is_finite() { creakiness = 0.0; }
            if !breathiness.is_finite() { breathiness = 0.0; }
            if !jitter.is_finite() { jitter = 0.0; }
            if !shimmer.is_finite() { shimmer = 0.0; }
            if !frame_ex_sharpness.is_finite() { frame_ex_sharpness = 0.0; }
            creakiness = clamp_double(creakiness, 0.0, 1.0);
            breathiness = clamp_double(breathiness, 0.0, 1.0);
            jitter = clamp_double(jitter, 0.0, 1.0);
            shimmer = clamp_double(shimmer, 0.0, 1.0);
            frame_ex_sharpness = clamp_double(frame_ex_sharpness, 0.0, 15.0);
            if breathiness > 0.0 { breathiness = breathiness.powf(0.55); }
            self.per_frame_tilt_offset_target = breathiness * K_BREATHINESS_TILT_MAX_DB;
            self.per_frame_asp_tilt_offset_target = breathiness * K_BREATHINESS_ASP_TILT_MAX_DB;
        } else {
            self.per_frame_tilt_offset_target = 0.0;
            self.per_frame_asp_tilt_offset_target = 0.0;
        }

        let mut base_pitch_hz = frame.voice_pitch;
        if !base_pitch_hz.is_finite() || base_pitch_hz < 0.0 { base_pitch_hz = 0.0; }
        let mut pitch_contour_mul = 1.0;
        let use_fujisaki = frame_ex.map_or(false, |e| e.fujisaki_enabled.is_finite() && e.fujisaki_enabled > 0.5);
        if use_fujisaki {
            let ex = frame_ex.unwrap();
            let mut reset_val = ex.fujisaki_reset;
            if !reset_val.is_finite() { reset_val = 0.0; }
            if reset_val > 0.5 && self.last_fujisaki_reset <= 0.5 {
                self.fujisaki_pitch.reset_past();
                self.last_fujisaki_phrase_amp = 0.0;
                self.last_fujisaki_accent_amp = 0.0;
            }
            self.last_fujisaki_reset = reset_val;
            let sr_scale = self.sample_rate as f64 / 22050.0;
            let mut phrase_amp = ex.fujisaki_phrase_amp;
            if !phrase_amp.is_finite() { phrase_amp = 0.0; }
            if phrase_amp > 0.0 && self.last_fujisaki_phrase_amp <= 0.0 {
                let mut pl = ex.fujisaki_phrase_len;
                if !pl.is_finite() { pl = 0.0; }
                let pl_samples = if pl > 0.0 { (pl * sr_scale + 0.5).floor() as i32 } else { 0 };
                self.fujisaki_pitch.phrase(phrase_amp, pl_samples);
            }
            self.last_fujisaki_phrase_amp = phrase_amp;
            let mut accent_amp = ex.fujisaki_accent_amp;
            if !accent_amp.is_finite() { accent_amp = 0.0; }
            if accent_amp > 0.0 && self.last_fujisaki_accent_amp <= 0.0 {
                let mut d = ex.fujisaki_accent_dur;
                let mut al = ex.fujisaki_accent_len;
                if !d.is_finite() { d = 0.0; }
                if !al.is_finite() { al = 0.0; }
                let mut d_samples = if d > 0.0 { (d * sr_scale + 0.5).floor() as i32 } else { 0 };
                let al_samples = if al > 0.0 { (al * sr_scale + 0.5).floor() as i32 } else { 0 };
                if d > 0.0 && d_samples < 1 { d_samples = 1; }
                self.fujisaki_pitch.accent(accent_amp, d_samples, al_samples);
            }
            self.last_fujisaki_accent_amp = accent_amp;
            pitch_contour_mul = self.fujisaki_pitch.process_multiplier();
            if !pitch_contour_mul.is_finite() || pitch_contour_mul <= 0.0 { pitch_contour_mul = 1.0; }
            self.fujisaki_was_enabled = true;
        } else if self.fujisaki_was_enabled {
            self.fujisaki_pitch.reset_past();
            self.fujisaki_was_enabled = false;
            self.last_fujisaki_reset = 0.0;
            self.last_fujisaki_phrase_amp = 0.0;
            self.last_fujisaki_accent_amp = 0.0;
        }

        let vibrato = (self.vibrato_gen.get_next(frame.vibrato_speed) * PITWO).sin() * 0.06
            * frame.vibrato_pitch_offset + 1.0;
        const TREMOR_SMOOTH_ALPHA: f64 = 0.01;
        self.tremor_depth_smooth += (self.tremor_depth - self.tremor_depth_smooth) * TREMOR_SMOOTH_ALPHA;
        let mut tremor_pitch_mod = 1.0;
        if self.tremor_depth_smooth > 0.001 {
            let tremor_phase = self.tremor_gen.get_next(5.0);
            self.last_tremor_sin = (tremor_phase * PITWO).sin();
            let irregularity = 1.0 + self.jitter_shimmer_rng.next_bipolar() * 0.15 * self.tremor_depth_smooth;
            tremor_pitch_mod = 1.0 + self.tremor_depth_smooth * 0.70 * self.last_tremor_sin * irregularity;
        } else {
            self.last_tremor_sin = 0.0;
        }
        let mut pitch_hz = base_pitch_hz * pitch_contour_mul * vibrato * tremor_pitch_mod;
        if !pitch_hz.is_finite() || pitch_hz < 0.0 { pitch_hz = 0.0; }
        if creakiness > 0.0 { pitch_hz *= 1.0 - 0.12 * creakiness; }
        if pitch_hz <= 0.0 {
            self.jitter_mul = 1.0;
            self.shimmer_mul = 1.0;
        }
        pitch_hz *= self.jitter_mul;
        let cycle_pos = self.pitch_gen.get_next(if pitch_hz > 0.0 { pitch_hz } else { 0.0 });
        let cycle_wrapped = pitch_hz > 0.0 && cycle_pos < self.last_cycle_pos;
        self.cycle_just_wrapped = cycle_wrapped;
        self.last_cycle_pos = cycle_pos;
        if cycle_wrapped {
            let jitter_rel = jitter * 0.15 + creakiness * 0.05;
            if jitter_rel > 0.0 {
                let r = self.jitter_shimmer_rng.next_bipolar();
                self.jitter_mul = 1.0 + r * jitter_rel;
                if self.jitter_mul < 0.2 { self.jitter_mul = 0.2; }
            } else {
                self.jitter_mul = 1.0;
            }
            let shimmer_rel = shimmer * 0.70 + creakiness * 0.12;
            if shimmer_rel > 0.0 {
                let r = self.jitter_shimmer_rng.next_bipolar();
                self.shimmer_mul = 1.0 + r * shimmer_rel;
                if self.shimmer_mul < 0.0 { self.shimmer_mul = 0.0; }
            } else {
                self.shimmer_mul = 1.0;
            }
        }

        let mut noise_mod = 1.0;
        if self.noise_glottal_mod_depth > 0.0 && pitch_hz > 0.0 {
            let half_cycle_atten = 0.5 * self.noise_glottal_mod_depth;
            noise_mod = if cycle_pos < 0.5 { 1.0 } else { 1.0 - half_cycle_atten };
            let mut mean_gain = 1.0 - 0.25 * self.noise_glottal_mod_depth;
            if mean_gain < 0.001 { mean_gain = 0.001; }
            noise_mod /= mean_gain;
        }
        self.last_noise_mod = noise_mod;

        let asp_white_frac = if self.sample_rate >= 44100 { 1.0 }
            else if self.sample_rate >= 22050 { 0.70 }
            else if self.sample_rate >= 16000 { 0.50 }
            else { 0.30 };
        let brown = self.aspiration_gen.get_next();
        let white = self.aspiration_gen.white();
        let asp_noise = asp_white_frac * white + (1.0 - asp_white_frac) * brown;
        let asp_base = 0.10 + 0.15 * breathiness;
        let aspiration = asp_noise * asp_base * self.noise_amplitude_scale * noise_mod;
        let aspiration = self.apply_aspiration_tilt(aspiration);

        let mut effective_oq = frame.glottal_open_quotient;
        if effective_oq <= 0.0 { effective_oq = 0.4; }
        if effective_oq < 0.10 { effective_oq = 0.10; }
        if effective_oq > 0.95 { effective_oq = 0.95; }
        if self.tremor_depth_smooth > 0.001 {
            let oq_tremor_mod = self.tremor_depth_smooth * 0.30 * self.last_tremor_sin;
            effective_oq += oq_tremor_mod;
            if effective_oq < 0.10 { effective_oq = 0.10; }
            if effective_oq > 0.95 { effective_oq = 0.95; }
        }
        if creakiness > 0.0 {
            effective_oq += 0.10 * creakiness;
            if effective_oq > 0.95 { effective_oq = 0.95; }
        }
        if breathiness > 0.0 {
            effective_oq -= 0.35 * breathiness;
            if effective_oq < 0.05 { effective_oq = 0.05; }
        }
        self.glottis_open = pitch_hz > 0.0 && cycle_pos >= effective_oq;

        let mut flow = 0.0;
        let mut peak_pos = self.voicing_peak_pos;
        if self.glottis_open {
            let mut open_len = 1.0 - effective_oq;
            if open_len < 0.0001 { open_len = 0.0001; }
            let mut sq_peak_delta = 0.0;
            if self.speed_quotient != 2.0 {
                let ref_peak = 2.0 / 3.0;
                let sq_peak = self.speed_quotient / (1.0 + self.speed_quotient);
                sq_peak_delta = (sq_peak - ref_peak) * 0.6;
            }
            peak_pos = self.voicing_peak_pos + sq_peak_delta
                + 0.02 * breathiness - 0.05 * creakiness;
            if pitch_hz > 0.0 {
                let period_samples = self.sample_rate as f64 / pitch_hz;
                let mut min_close_frac = 2.0 / (period_samples * open_len);
                if min_close_frac > 0.5 { min_close_frac = 0.5; }
                let limit_peak_pos = 1.0 - min_close_frac;
                if limit_peak_pos < peak_pos { peak_pos = limit_peak_pos; }
                if peak_pos < 0.50 { peak_pos = 0.50; }
            }
            if self.source_oversample_active && pitch_hz > 0.0 {
                let phase_inc = pitch_hz / self.sample_rate as f64;
                let mut mid_cycle_pos = cycle_pos - phase_inc * 0.5;
                if mid_cycle_pos < 0.0 { mid_cycle_pos += 1.0; }
                let flow_mid = self.flow_at_phase(
                    mid_cycle_pos, effective_oq, peak_pos, pitch_hz, frame_ex_sharpness,
                    self.os_base_sharpness, self.os_lf_blend_base, self.os_lf_cap);
                let flow_cur = self.flow_at_phase(
                    cycle_pos, effective_oq, peak_pos, pitch_hz, frame_ex_sharpness,
                    self.os_base_sharpness, self.os_lf_blend_base, self.os_lf_cap);
                flow = 0.5 * (flow_mid + flow_cur);
            } else {
                flow = self.flow_at_phase(
                    cycle_pos, effective_oq, peak_pos, pitch_hz, frame_ex_sharpness,
                    self.output_base_sharpness, self.output_lf_blend_base, self.output_lf_cap);
            }
        }

        if self.chorus_depth > 0.001 && pitch_hz > 0.0 {
            let chorus_pitch_hz = pitch_hz + self.chorus_detune_hz;
            self.chorus_phase = (self.chorus_phase + chorus_pitch_hz / self.sample_rate as f64) % 1.0;
            let chorus_flow = if self.source_oversample_active {
                let c_inc = chorus_pitch_hz / self.sample_rate as f64;
                let mut c_mid = self.chorus_phase - c_inc * 0.5;
                if c_mid < 0.0 { c_mid += 1.0; }
                let c_flow_mid = self.flow_at_phase(
                    c_mid, effective_oq, peak_pos, pitch_hz, frame_ex_sharpness,
                    self.os_base_sharpness, self.os_lf_blend_base, self.os_lf_cap);
                let c_flow_cur = self.flow_at_phase(
                    self.chorus_phase, effective_oq, peak_pos, pitch_hz, frame_ex_sharpness,
                    self.os_base_sharpness, self.os_lf_blend_base, self.os_lf_cap);
                0.5 * (c_flow_mid + c_flow_cur)
            } else {
                self.flow_at_phase(
                    self.chorus_phase, effective_oq, peak_pos, pitch_hz, frame_ex_sharpness,
                    self.output_base_sharpness, self.output_lf_blend_base, self.output_lf_cap)
            };
            let blend = self.chorus_depth * 0.5;
            flow = flow * (1.0 - blend) + chorus_flow * blend;
        } else if pitch_hz <= 0.0 {
            self.chorus_phase = 0.0;
        }

        const FLOW_SCALE: f64 = 1.6;
        flow *= FLOW_SCALE;
        {
            let af = if flow > 0.0 { flow } else { 0.0 };
            if af > self.flow_peak_env {
                self.flow_peak_env = af;
            } else {
                self.flow_peak_env *= 0.9997;
            }
            self.flow_norm = if self.flow_peak_env > 1e-6 { af / self.flow_peak_env } else { 0.0 };
            if self.flow_norm > 1.0 { self.flow_norm = 1.0; }
            let mean_alpha = 1.0 - (-1.0 / (self.sample_rate as f64 * 0.020)).exp();
            self.flow_norm_mean += (self.flow_norm - self.flow_norm_mean) * mean_alpha;
        }

        let d_flow = flow - self.last_flow;
        self.last_flow = flow;
        let mut src_deriv = d_flow * self.radiation_deriv_gain;
        src_deriv = 0.6 * (src_deriv / 0.6).tanh();
        let mut voiced_src = (flow + self.radiation_mix * src_deriv) / (1.0 + self.radiation_mix * 0.5);
        let pre = voiced_src - self.voiced_pre_emph_a * self.last_voiced_src;
        self.last_voiced_src = voiced_src;
        voiced_src = (1.0 - self.voiced_pre_emph_mix) * voiced_src + self.voiced_pre_emph_mix * pre;
        voiced_src = self.apply_tilt(voiced_src);

        let mut voice_turb_amp = frame.voice_turbulence_amplitude;
        if !voice_turb_amp.is_finite() { voice_turb_amp = 0.0; }
        voice_turb_amp = clamp_double(voice_turb_amp, 0.0, 1.0);
        if breathiness > 0.0 {
            voice_turb_amp = clamp_double(voice_turb_amp + 0.5 * breathiness, 0.0, 1.0);
        }
        let mut turbulence = aspiration * voice_turb_amp;
        if self.glottis_open {
            let mut flow01 = flow / FLOW_SCALE;
            if flow01 < 0.0 { flow01 = 0.0; }
            if flow01 > 1.0 { flow01 = 1.0; }
            turbulence *= flow01.powf(K_TURBULENCE_FLOW_POWER);
        } else {
            turbulence = 0.0;
        }

        let mut voice_amp = frame.voice_amplitude;
        if !voice_amp.is_finite() { voice_amp = 0.0; }
        voice_amp = clamp_double(voice_amp, 0.0, 1.0);
        if creakiness > 0.0 { voice_amp *= 1.0 - 0.35 * creakiness; }
        if breathiness > 0.0 { voice_amp *= 1.0 - 0.98 * breathiness; }
        voice_amp *= self.shimmer_mul;
        if self.tremor_depth_smooth > 0.001 {
            let amp_irregularity = 1.0 + self.jitter_shimmer_rng.next_bipolar() * 0.1 * self.tremor_depth_smooth;
            let tremor_amp_mod = 1.0 + self.tremor_depth_smooth * 0.5 * self.last_tremor_sin * amp_irregularity;
            voice_amp *= tremor_amp_mod;
        }

        let voiced_in = voiced_src * voice_amp + turbulence;
        let dc_pole = 0.9995;
        let mut voiced = voiced_in - self.last_voiced_in + dc_pole * self.last_voiced_out;
        self.last_voiced_in = voiced_in;
        self.last_voiced_out = voiced;
        if voice_amp < 0.001 && turbulence < 1e-6 {
            self.last_voiced_in *= 0.97;
            self.last_voiced_out *= 0.97;
        }
        if self.voiced_anti_alias_active {
            voiced = self.voiced_anti_alias_lp2.process(self.voiced_anti_alias_lp1.process(voiced));
        }

        let mut target_asp_amp = frame.aspiration_amplitude;
        if !target_asp_amp.is_finite() { target_asp_amp = 0.0; }
        if target_asp_amp < 0.0 { target_asp_amp = 0.0; }
        if target_asp_amp > 1.0 { target_asp_amp = 1.0; }
        if breathiness > 0.0 {
            target_asp_amp = clamp_double(target_asp_amp + breathiness, 0.0, 1.0);
        }
        if !self.smooth_asp_amp_init {
            self.smooth_asp_amp = target_asp_amp;
            self.smooth_asp_amp_init = true;
        } else {
            let coeff = if target_asp_amp > self.smooth_asp_amp { self.asp_attack_coeff } else { self.asp_release_coeff };
            self.smooth_asp_amp += (target_asp_amp - self.smooth_asp_amp) * coeff;
        }
        let asp_out = aspiration * self.smooth_asp_amp;
        self.last_asp_out = asp_out;
        asp_out + voiced
    }
}
