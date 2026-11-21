/*
TGSpeechBox Rust port — Fujisaki-Bartman pitch contour model.
Ported from src/pitchModel.h (MIT).
*/

use crate::dsp_common::clamp_double;

pub struct FujisakiBartmanPitch {
    pa: f64,
    pb: f64,
    pc: f64,
    aa: f64,
    ab: f64,
    ac: f64,
    px1: f64,
    px2: f64,
    ax1: f64,
    ax2: f64,
    phr: f64,
    acc: f64,
    countdown: i32,
    default_phrase_len: i32,
    default_accent_len: i32,
    default_accent_dur: i32,
}

fn clamp_int(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo { lo } else if v > hi { hi } else { v }
}

impl FujisakiBartmanPitch {
    pub fn new(sample_rate: i32) -> FujisakiBartmanPitch {
        let mut p = FujisakiBartmanPitch {
            pa: 0.0, pb: 0.0, pc: 0.0,
            aa: 0.0, ab: 0.0, ac: 0.0,
            px1: 0.0, px2: 0.0, ax1: 0.0, ax2: 0.0,
            phr: 0.0, acc: 0.0, countdown: 0,
            default_phrase_len: 4250,
            default_accent_len: 1024,
            default_accent_dur: 7500,
        };
        if sample_rate > 0 {
            let scale = sample_rate as f64 / 22050.0;
            p.default_phrase_len = (0.5 + 4250.0 * scale).floor() as i32;
            p.default_accent_len = (0.5 + 1024.0 * scale).floor() as i32;
            p.default_accent_dur = (0.5 + 7500.0 * scale).floor() as i32;
        }
        p.default_phrase_len = clamp_int(p.default_phrase_len, 1, 200000);
        p.default_accent_len = clamp_int(p.default_accent_len, 1, 200000);
        p.default_accent_dur = clamp_int(p.default_accent_dur, 1, 200000);
        p.design_phrase(p.default_phrase_len);
        p.design_accent(p.default_accent_len);
        p
    }

    fn design_phrase(&mut self, mut l: i32) {
        if l < 1 { l = 1; }
        let nf = -1.0 / l as f64;
        let r = nf.exp();
        let c = -(r * r);
        let b = 2.0 * r;
        let p = (std::f64::consts::E * nf).exp();
        let a = 1.0 - b * p - c * p;
        self.pa = a; self.pb = b; self.pc = c;
    }

    fn design_accent(&mut self, mut l: i32) {
        if l < 1 { l = 1; }
        let nf = -1.0 / l as f64;
        let r = nf.exp();
        let c = -(r * r);
        let b = 2.0 * r;
        let a = 1.0 - b - c;
        self.aa = a; self.ab = b; self.ac = c;
    }

    pub fn reset_past(&mut self) {
        self.px1 = 0.0;
        self.px2 = 0.0;
        self.ax1 = 0.0;
        self.ax2 = 0.0;
        self.phr = 0.0;
        self.acc = 0.0;
        self.countdown = 0;
    }

    pub fn phrase(&mut self, a: f64, phrase_len_samples: i32) {
        if !(a > 0.0) { return; }
        self.phr = a;
        let l = if phrase_len_samples > 0 { phrase_len_samples } else { self.default_phrase_len };
        self.design_phrase(l);
    }

    pub fn accent(&mut self, a: f64, duration_samples: i32, accent_len_samples: i32) {
        if !(a > 0.0) { return; }
        self.acc = a;
        self.countdown = if duration_samples > 0 { duration_samples } else { self.default_accent_dur };
        let l = if accent_len_samples > 0 { accent_len_samples } else { self.default_accent_len };
        self.design_accent(l);
    }

    pub fn process_multiplier(&mut self) -> f64 {
        let y1 = self.pa * self.phr + self.pb * self.px1 + self.pc * self.px2;
        self.px2 = self.px1;
        self.px1 = y1;
        self.phr = 0.0;

        let mut aimp = 0.0;
        if self.countdown > 0 {
            aimp = self.acc;
            self.countdown -= 1;
        }
        let y2 = self.aa * aimp + self.ab * self.ax1 + self.ac * self.ax2;
        self.ax2 = self.ax1;
        self.ax1 = y2;

        clamp_double(y1 + y2, -24.0, 24.0).exp()
    }
}
