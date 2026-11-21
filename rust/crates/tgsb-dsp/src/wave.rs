/*
TGSpeechBox Rust port — main speech wave generator (orchestrator).
Ported from src/speechWaveGenerator.cpp (MIT).
*/

use tgsb_frame::{Frame, FrameEx, Sample, VoicingTone};

use crate::dsp_common::{
    clamp_double, FastRandom, NoiseGenerator, OnePoleLowpass, PITWO,
    K_ASP_BURST_FC_11K, K_ASP_BURST_FC_16K, K_ASP_BURST_FC_22K, K_ASP_BURST_FC_44K,
    K_BURSTINESS_SCALE, K_BYPASS_MIN_GAIN, K_BYPASS_VOICED_DUCK, K_CONSONANT_CLARITY_BOOST,
    K_FRIC_BURST_FC_11K, K_FRIC_BURST_FC_16K, K_FRIC_BURST_FC_22K, K_FRIC_BURST_FC_44K,
    K_FRIC_NOISE_SCALE, K_FRIC_SOFT_CLIP_K, K_FRIC_SUSTAIN_FC_11K, K_FRIC_SUSTAIN_FC_16K,
    K_FRIC_SUSTAIN_FC_22K, K_FRIC_SUSTAIN_FC_44K, K_VOICED_FRIC_DUCK,
    K_VOICED_FRIC_DUCK_POWER, K_VOICED_FRIC_MOD_DEPTH, K_VOICED_FRIC_PRODUCT_THRESH,
};
use crate::formant::{CascadeFormantGenerator, ParallelFormantGenerator};
use crate::frame_manager::FrameManager;
use crate::resonator::Resonator;
use crate::voice::VoiceGenerator;

const K_SOURCE_TRACT_COUPLING: f64 = 0.30;
const K_GLOTTAL_CYCLE_BW_DELTA: f64 = 15.0;
const K_SUBGLOTTAL_COUPLING: f64 = 0.3;
const K_SG1_FREQ: f64 = 630.0;
const K_SG1_BW: f64 = 70.0;
const K_SG_Z1_FREQ: f64 = 590.0;
const K_SG_Z1_BW: f64 = 90.0;
const K_SG_CLOSED_PHASE_LEAK: f64 = 0.20;
const K_SHELF_DUCK_MAX: f64 = 0.90;
const K_CLIP_LEVEL: f64 = 32767.0 / 6000.0;
const KNEE_HALF: f64 = 1.0;
const K_LIMITER_LOOKAHEAD_MAX: usize = 220;

pub struct SpeechWaveGenerator {
    sample_rate: i32,
    frame_manager: FrameManager,
    voice_generator: VoiceGenerator,
    fric_generator: NoiseGenerator,
    cascade: CascadeFormantGenerator,
    parallel: ParallelFormantGenerator,
    last_input: f64,
    last_output: f64,
    was_silence: bool,
    smooth_pre_gain: f64,
    pre_gain_attack_alpha: f64,
    pre_gain_release_alpha: f64,
    smooth_fric_amp: f64,
    fric_attack_alpha: f64,
    fric_release_alpha: f64,
    hs_in1: f64,
    hs_in2: f64,
    hs_out1: f64,
    hs_out2: f64,
    hs_b0: f64,
    hs_b1: f64,
    hs_b2: f64,
    hs_a1: f64,
    hs_a2: f64,
    current_tone: VoicingTone,
    fric_burst_lp1: OnePoleLowpass,
    fric_burst_lp2: OnePoleLowpass,
    fric_sustain_lp1: OnePoleLowpass,
    fric_sustain_lp2: OnePoleLowpass,
    last_target_fric_amp: f64,
    last_target_asp_amp: f64,
    fric_burst_fc: f64,
    fric_sustain_fc: f64,
    burst_env: f64,
    burst_env_decay_mul: f64,
    asp_lp1: OnePoleLowpass,
    asp_lp2: OnePoleLowpass,
    asp_burst_fc: f64,
    shelf_mix: f64,
    shelf_mix_alpha: f64,
    last_bright_out: f64,
    stop_fade_remaining: i32,
    stop_fade_total: i32,
    start_fade_remaining: i32,
    start_fade_total: i32,
    smooth_cascade_duck: f64,
    cascade_duck_alpha: f64,
    limiter_gain: f64,
    limiter_attack_alpha: f64,
    limiter_release_alpha: f64,
    limiter_threshold: f64,
    limiter_hold_counter: i32,
    limiter_hold_samples: i32,
    limiter_delay_buf: [f64; K_LIMITER_LOOKAHEAD_MAX],
    limiter_delay_len: usize,
    limiter_delay_idx: usize,
    stc_tilt_state: f64,
    stc_smooth_pole: f64,
    stc_smooth_bw_off: f64,
    stc_alpha: f64,
    sg_pole: Resonator,
    sg_zero: Resonator,
    sg_coupling_smooth: f64,
    sg_coupling_alpha: f64,
    output_gain: f64,
    time_stretch_factor: f64,
    time_stretch_accum: f64,
    pr_b0: f64,
    pr_b1: f64,
    pr_b2: f64,
    pr_a1: f64,
    pr_a2: f64,
    pr_in1: f64,
    pr_in2: f64,
    pr_out1: f64,
    pr_out2: f64,
    pr2_b0: f64,
    pr2_b1: f64,
    pr2_b2: f64,
    pr2_a1: f64,
    pr2_a2: f64,
    pr2_in1: f64,
    pr2_in2: f64,
    pr2_out1: f64,
    pr2_out2: f64,
    _rng_marker: FastRandom,
}

