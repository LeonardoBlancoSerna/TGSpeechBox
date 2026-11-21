/*
TGSpeechBox Rust port — frame interpolation and queue management.
Ported from src/frame.cpp (MIT).
*/

use std::collections::VecDeque;

use tgsb_frame::{Frame, FrameEx, FRAME_NUM_PARAMS, FRAMEEX_NUM_PARAMS};

use crate::utils::{
    calculate_freq_at_fade_position, calculate_value_at_fade_position, cosine_smooth,
};

const K_AMP_SPAN_MS: f64 = 45.0;
const K_FORMANT_ALPHA: f64 = 0.004;
const PI: f64 = std::f64::consts::PI;

fn pidx(offset: usize) -> usize {
    offset / std::mem::size_of::<f64>()
}

fn voice_pitch_idx() -> usize { pidx(std::mem::offset_of!(Frame, voice_pitch)) }
fn end_voice_pitch_idx() -> usize { pidx(std::mem::offset_of!(Frame, end_voice_pitch)) }
fn cf_first_idx() -> usize { pidx(std::mem::offset_of!(Frame, cf1)) }
fn cf_last_idx() -> usize { pidx(std::mem::offset_of!(Frame, cf_np)) }
fn pf_first_idx() -> usize { pidx(std::mem::offset_of!(Frame, pf1)) }
fn pf_last_idx() -> usize { pidx(std::mem::offset_of!(Frame, pf6)) }
fn f1_indices() -> [usize; 4] {
    [
        pidx(std::mem::offset_of!(Frame, cf1)),
        pidx(std::mem::offset_of!(Frame, pf1)),
        pidx(std::mem::offset_of!(Frame, cb1)),
        pidx(std::mem::offset_of!(Frame, pb1)),
    ]
}
fn f2_indices() -> [usize; 4] {
    [
        pidx(std::mem::offset_of!(Frame, cf2)),
        pidx(std::mem::offset_of!(Frame, pf2)),
        pidx(std::mem::offset_of!(Frame, cb2)),
        pidx(std::mem::offset_of!(Frame, pb2)),
    ]
}
fn f3_indices() -> [usize; 4] {
    [
        pidx(std::mem::offset_of!(Frame, cf3)),
        pidx(std::mem::offset_of!(Frame, pf3)),
        pidx(std::mem::offset_of!(Frame, cb3)),
        pidx(std::mem::offset_of!(Frame, pb3)),
    ]
}
fn nasal_indices() -> [usize; 5] {
    [
        pidx(std::mem::offset_of!(Frame, cf_n0)),
        pidx(std::mem::offset_of!(Frame, cf_np)),
        pidx(std::mem::offset_of!(Frame, cb_n0)),
        pidx(std::mem::offset_of!(Frame, cb_np)),
        pidx(std::mem::offset_of!(Frame, ca_np)),
    ]
}
fn amplitude_indices() -> [usize; 5] {
    [
        pidx(std::mem::offset_of!(Frame, voice_amplitude)),
        pidx(std::mem::offset_of!(Frame, aspiration_amplitude)),
        pidx(std::mem::offset_of!(Frame, frication_amplitude)),
        pidx(std::mem::offset_of!(Frame, voice_turbulence_amplitude)),
        pidx(std::mem::offset_of!(Frame, pre_formant_gain)),
    ]
}
fn noise_source_indices() -> [usize; 2] {
    [
        pidx(std::mem::offset_of!(Frame, aspiration_amplitude)),
        pidx(std::mem::offset_of!(Frame, frication_amplitude)),
    ]
}
fn voicing_idx() -> usize { pidx(std::mem::offset_of!(Frame, voice_amplitude)) }

fn is_frequency_param(i: usize) -> bool {
    i == voice_pitch_idx()
        || i == end_voice_pitch_idx()
        || (i >= cf_first_idx() && i <= cf_last_idx())
        || (i >= pf_first_idx() && i <= pf_last_idx())
}
fn is_one_of(i: usize, list: &[usize]) -> bool { list.contains(&i) }
fn is_f1(i: usize) -> bool { is_one_of(i, &f1_indices()) }
fn is_f2(i: usize) -> bool { is_one_of(i, &f2_indices()) }
fn is_f3(i: usize) -> bool { is_one_of(i, &f3_indices()) }
fn is_nasal(i: usize) -> bool { is_one_of(i, &nasal_indices()) }
fn is_amplitude(i: usize) -> bool { is_one_of(i, &amplitude_indices()) }
fn is_noise_source(i: usize) -> bool { is_one_of(i, &noise_source_indices()) }
fn is_voicing(i: usize) -> bool { i == voicing_idx() }

struct FrameRequest {
    min_num_samples: u32,
    num_fade_samples: u32,
    null_frame: bool,
    has_frame_ex: bool,
    frame_ex: FrameEx,
    frame: Frame,
    voice_pitch_inc: f64,
    voice_amplitude_inc: f64,
    amp_onset_samples: f64,
    va_from: f64,
    og_from: f64,
    va_target: f64,
    og_target: f64,
    end_cf1: f64,
    end_cf2: f64,
    end_cf3: f64,
    end_pf1: f64,
    end_pf2: f64,
    end_pf3: f64,
    formant_alpha: f64,
    user_index: i32,
}

impl FrameRequest {
    fn new() -> FrameRequest {
        FrameRequest {
            min_num_samples: 0,
            num_fade_samples: 0,
            null_frame: true,
            has_frame_ex: false,
            frame_ex: FrameEx::DEFAULTS,
            frame: zero_frame(),
            voice_pitch_inc: 0.0,
            voice_amplitude_inc: 0.0,
            amp_onset_samples: 0.0,
            va_from: 0.0,
            og_from: 0.0,
            va_target: 0.0,
            og_target: 0.0,
            end_cf1: f64::NAN,
            end_cf2: f64::NAN,
            end_cf3: f64::NAN,
            end_pf1: f64::NAN,
            end_pf2: f64::NAN,
            end_pf3: f64::NAN,
            formant_alpha: 0.0,
            user_index: -1,
        }
    }
}

fn zero_frame() -> Frame {
    unsafe { std::mem::zeroed::<Frame>() }
}

pub struct FrameManager {
    sample_rate: i32,
    queue: VecDeque<Box<FrameRequest>>,
    old_request: Box<FrameRequest>,
    new_request: Option<Box<FrameRequest>>,
    cur_frame: Frame,
    cur_frame_ex: FrameEx,
    cur_frame_is_null: bool,
    cur_has_frame_ex: bool,
    sample_counter: u32,
    last_user_index: i32,
    purge_flag: bool,
}