impl SpeechWaveGenerator {
    pub fn new(sample_rate: i32) -> SpeechWaveGenerator {
        let attack_alpha = |ms: f64| 1.0 - (-1.0 / (sample_rate as f64 * (ms * 0.001))).exp();
        let mut g = SpeechWaveGenerator {
            sample_rate,
            frame_manager: FrameManager::new(sample_rate),
            voice_generator: VoiceGenerator::new(sample_rate),
            fric_generator: NoiseGenerator::new(),
            cascade: CascadeFormantGenerator::new(sample_rate),
            parallel: ParallelFormantGenerator::new(sample_rate),
            last_input: 0.0,
            last_output: 0.0,
            was_silence: true,
            smooth_pre_gain: 0.0,
            pre_gain_attack_alpha: attack_alpha(1.0),
            pre_gain_release_alpha: attack_alpha(0.5),
            smooth_fric_amp: 0.0,
            fric_attack_alpha: attack_alpha(0.8),
            fric_release_alpha: attack_alpha(1.2),
            hs_in1: 0.0, hs_in2: 0.0, hs_out1: 0.0, hs_out2: 0.0,
            hs_b0: 0.0, hs_b1: 0.0, hs_b2: 0.0, hs_a1: 0.0, hs_a2: 0.0,
            current_tone: VoicingTone::defaults(),
            fric_burst_lp1: OnePoleLowpass::new(sample_rate),
            fric_burst_lp2: OnePoleLowpass::new(sample_rate),
            fric_sustain_lp1: OnePoleLowpass::new(sample_rate),
            fric_sustain_lp2: OnePoleLowpass::new(sample_rate),
            last_target_fric_amp: 0.0,
            last_target_asp_amp: 0.0,
            fric_burst_fc: 0.0,
            fric_sustain_fc: 0.0,
            burst_env: 0.0,
            burst_env_decay_mul: (-1.0 / (sample_rate as f64 * (6.0 * 0.001))).exp(),
            asp_lp1: OnePoleLowpass::new(sample_rate),
            asp_lp2: OnePoleLowpass::new(sample_rate),
            asp_burst_fc: 0.0,
            shelf_mix: 1.0,
            shelf_mix_alpha: attack_alpha(4.0),
            last_bright_out: 0.0,
            stop_fade_remaining: 0,
            stop_fade_total: 0,
            start_fade_remaining: 0,
            start_fade_total: 0,
            smooth_cascade_duck: 1.0,
            cascade_duck_alpha: attack_alpha(3.0),
            limiter_gain: 1.0,
            limiter_attack_alpha: attack_alpha(1.0),
            limiter_release_alpha: attack_alpha(40.0),
            limiter_threshold: 4.0,
            limiter_hold_counter: 0,
            limiter_hold_samples: (5.0 * sample_rate as f64 * 0.001) as i32,
            limiter_delay_buf: [0.0; K_LIMITER_LOOKAHEAD_MAX],
            limiter_delay_len: (5.0 * sample_rate as f64 * 0.001) as usize,
            limiter_delay_idx: 0,
            stc_tilt_state: 0.0,
            stc_smooth_pole: 0.0,
            stc_smooth_bw_off: 0.0,
            stc_alpha: attack_alpha(5.0),
            sg_pole: Resonator::new(sample_rate, false),
            sg_zero: Resonator::new(sample_rate, true),
            sg_coupling_smooth: 0.0,
            sg_coupling_alpha: (-1.0 / (sample_rate as f64 * (1.5 * 0.001))).exp() * -1.0 + 1.0,
            output_gain: 1.0,
            time_stretch_factor: 1.0,
            time_stretch_accum: 0.0,
            pr_b0: 0.0, pr_b1: 0.0, pr_b2: 0.0, pr_a1: 0.0, pr_a2: 0.0,
            pr_in1: 0.0, pr_in2: 0.0, pr_out1: 0.0, pr_out2: 0.0,
            pr2_b0: 0.0, pr2_b1: 0.0, pr2_b2: 0.0, pr2_a1: 0.0, pr2_a2: 0.0,
            pr2_in1: 0.0, pr2_in2: 0.0, pr2_out1: 0.0, pr2_out2: 0.0,
            _rng_marker: FastRandom::new(12345),
        };
        if g.limiter_delay_len > K_LIMITER_LOOKAHEAD_MAX { g.limiter_delay_len = K_LIMITER_LOOKAHEAD_MAX; }
        if g.limiter_delay_len < 1 { g.limiter_delay_len = 1; }

        g.current_tone.cascade_bw_scale = 1.0;
        g.init_high_shelf(
            g.current_tone.high_shelf_fc_hz,
            g.current_tone.high_shelf_gain_db,
            g.current_tone.high_shelf_q,
        );
        g.init_presence();
        g.cascade.set_pitch_sync_params(
            g.current_tone.pitch_sync_f1_delta_hz,
            g.current_tone.pitch_sync_b1_delta_hz + K_GLOTTAL_CYCLE_BW_DELTA,
        );

        let sr = sample_rate as f64;
        if sample_rate <= 11025 {
            g.fric_burst_fc = K_FRIC_BURST_FC_11K;
            g.fric_sustain_fc = K_FRIC_SUSTAIN_FC_11K;
        } else if sample_rate <= 16000 {
            let t = (sr - 11025.0) / (16000.0 - 11025.0);
            g.fric_burst_fc = K_FRIC_BURST_FC_11K + t * (K_FRIC_BURST_FC_16K - K_FRIC_BURST_FC_11K);
            g.fric_sustain_fc = K_FRIC_SUSTAIN_FC_11K + t * (K_FRIC_SUSTAIN_FC_16K - K_FRIC_SUSTAIN_FC_11K);
        } else if sample_rate <= 22050 {
            let t = (sr - 16000.0) / (22050.0 - 16000.0);
            g.fric_burst_fc = K_FRIC_BURST_FC_16K + t * (K_FRIC_BURST_FC_22K - K_FRIC_BURST_FC_16K);
            g.fric_sustain_fc = K_FRIC_SUSTAIN_FC_16K + t * (K_FRIC_SUSTAIN_FC_22K - K_FRIC_SUSTAIN_FC_16K);
        } else if sample_rate <= 44100 {
            let t = (sr - 22050.0) / (44100.0 - 22050.0);
            g.fric_burst_fc = K_FRIC_BURST_FC_22K + t * (K_FRIC_BURST_FC_44K - K_FRIC_BURST_FC_22K);
            g.fric_sustain_fc = K_FRIC_SUSTAIN_FC_22K + t * (K_FRIC_SUSTAIN_FC_44K - K_FRIC_SUSTAIN_FC_22K);
        } else {
            let scale = sr / 44100.0;
            g.fric_burst_fc = K_FRIC_BURST_FC_44K * scale;
            g.fric_sustain_fc = K_FRIC_SUSTAIN_FC_44K * scale;
        }
        g.fric_burst_lp1.set_cutoff_hz(g.fric_burst_fc);
        g.fric_burst_lp2.set_cutoff_hz(g.fric_burst_fc);
        g.fric_sustain_lp1.set_cutoff_hz(g.fric_sustain_fc);
        g.fric_sustain_lp2.set_cutoff_hz(g.fric_sustain_fc);

        if sample_rate <= 11025 {
            g.asp_burst_fc = K_ASP_BURST_FC_11K;
        } else if sample_rate <= 16000 {
            let t = (sr - 11025.0) / (16000.0 - 11025.0);
            g.asp_burst_fc = K_ASP_BURST_FC_11K + t * (K_ASP_BURST_FC_16K - K_ASP_BURST_FC_11K);
        } else if sample_rate <= 22050 {
            let t = (sr - 16000.0) / (22050.0 - 16000.0);
            g.asp_burst_fc = K_ASP_BURST_FC_16K + t * (K_ASP_BURST_FC_22K - K_ASP_BURST_FC_16K);
        } else if sample_rate <= 44100 {
            let t = (sr - 22050.0) / (44100.0 - 22050.0);
            g.asp_burst_fc = K_ASP_BURST_FC_22K + t * (K_ASP_BURST_FC_44K - K_ASP_BURST_FC_22K);
        } else {
            g.asp_burst_fc = K_ASP_BURST_FC_44K * sr / 44100.0;
        }
        g.asp_lp1.set_cutoff_hz(g.asp_burst_fc);
        g.asp_lp2.set_cutoff_hz(g.asp_burst_fc);
        g
    }

    pub fn queue_frame(
        &mut self,
        frame: Option<Frame>,
        frame_ex: Option<FrameEx>,
        min_num_samples: u32,
        num_fade_samples: u32,
        user_index: i32,
        purge_queue: bool,
    ) {
        self.frame_manager.queue_frame(
            frame,
            frame_ex,
            min_num_samples,
            num_fade_samples,
            user_index,
            purge_queue,
        );
    }

    pub fn generate(&mut self, sample_count: usize, buffer: &mut [Sample]) -> usize {
        self.generate_index_aware(sample_count, buffer, None)
    }

    pub fn generate_index_aware(
        &mut self,
        sample_count: usize,
        buffer: &mut [Sample],
        mut index_out: Option<&mut i32>,
    ) -> usize {
        if let Some(out) = index_out.as_deref_mut() { *out = -1; }
        let count = sample_count.min(buffer.len());
        let entry_user_index = self.frame_manager.get_last_index();

        if self.frame_manager.check_and_clear_purge_flag() {
            self.hs_in1 = 0.0; self.hs_in2 = 0.0; self.hs_out1 = 0.0; self.hs_out2 = 0.0;
            self.last_input = 0.0;
            self.last_output = 0.0;
            self.start_fade_total = (self.sample_rate as f64 * 0.004) as i32;
            if self.start_fade_total < 64 { self.start_fade_total = 64; }
            self.start_fade_remaining = self.start_fade_total;
            self.was_silence = true;
        }

        for i in 0..count {
            let (frame_opt, frame_ex_opt) = self.frame_manager.current();
            if let Some(ref frame) = frame_opt {
                if self.was_silence {
                    self.voice_generator.reset();
                    self.fric_generator.reset();
                    self.cascade.reset();
                    self.parallel.reset();
                    self.last_input = 0.0;
                    self.last_output = 0.0;
                    self.smooth_pre_gain = 0.0;
                    self.smooth_fric_amp = 0.0;
                    self.last_target_fric_amp = 0.0;
                    self.last_target_asp_amp = 0.0;
                    self.burst_env = 0.0;
                    self.fric_burst_lp1.reset(); self.fric_burst_lp2.reset();
                    self.fric_sustain_lp1.reset(); self.fric_sustain_lp2.reset();
                    self.asp_lp1.reset(); self.asp_lp2.reset();
                    self.shelf_mix = 1.0;
                    self.smooth_cascade_duck = 1.0;
                    self.limiter_gain = 1.0;
                    self.limiter_hold_counter = 0;
                    self.limiter_delay_idx = 0;
                    self.limiter_delay_buf = [0.0; K_LIMITER_LOOKAHEAD_MAX];
                    self.stc_tilt_state = 0.0;
                    self.stc_smooth_pole = 0.0;
                    self.stc_smooth_bw_off = 0.0;
                    self.sg_pole.reset();
                    self.sg_zero.reset();
                    self.sg_coupling_smooth = 0.0;
                    self.stop_fade_total = 0;
                    self.stop_fade_remaining = 0;
                    self.hs_in1 = 0.0; self.hs_in2 = 0.0; self.hs_out1 = 0.0; self.hs_out2 = 0.0;
                    self.start_fade_total = (self.sample_rate as f64 * 0.005) as i32;
                    if self.start_fade_total < 16 { self.start_fade_total = 16; }
                    self.start_fade_remaining = self.start_fade_total;
                    self.was_silence = false;
                }

                let target_pre_gain = frame.pre_formant_gain;
                let prev_smooth = self.smooth_pre_gain;
                let alpha = if target_pre_gain > self.smooth_pre_gain {
                    self.pre_gain_attack_alpha
                } else {
                    self.pre_gain_release_alpha
                };
                self.smooth_pre_gain += (target_pre_gain - self.smooth_pre_gain) * alpha;
                if self.smooth_pre_gain < 0.01 {
                    self.cascade.decay(0.95);
                    self.parallel.decay(0.95);
                    self.last_input *= 0.9;
                    self.last_output *= 0.9;
                }
                if prev_smooth < 0.005 && target_pre_gain > 0.01 {
                    self.cascade.decay(0.3);
                    self.parallel.decay(0.3);
                }

                let voice = self.voice_generator.get_next(frame, frame_ex_opt.as_ref());
                let asp = self.voice_generator.get_last_asp_out();
                let mut voiced_only = voice - asp;

                let mut f1_hz = frame.cf1;
                if f1_hz < 150.0 { f1_hz = 150.0; }
                if f1_hz > 1000.0 { f1_hz = 1000.0; }
                let f1_norm = (f1_hz - 150.0) / 850.0;
                let coupling_amt = (1.0 - f1_norm) * K_SOURCE_TRACT_COUPLING;
                let target_pole = coupling_amt * 0.35;
                self.stc_smooth_pole += (target_pole - self.stc_smooth_pole) * self.stc_alpha;
                let p = self.stc_smooth_pole;
                let stc_out = (1.0 - p) * voiced_only + p * self.stc_tilt_state;
                self.stc_tilt_state = stc_out;
                voiced_only = stc_out;
                let target_bw_off = coupling_amt * 25.0;
                self.stc_smooth_bw_off += (target_bw_off - self.stc_smooth_bw_off) * self.stc_alpha;
                self.cascade.set_f1_bw_offset(self.stc_smooth_bw_off);

                let target_fric_amp = frame.frication_amplitude;
                let fric_alpha = if target_fric_amp > self.smooth_fric_amp {
                    self.fric_attack_alpha
                } else {
                    self.fric_release_alpha
                };
                self.smooth_fric_amp += (target_fric_amp - self.smooth_fric_amp) * fric_alpha;
                let mut fric_amp = self.smooth_fric_amp;
                if K_FRIC_SOFT_CLIP_K > 0.0 {
                    fric_amp = fric_amp * (1.0 - K_FRIC_SOFT_CLIP_K * fric_amp);
                    if fric_amp < 0.0 { fric_amp = 0.0; }
                }

                let mut bypass = frame.parallel_bypass;
                if bypass < 0.0 { bypass = 0.0; }
                if bypass > 1.0 { bypass = 1.0; }
                let bypass_gain = 1.0 - bypass * (1.0 - K_BYPASS_MIN_GAIN);
                let mut va = frame.voice_amplitude;
                if va < 0.0 { va = 0.0; }
                if va > 1.0 { va = 1.0; }
                let bypass_voiced_duck = if bypass > 0.3 && va > 0.0 { 1.0 - K_BYPASS_VOICED_DUCK * va } else { 1.0 };
                let mut voiced_fric_scale = 1.0;
                if va > 0.0 {
                    voiced_fric_scale = 1.0 - K_VOICED_FRIC_DUCK * va.powf(K_VOICED_FRIC_DUCK_POWER);
                    if voiced_fric_scale < 0.0 { voiced_fric_scale = 0.0; }
                }

                let d_fric = target_fric_amp - self.last_target_fric_amp;
                self.last_target_fric_amp = target_fric_amp;
                let mut inst_fric = 0.0;
                if d_fric > 0.0 {
                    let sr_scale = self.sample_rate as f64 / 22050.0;
                    inst_fric = d_fric * K_BURSTINESS_SCALE * sr_scale;
                    if inst_fric > 1.0 { inst_fric = 1.0; }
                }
                let d_asp = frame.aspiration_amplitude - self.last_target_asp_amp;
                self.last_target_asp_amp = frame.aspiration_amplitude;
                let mut inst_asp = 0.0;
                if d_asp > 0.0 {
                    let sr_scale = self.sample_rate as f64 / 22050.0;
                    inst_asp = d_asp * 40.0 * sr_scale;
                    if inst_asp > 1.0 { inst_asp = 1.0; }
                }
                let mut inst = inst_fric.max(inst_asp);
                inst *= 1.0 - va;
                self.burst_env *= self.burst_env_decay_mul;
                if inst > self.burst_env { self.burst_env = inst; }
                let burstiness = self.burst_env;

                let mut asp = asp;
                let asp_filt = self.asp_lp2.process(self.asp_lp1.process(asp));
                asp += burstiness * (asp_filt - asp);
                let mut voice_for_cascade = voiced_only + asp;

                let sg_target = if self.voice_generator.glottis_open {
                    K_SUBGLOTTAL_COUPLING
                } else {
                    K_SUBGLOTTAL_COUPLING * K_SG_CLOSED_PHASE_LEAK
                };
                self.sg_coupling_smooth += (sg_target - self.sg_coupling_smooth) * self.sg_coupling_alpha;
                let mut sg_filtered = self.sg_zero.resonate(voice_for_cascade, K_SG_Z1_FREQ, K_SG_Z1_BW, true);
                sg_filtered = self.sg_pole.resonate(sg_filtered, K_SG1_FREQ, K_SG1_BW, true);
                voice_for_cascade += self.sg_coupling_smooth * (sg_filtered - voice_for_cascade);

                let cascade_out = self.cascade.get_next(
                    frame,
                    frame_ex_opt.as_ref(),
                    self.voice_generator.glottis_open,
                    voice_for_cascade * self.smooth_pre_gain,
                );

                let mut fric_noise = self.fric_generator.get_next()
                    * K_FRIC_NOISE_SCALE * fric_amp * bypass_gain * bypass_voiced_duck * voiced_fric_scale;
                fric_noise = self.voice_generator.apply_frication_tilt(fric_noise);
                fric_noise *= self.voice_generator.get_last_noise_mod();
                if va * fric_amp > K_VOICED_FRIC_PRODUCT_THRESH {
                    let flow_ac = self.voice_generator.get_flow_norm_ac();
                    let mut mod_v = 1.0 + K_VOICED_FRIC_MOD_DEPTH * flow_ac;
                    if mod_v < 0.0 { mod_v = 0.0; }
                    fric_noise *= mod_v;
                }
                let fric_burst = self.fric_burst_lp2.process(self.fric_burst_lp1.process(fric_noise));
                let fric_sustain = self.fric_sustain_lp2.process(self.fric_sustain_lp1.process(fric_noise));
                let mut fric = fric_sustain + burstiness * (fric_burst - fric_sustain);
                fric *= 1.0 + K_CONSONANT_CLARITY_BOOST * (1.0 - burstiness);
                let parallel_out = self.parallel.get_next(
                    frame,
                    frame_ex_opt.as_ref(),
                    self.voice_generator.glottis_open,
                    fric * self.smooth_pre_gain,
                );

                let nasal_protect = 1.0 - frame.ca_np;
                let target_duck = 1.0 - 0.7 * self.burst_env * (1.0 - va) * nasal_protect;
                self.smooth_cascade_duck += self.cascade_duck_alpha * (target_duck - self.smooth_cascade_duck);
                let out = (cascade_out * self.smooth_cascade_duck + parallel_out) * frame.output_gain;

                let filtered_out = out - self.last_input + 0.9995 * self.last_output;
                self.last_input = out;
                self.last_output = filtered_out;
                let shelved = self.apply_high_shelf(filtered_out);
                let target_shelf_mix = 1.0 - K_SHELF_DUCK_MAX * self.burst_env * (1.0 - va);
                self.shelf_mix += (target_shelf_mix - self.shelf_mix) * self.shelf_mix_alpha;
                let mut bright = filtered_out + self.shelf_mix * (shelved - filtered_out);
                bright = self.apply_presence(bright) * 0.94;

                if self.start_fade_remaining > 0 {
                    let fade_in = 1.0 - self.start_fade_remaining as f64 / self.start_fade_total as f64;
                    bright *= fade_in;
                    self.start_fade_remaining -= 1;
                }
                bright *= self.output_gain;
                bright = self.apply_limiter(bright);
                self.last_bright_out = bright;

                let mut scaled = bright * 6000.0;
                if scaled > 32767.0 { scaled = 32767.0; }
                if scaled < -32767.0 { scaled = -32767.0; }
                buffer[i] = Sample { value: scaled as i16 };

                if self.time_stretch_factor > 1.01 {
                    self.time_stretch_accum += self.time_stretch_factor - 1.0;
                    while self.time_stretch_accum >= 1.0 {
                        self.frame_manager.current();
                        self.time_stretch_accum -= 1.0;
                    }
                }

                if let Some(out) = index_out.as_deref_mut() {
                    let cur = self.frame_manager.get_last_index();
                    if cur != entry_user_index {
                        *out = cur;
                        for s in buffer.iter_mut().take(count).skip(i + 1) { *s = Sample { value: 0 }; }
                        return i + 1;
                    }
                }
            } else {
                if !self.was_silence {
                    if self.stop_fade_total == 0 {
                        self.stop_fade_total = (self.sample_rate as f64 * 0.004) as i32;
                        if self.stop_fade_total < 16 { self.stop_fade_total = 16; }
                        self.stop_fade_remaining = self.stop_fade_total;
                    }
                    if self.stop_fade_remaining > 0 {
                        let t = (self.stop_fade_remaining - 1) as f64 / (self.stop_fade_total - 1) as f64;
                        let tail = self.last_bright_out * t;
                        let mut scaled = tail * 6000.0;
                        if scaled > 32767.0 { scaled = 32767.0; }
                        if scaled < -32767.0 { scaled = -32767.0; }
                        buffer[i] = Sample { value: scaled as i16 };
                        self.stop_fade_remaining -= 1;
                        if let Some(out) = index_out.as_deref_mut() {
                            let cur = self.frame_manager.get_last_index();
                            if cur != entry_user_index {
                                *out = cur;
                                for s in buffer.iter_mut().take(count).skip(i + 1) { *s = Sample { value: 0 }; }
                                return i + 1;
                            }
                        }
                        continue;
                    }
                }
                self.was_silence = true;
                self.stop_fade_total = 0;
                self.stop_fade_remaining = 0;
                self.hs_in1 = 0.0; self.hs_in2 = 0.0; self.hs_out1 = 0.0; self.hs_out2 = 0.0;
                for s in buffer.iter_mut().take(count).skip(i) { *s = Sample { value: 0 }; }
                if let Some(out) = index_out.as_deref_mut() {
                    let cur = self.frame_manager.get_last_index();
                    if cur != entry_user_index { *out = cur; }
                }
                return i;
            }
        }
        count
    }