impl FrameManager {
    pub fn new(sample_rate: i32) -> FrameManager {
        FrameManager {
            sample_rate,
            queue: VecDeque::new(),
            old_request: Box::new(FrameRequest::new()),
            new_request: None,
            cur_frame: zero_frame(),
            cur_frame_ex: FrameEx::DEFAULTS,
            cur_frame_is_null: true,
            cur_has_frame_ex: false,
            sample_counter: 0,
            last_user_index: -1,
            purge_flag: false,
        }
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
        let mut req = Box::new(FrameRequest::new());
        req.min_num_samples = min_num_samples;
        req.num_fade_samples = if num_fade_samples > 0 { num_fade_samples } else { 1 };
        if let Some(f) = frame {
            req.null_frame = false;
            req.frame = f;
            req.voice_pitch_inc = if min_num_samples > 0 {
                (f.end_voice_pitch - f.voice_pitch) / min_num_samples as f64
            } else {
                0.0
            };
        } else {
            req.null_frame = true;
            req.frame = zero_frame();
            req.voice_pitch_inc = 0.0;
        }
        req.va_target = req.frame.voice_amplitude;
        req.og_target = req.frame.output_gain;

        if let Some(ex) = frame_ex {
            req.has_frame_ex = true;
            req.frame_ex = ex;
            if !req.null_frame
                && req.min_num_samples > 0
                && req.frame.voice_amplitude > 0.0
                && req.frame_ex.end_voice_amplitude.is_finite()
            {
                let mut end_va = req.frame_ex.end_voice_amplitude;
                if end_va < 0.0 { end_va = 0.0; }
                if end_va > req.frame.voice_amplitude { end_va = req.frame.voice_amplitude; }
                req.voice_amplitude_inc =
                    (end_va - req.frame.voice_amplitude) / req.min_num_samples as f64;
            }
            if !req.null_frame
                && req.min_num_samples > 0
                && req.frame_ex.amplitude_onset_ms.is_finite()
                && req.frame_ex.amplitude_onset_ms > 0.0
            {
                let mut onset = req.frame_ex.amplitude_onset_ms * self.sample_rate as f64 / 1000.0;
                if onset > req.min_num_samples as f64 { onset = req.min_num_samples as f64; }
                req.amp_onset_samples = onset;
                if req.voice_amplitude_inc != 0.0 {
                    let mut end_va = req.frame_ex.end_voice_amplitude;
                    if end_va < 0.0 { end_va = 0.0; }
                    if end_va > req.va_target { end_va = req.va_target; }
                    let mut rem = req.min_num_samples as f64 - onset;
                    if rem < 1.0 { rem = 1.0; }
                    req.voice_amplitude_inc = (end_va - req.va_target) / rem;
                }
            }

            let mut has_any = false;
            if req.frame_ex.end_cf1.is_finite() { req.end_cf1 = req.frame_ex.end_cf1; has_any = true; }
            if req.frame_ex.end_cf2.is_finite() { req.end_cf2 = req.frame_ex.end_cf2; has_any = true; }
            if req.frame_ex.end_cf3.is_finite() { req.end_cf3 = req.frame_ex.end_cf3; has_any = true; }
            if req.frame_ex.end_pf1.is_finite() { req.end_pf1 = req.frame_ex.end_pf1; has_any = true; }
            if req.frame_ex.end_pf2.is_finite() { req.end_pf2 = req.frame_ex.end_pf2; has_any = true; }
            if req.frame_ex.end_pf3.is_finite() { req.end_pf3 = req.frame_ex.end_pf3; has_any = true; }
            if has_any {
                let mut alpha = K_FORMANT_ALPHA;
                if min_num_samples > 0 {
                    let max_tau = min_num_samples as f64 * 0.10;
                    let base_tau = 1.0 / K_FORMANT_ALPHA;
                    if base_tau > max_tau && max_tau > 0.0 {
                        alpha = 1.0 / max_tau;
                    }
                }
                req.formant_alpha = alpha;
            }
        } else {
            req.has_frame_ex = false;
            req.frame_ex = FrameEx::DEFAULTS;
        }
        req.user_index = user_index;

        if purge_queue {
            self.queue.clear();
            self.sample_counter = self.old_request.min_num_samples;
            self.old_request.null_frame = true;
            self.cur_frame_is_null = true;
            self.new_request = None;
            self.purge_flag = true;
        }
        self.queue.push_back(req);
    }

    pub fn get_last_index(&self) -> i32 { self.last_user_index }

    pub fn check_and_clear_purge_flag(&mut self) -> bool {
        let was = self.purge_flag;
        self.purge_flag = false;
        was
    }

    pub fn current(&mut self) -> (Option<Frame>, Option<FrameEx>) {
        self.update_current_frame();
        if self.cur_frame_is_null {
            (None, None)
        } else if self.cur_has_frame_ex {
            (Some(self.cur_frame), Some(self.cur_frame_ex))
        } else {
            (Some(self.cur_frame), None)
        }
    }