    fn init_presence(&mut self) {
        let gain_db = 6.0f64;
        let q = 0.75f64;
        let mut nyq = 0.5 * self.sample_rate as f64;
        let mut fc = 4100.0f64;
        if fc > nyq * 0.90 { fc = nyq * 0.90; }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = PITWO * fc / self.sample_rate as f64;
        let (cosw0, sinw0) = (w0.cos(), w0.sin());
        let alpha = sinw0 / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        self.pr_b0 = (1.0 + alpha * a) / a0;
        self.pr_b1 = (-2.0 * cosw0) / a0;
        self.pr_b2 = (1.0 - alpha * a) / a0;
        self.pr_a1 = (-2.0 * cosw0) / a0;
        self.pr_a2 = (1.0 - alpha / a) / a0;
        self.pr_in1 = 0.0; self.pr_in2 = 0.0; self.pr_out1 = 0.0; self.pr_out2 = 0.0;

        let gain2_db = 4.0f64;
        let q2 = 0.8f64;
        nyq = 0.5 * self.sample_rate as f64;
        let mut fc2 = 4900.0f64;
        if fc2 > nyq * 0.90 { fc2 = nyq * 0.90; }
        let a2v = 10f64.powf(gain2_db / 40.0);
        let w02 = PITWO * fc2 / self.sample_rate as f64;
        let (cosw02, sinw02) = (w02.cos(), w02.sin());
        let alpha2 = sinw02 / (2.0 * q2);
        let a02 = 1.0 + alpha2 / a2v;
        self.pr2_b0 = (1.0 + alpha2 * a2v) / a02;
        self.pr2_b1 = (-2.0 * cosw02) / a02;
        self.pr2_b2 = (1.0 - alpha2 * a2v) / a02;
        self.pr2_a1 = (-2.0 * cosw02) / a02;
        self.pr2_a2 = (1.0 - alpha2 / a2v) / a02;
        self.pr2_in1 = 0.0; self.pr2_in2 = 0.0; self.pr2_out1 = 0.0; self.pr2_out2 = 0.0;
    }

    fn apply_presence(&mut self, input: f64) -> f64 {
        let out = self.pr_b0 * input + self.pr_b1 * self.pr_in1 + self.pr_b2 * self.pr_in2
            - self.pr_a1 * self.pr_out1 - self.pr_a2 * self.pr_out2;
        self.pr_in2 = self.pr_in1; self.pr_in1 = input;
        self.pr_out2 = self.pr_out1; self.pr_out1 = out;
        let out2 = self.pr2_b0 * out + self.pr2_b1 * self.pr2_in1 + self.pr2_b2 * self.pr2_in2
            - self.pr2_a1 * self.pr2_out1 - self.pr2_a2 * self.pr2_out2;
        self.pr2_in2 = self.pr2_in1; self.pr2_in1 = out;
        self.pr2_out2 = self.pr2_out1; self.pr2_out1 = out2;
        out2
    }

    fn init_high_shelf(&mut self, mut fc: f64, mut gain_db: f64, mut q: f64) {
        let nyq = 0.5 * self.sample_rate as f64;
        if !fc.is_finite() { fc = 2000.0; }
        if !gain_db.is_finite() { gain_db = 0.0; }
        if !q.is_finite() { q = 0.7; }
        if fc < 20.0 { fc = 20.0; }
        if fc > nyq * 0.95 { fc = nyq * 0.95; }
        if q < 0.1 { q = 0.1; }
        if q > 4.0 { q = 4.0; }
        if gain_db < -24.0 { gain_db = -24.0; }
        if gain_db > 24.0 { gain_db = 24.0; }
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = PITWO * fc / self.sample_rate as f64;
        let cosw0 = w0.cos();
        let sinw0 = w0.sin();
        let alpha = sinw0 / (2.0 * q);
        let sqrt_a = a.sqrt();
        let a0 = (a + 1.0) - (a - 1.0) * cosw0 + 2.0 * sqrt_a * alpha;
        self.hs_b0 = (a * ((a + 1.0) + (a - 1.0) * cosw0 + 2.0 * sqrt_a * alpha)) / a0;
        self.hs_b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cosw0)) / a0;
        self.hs_b2 = (a * ((a + 1.0) + (a - 1.0) * cosw0 - 2.0 * sqrt_a * alpha)) / a0;
        self.hs_a1 = (2.0 * ((a - 1.0) - (a + 1.0) * cosw0)) / a0;
        self.hs_a2 = ((a + 1.0) - (a - 1.0) * cosw0 - 2.0 * sqrt_a * alpha) / a0;
    }

    fn apply_high_shelf(&mut self, input: f64) -> f64 {
        let out = self.hs_b0 * input + self.hs_b1 * self.hs_in1 + self.hs_b2 * self.hs_in2
            - self.hs_a1 * self.hs_out1 - self.hs_a2 * self.hs_out2;
        self.hs_in2 = self.hs_in1; self.hs_in1 = input;
        self.hs_out2 = self.hs_out1; self.hs_out1 = out;
        out
    }

    fn apply_limiter(&mut self, bright: f64) -> f64 {
        let eff_threshold = (self.limiter_threshold * self.output_gain).min(K_CLIP_LEVEL);
        let knee_lo = eff_threshold - KNEE_HALF;
        let knee_hi = eff_threshold + KNEE_HALF;

        let delayed = self.limiter_delay_buf[self.limiter_delay_idx];
        self.limiter_delay_buf[self.limiter_delay_idx] = bright;
        self.limiter_delay_idx = (self.limiter_delay_idx + 1) % self.limiter_delay_len;

        let mut peak_ahead = 0.0;
        for &v in self.limiter_delay_buf.iter().take(self.limiter_delay_len) {
            let mag = v.abs();
            if mag > peak_ahead { peak_ahead = mag; }
        }
        let target_gain = if peak_ahead <= knee_lo || peak_ahead < 1e-12 {
            1.0
        } else if peak_ahead >= knee_hi {
            eff_threshold / peak_ahead
        } else {
            let t = (peak_ahead - knee_lo) / (knee_hi - knee_lo);
            1.0 + t * t * (eff_threshold / peak_ahead - 1.0)
        };
        if target_gain < self.limiter_gain {
            self.limiter_gain += self.limiter_attack_alpha * (target_gain - self.limiter_gain);
            self.limiter_hold_counter = self.limiter_hold_samples;
        } else if self.limiter_hold_counter > 0 {
            self.limiter_hold_counter -= 1;
        } else {
            self.limiter_gain += self.limiter_release_alpha * (target_gain - self.limiter_gain);
        }
        delayed * self.limiter_gain
    }

    pub fn set_voicing_tone(&mut self, tone: VoicingTone) {
        let mut merged = tone;
        merged.magic = tgsb_frame::VOICINGTONE_MAGIC;
        merged.struct_size = std::mem::size_of::<VoicingTone>() as u32;
        merged.struct_version = tgsb_frame::VOICINGTONE_VERSION;
        merged.dsp_version = tgsb_frame::DSP_VERSION;
        self.current_tone = merged;

        self.voice_generator.set_voicing_params(
            merged.voicing_peak_pos,
            merged.voiced_pre_emph_a,
            merged.voiced_pre_emph_mix,
            merged.voiced_tilt_db_per_oct,
            merged.noise_glottal_mod_depth,
            merged.speed_quotient,
            merged.aspiration_tilt_db_per_oct,
        );
        self.init_high_shelf(merged.high_shelf_fc_hz, merged.high_shelf_gain_db, merged.high_shelf_q);
        self.init_presence();
        self.cascade.set_pitch_sync_params(
            merged.pitch_sync_f1_delta_hz,
            merged.pitch_sync_b1_delta_hz + K_GLOTTAL_CYCLE_BW_DELTA,
        );
        self.cascade.set_cascade_bw_scale(merged.cascade_bw_scale);
        self.cascade.set_nasal_bw_scale(merged.nasal_bw_scale);
        self.cascade.set_f4_freq_scale(merged.f4_freq_scale);
        self.cascade.set_nasal_gain_scale(merged.nasal_gain_scale);
        self.parallel.set_f4_freq_scale(merged.f4_freq_scale);
        self.voice_generator.set_tremor_depth(merged.tremor_depth);
        self.voice_generator.set_chorus_params(merged.chorus_depth, merged.chorus_detune_hz);
    }

    pub fn get_voicing_tone(&self) -> VoicingTone { self.current_tone }

    pub fn set_output_gain(&mut self, gain: f64) {
        self.output_gain = clamp_double(gain, 0.0, 10.0);
    }

    pub fn set_time_stretch(&mut self, factor: f64) {
        self.time_stretch_factor = clamp_double(factor, 1.0, 8.0);
        self.time_stretch_accum = 0.0;
    }
}