    fn update_current_frame(&mut self) {
        self.sample_counter = self.sample_counter.wrapping_add(1);

        if self.new_request.is_some() {
            let fade = self.new_request.as_ref().unwrap().num_fade_samples;
            if self.sample_counter > fade {
                let new = self.new_request.take().unwrap();
                self.old_request = new;
                self.new_request = None;
                self.cur_frame = self.old_request.frame;
                self.cur_frame_ex = self.old_request.frame_ex;
                self.cur_has_frame_ex = self.old_request.has_frame_ex;
                if self.old_request.amp_onset_samples > 0.0 {
                    let onset = self.old_request.amp_onset_samples;
                    let va_from = self.old_request.va_from;
                    let va_target = self.old_request.va_target;
                    let og_from = self.old_request.og_from;
                    let og_target = self.old_request.og_target;
                    let mut ratio = self.sample_counter as f64 / onset;
                    if ratio > 1.0 { ratio = 1.0; }
                    if ratio < 0.0 { ratio = 0.0; }
                    self.cur_frame.voice_amplitude = va_from + (va_target - va_from) * ratio;
                    self.cur_frame.output_gain = og_from + (og_target - og_from) * ratio;
                    if self.cur_frame.voice_amplitude < 0.0 { self.cur_frame.voice_amplitude = 0.0; }
                    if self.cur_frame.output_gain < 0.0 { self.cur_frame.output_gain = 0.0; }
                }
            } else {
                self.interpolate_new_frame();
            }
            return;
        }

        if self.sample_counter > self.old_request.min_num_samples {
            if let Some(mut next) = self.queue.pop_front() {
                let was_from_silence = self.cur_frame_is_null || self.old_request.null_frame;
                self.cur_frame_is_null = false;
                if next.null_frame {
                    let old_asp = self.old_request.frame.aspiration_amplitude;
                    next.frame = self.old_request.frame;
                    if old_asp > 0.01 {
                        next.frame.pre_formant_gain = 0.3;
                    } else {
                        next.frame.pre_formant_gain = 0.0;
                        next.frame.aspiration_amplitude = 0.0;
                    }
                    next.frame.voice_amplitude = 0.0;
                    next.frame.frication_amplitude = 0.0;
                    next.frame.voice_turbulence_amplitude = 0.0;
                    next.frame.voice_pitch = self.cur_frame.voice_pitch;
                    next.voice_pitch_inc = 0.0;
                    next.voice_amplitude_inc = 0.0;
                    next.amp_onset_samples = 0.0;
                    next.frame_ex = self.old_request.frame_ex;
                    next.has_frame_ex = self.old_request.has_frame_ex;
                } else if self.old_request.null_frame {
                    self.old_request.frame = next.frame;
                    self.old_request.frame.pre_formant_gain = 0.0;
                    self.old_request.frame.voice_amplitude = 0.0;
                    self.old_request.frame.aspiration_amplitude = 0.0;
                    self.old_request.frame.frication_amplitude = 0.0;
                    self.old_request.frame.voice_turbulence_amplitude = 0.0;
                    self.old_request.null_frame = false;
                    self.old_request.frame_ex = next.frame_ex;
                    self.old_request.has_frame_ex = next.has_frame_ex;
                }
                if next.user_index != -1 {
                    self.last_user_index = next.user_index;
                }
                self.sample_counter = 0;
                if was_from_silence {
                    self.cur_frame = self.old_request.frame;
                    self.cur_frame_ex = self.old_request.frame_ex;
                    self.cur_has_frame_ex = self.old_request.has_frame_ex;
                }
                let ramp = next.num_fade_samples.min(next.min_num_samples);
                next.frame.voice_pitch += next.voice_pitch_inc * ramp as f64;
                next.va_from = self.cur_frame.voice_amplitude;
                next.og_from = self.cur_frame.output_gain;
                if next.voice_amplitude_inc != 0.0 && next.amp_onset_samples <= 0.0 {
                    next.frame.voice_amplitude += next.voice_amplitude_inc * ramp as f64;
                    if next.frame.voice_amplitude < 0.0 { next.frame.voice_amplitude = 0.0; }
                }
                self.new_request = Some(next);
            } else {
                self.cur_frame_is_null = true;
                self.cur_has_frame_ex = false;
                self.cur_frame_ex = FrameEx::DEFAULTS;
                self.old_request.null_frame = true;
            }
        } else {
            self.advance_old_frame();
        }
    }

    fn interpolate_new_frame(&mut self) {
        let linear_ratio = self.sample_counter as f64 / self.new_request.as_ref().unwrap().num_fade_samples as f64;
        let amp_span_samples = K_AMP_SPAN_MS * self.sample_rate as f64 / 1000.0;
        let mut amp_ratio = linear_ratio;
        if self.new_request.as_ref().unwrap().num_fade_samples as f64 > amp_span_samples {
            amp_ratio = self.sample_counter as f64 / amp_span_samples;
            if amp_ratio > 1.0 { amp_ratio = 1.0; }
        }

        let (sc_f1, sc_f2, sc_f3, sc_n, amp_mode, source_hold_raw, voicing_hold_raw) = {
            let n = self.new_request.as_ref().unwrap();
            if n.has_frame_ex {
                (
                    n.frame_ex.trans_f1_scale,
                    n.frame_ex.trans_f2_scale,
                    n.frame_ex.trans_f3_scale,
                    n.frame_ex.trans_nasal_scale,
                    n.frame_ex.trans_amplitude_mode,
                    n.frame_ex.trans_source_hold_ratio,
                    n.frame_ex.trans_voicing_hold_ratio,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
            }
        };
        let source_hold = if source_hold_raw > 0.99 { 0.99 } else { source_hold_raw };
        let voicing_hold = if voicing_hold_raw > 0.99 { 0.99 } else { voicing_hold_raw };
        let no_formant_targets = {
            let n = self.new_request.as_ref().unwrap();
            n.has_frame_ex
                && !n.end_cf1.is_finite()
                && !n.end_cf2.is_finite()
                && !n.end_cf3.is_finite()
        };

        let old = self.old_request.frame.params();
        let new = self.new_request.as_ref().unwrap().frame.params();
        let mut cur = [0.0f64; FRAME_NUM_PARAMS];

        for i in 0..FRAME_NUM_PARAMS {
            let old_val = old[i];
            let new_val = new[i];
            if no_formant_targets {
                if is_noise_source(i) && source_hold > 0.0 {
                    let dr = if linear_ratio <= source_hold { 0.0 } else { (linear_ratio - source_hold) / (1.0 - source_hold) };
                    let val = old_val * (1.0 - dr) + new_val * linear_ratio;
                    cur[i] = val.min(old_val.max(new_val));
                } else if is_voicing(i) && voicing_hold > 0.0 {
                    let dr = if linear_ratio <= voicing_hold { 0.0 } else { (linear_ratio - voicing_hold) / (1.0 - voicing_hold) };
                    let val = old_val * (1.0 - linear_ratio) + new_val * dr;
                    cur[i] = val.min(old_val.max(new_val));
                } else if is_amplitude(i) {
                    cur[i] = calculate_value_at_fade_position(old_val, new_val, amp_ratio);
                } else {
                    cur[i] = calculate_value_at_fade_position(old_val, new_val, linear_ratio);
                }
            } else {
                let mut param_linear = linear_ratio;
                let mut scale = 0.0;
                if sc_f1 > 0.0 && is_f1(i) { scale = sc_f1; }
                else if sc_f2 > 0.0 && is_f2(i) { scale = sc_f2; }
                else if sc_f3 > 0.0 && is_f3(i) { scale = sc_f3; }
                else if sc_n > 0.0 && is_nasal(i) { scale = sc_n; }
                if scale > 0.0 && scale != 1.0 {
                    param_linear = linear_ratio / scale;
                    if param_linear > 1.0 { param_linear = 1.0; }
                }
                if is_frequency_param(i) {
                    cur[i] = calculate_freq_at_fade_position(old_val, new_val, cosine_smooth(param_linear));
                } else if is_noise_source(i) && source_hold > 0.0 {
                    let delayed = if param_linear <= source_hold { 0.0 } else { (param_linear - source_hold) / (1.0 - source_hold) };
                    if amp_mode > 0.5 {
                        let theta = delayed * PI / 2.0;
                        let theta_new = param_linear * PI / 2.0;
                        cur[i] = old_val * theta.cos() + new_val * theta_new.sin();
                    } else {
                        let val = old_val * (1.0 - delayed) + new_val * param_linear;
                        cur[i] = val.min(old_val.max(new_val));
                    }
                } else if is_voicing(i) && voicing_hold > 0.0 {
                    let delayed = if param_linear <= voicing_hold { 0.0 } else { (param_linear - voicing_hold) / (1.0 - voicing_hold) };
                    if amp_mode > 0.5 {
                        let theta = param_linear * PI / 2.0;
                        let theta_new = delayed * PI / 2.0;
                        cur[i] = old_val * theta.cos() + new_val * theta_new.sin();
                    } else {
                        let val = old_val * (1.0 - param_linear) + new_val * delayed;
                        cur[i] = val.min(old_val.max(new_val));
                    }
                } else if is_amplitude(i) && amp_mode > 0.5 {
                    let theta = amp_ratio * PI / 2.0;
                    cur[i] = old_val * theta.cos() + new_val * theta.sin();
                } else if is_amplitude(i) {
                    cur[i] = calculate_value_at_fade_position(old_val, new_val, amp_ratio);
                } else {
                    cur[i] = calculate_value_at_fade_position(old_val, new_val, param_linear);
                }
            }
        }

        self.cur_frame = unsafe { std::mem::transmute::<[f64; FRAME_NUM_PARAMS], Frame>(cur) };

        if !no_formant_targets {
            for &(cf_idx, cb_idx) in &[
                (pidx(std::mem::offset_of!(Frame, cf1)), pidx(std::mem::offset_of!(Frame, cb1))),
                (pidx(std::mem::offset_of!(Frame, cf2)), pidx(std::mem::offset_of!(Frame, cb2))),
                (pidx(std::mem::offset_of!(Frame, cf3)), pidx(std::mem::offset_of!(Frame, cb3))),
                (pidx(std::mem::offset_of!(Frame, pf1)), pidx(std::mem::offset_of!(Frame, pb1))),
                (pidx(std::mem::offset_of!(Frame, pf2)), pidx(std::mem::offset_of!(Frame, pb2))),
                (pidx(std::mem::offset_of!(Frame, pf3)), pidx(std::mem::offset_of!(Frame, pb3))),
            ] {
                let delta = (new[cf_idx] - old[cf_idx]).abs();
                if delta > 150.0 {
                    let extra = (linear_ratio * PI).sin() * (delta - 150.0) * 0.40;
                    self.cur_frame.params_mut()[cb_idx] += extra;
                }
            }
        }

        if self.new_request.as_ref().unwrap().amp_onset_samples > 0.0 {
            let onset = self.new_request.as_ref().unwrap().amp_onset_samples;
            let va_from = self.new_request.as_ref().unwrap().va_from;
            let va_target = self.new_request.as_ref().unwrap().va_target;
            let og_from = self.new_request.as_ref().unwrap().og_from;
            let og_target = self.new_request.as_ref().unwrap().og_target;
            let mut ratio = self.sample_counter as f64 / onset;
            if ratio > 1.0 { ratio = 1.0; }
            if ratio < 0.0 { ratio = 0.0; }
            self.cur_frame.voice_amplitude = va_from + (va_target - va_from) * ratio;
            self.cur_frame.output_gain = og_from + (og_target - og_from) * ratio;
            if self.cur_frame.voice_amplitude < 0.0 { self.cur_frame.voice_amplitude = 0.0; }
            if self.cur_frame.output_gain < 0.0 { self.cur_frame.output_gain = 0.0; }
        }

        if self.old_request.has_frame_ex || self.new_request.as_ref().unwrap().has_frame_ex {
            self.cur_has_frame_ex = true;
            let old_ex = self.old_request.frame_ex.params();
            let new_ex = self.new_request.as_ref().unwrap().frame_ex.params();
            let pitch_start = pidx(std::mem::offset_of!(FrameEx, fujisaki_enabled));
            let mut cur_ex = [0.0f64; FRAMEEX_NUM_PARAMS];
            for i in 0..FRAMEEX_NUM_PARAMS {
                cur_ex[i] = if i >= pitch_start {
                    new_ex[i]
                } else {
                    calculate_value_at_fade_position(old_ex[i], new_ex[i], linear_ratio)
                };
            }
            self.cur_frame_ex = unsafe { std::mem::transmute::<[f64; FRAMEEX_NUM_PARAMS], FrameEx>(cur_ex) };
        } else {
            self.cur_has_frame_ex = false;
            self.cur_frame_ex = FrameEx::DEFAULTS;
        }
    }

    fn advance_old_frame(&mut self) {
        if self.old_request.null_frame
            && (self.cur_frame.aspiration_amplitude > 0.0 || self.cur_frame.pre_formant_gain > 0.0)
        {
            let tail = (-1000.0 / (12.0 * self.sample_rate as f64)).exp();
            self.cur_frame.aspiration_amplitude *= tail;
            self.cur_frame.pre_formant_gain *= tail;
            if self.cur_frame.aspiration_amplitude < 1e-4 { self.cur_frame.aspiration_amplitude = 0.0; }
            if self.cur_frame.pre_formant_gain < 1e-4 { self.cur_frame.pre_formant_gain = 0.0; }
            self.old_request.frame.aspiration_amplitude = self.cur_frame.aspiration_amplitude;
            self.old_request.frame.pre_formant_gain = self.cur_frame.pre_formant_gain;
        }
        self.cur_frame.voice_pitch += self.old_request.voice_pitch_inc;
        self.old_request.frame.voice_pitch = self.cur_frame.voice_pitch;

        if self.old_request.amp_onset_samples > 0.0
            && self.sample_counter as f64 <= self.old_request.amp_onset_samples
        {
            let onset = self.old_request.amp_onset_samples;
            let va_from = self.old_request.va_from;
            let va_target = self.old_request.va_target;
            let og_from = self.old_request.og_from;
            let og_target = self.old_request.og_target;
            let mut ratio = self.sample_counter as f64 / onset;
            if ratio > 1.0 { ratio = 1.0; }
            if ratio < 0.0 { ratio = 0.0; }
            self.cur_frame.voice_amplitude = va_from + (va_target - va_from) * ratio;
            self.cur_frame.output_gain = og_from + (og_target - og_from) * ratio;
            if self.cur_frame.voice_amplitude < 0.0 { self.cur_frame.voice_amplitude = 0.0; }
            if self.cur_frame.output_gain < 0.0 { self.cur_frame.output_gain = 0.0; }
            self.old_request.frame.voice_amplitude = self.cur_frame.voice_amplitude;
            self.old_request.frame.output_gain = self.cur_frame.output_gain;
        } else if self.old_request.voice_amplitude_inc != 0.0 {
            self.cur_frame.voice_amplitude += self.old_request.voice_amplitude_inc;
            if self.cur_frame.voice_amplitude < 0.0 { self.cur_frame.voice_amplitude = 0.0; }
            self.old_request.frame.voice_amplitude = self.cur_frame.voice_amplitude;
        }

        let alpha = self.old_request.formant_alpha;
        if self.old_request.end_cf1.is_finite() {
            self.cur_frame.cf1 += alpha * (self.old_request.end_cf1 - self.cur_frame.cf1);
            self.old_request.frame.cf1 = self.cur_frame.cf1;
        }
        if self.old_request.end_cf2.is_finite() {
            self.cur_frame.cf2 += alpha * (self.old_request.end_cf2 - self.cur_frame.cf2);
            self.old_request.frame.cf2 = self.cur_frame.cf2;
        }
        if self.old_request.end_cf3.is_finite() {
            self.cur_frame.cf3 += alpha * (self.old_request.end_cf3 - self.cur_frame.cf3);
            self.old_request.frame.cf3 = self.cur_frame.cf3;
        }
        if self.old_request.end_pf1.is_finite() {
            self.cur_frame.pf1 += alpha * (self.old_request.end_pf1 - self.cur_frame.pf1);
            self.old_request.frame.pf1 = self.cur_frame.pf1;
        }
        if self.old_request.end_pf2.is_finite() {
            self.cur_frame.pf2 += alpha * (self.old_request.end_pf2 - self.cur_frame.pf2);
            self.old_request.frame.pf2 = self.cur_frame.pf2;
        }
        if self.old_request.end_pf3.is_finite() {
            self.cur_frame.pf3 += alpha * (self.old_request.end_pf3 - self.cur_frame.pf3);
            self.old_request.frame.pf3 = self.cur_frame.pf3;
        }
    }
}
