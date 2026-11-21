/*
TGSpeechBox Rust port — ABI-stable POD types.
Copyright 2014 NV Access Limited.
Copyright 2025-2026 Tamas Geczy.
Licensed under the MIT License. See LICENSE for details.

These types mirror, field for field and in the same order, the structs in
src/frame.h, src/sample.h and src/voicingTone.h. Layout is pinned by the
tests in tests/abi.rs against the C++ reference dump in ../harness/abi_reference.txt
(regenerate with ../harness/build_abi_dump.ps1 after changing the headers).
*/

use std::f64::NAN;

/// DSP behavior version (SPEECHPLAYER_DSP_VERSION in src/voicingTone.h).
pub const DSP_VERSION: u32 = 9;
/// VoicingTone ABI magic "VOT2" (SPEECHPLAYER_VOICINGTONE_MAGIC).
pub const VOICINGTONE_MAGIC: u32 = 0x32544F56;
/// VoicingTone struct layout version (SPEECHPLAYER_VOICINGTONE_VERSION, V5).
pub const VOICINGTONE_VERSION: u32 = 5;

/// Number of doubles in `speechPlayer_frame_t` (the legacy 47-parameter frame).
pub const FRAME_NUM_PARAMS: usize = 47;
/// Number of doubles in `speechPlayer_frameEx_t` (DSP v5+ extensions;
/// 29 through DSP v8, plus the three DSP v9 fields).
pub const FRAMEEX_NUM_PARAMS: usize = 32;

/// `sample` — one output PCM sample (`typedef short sampleVal;` in src/sample.h).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub value: i16,
}

/// `speechPlayer_frame_t` — the 47-parameter frame, stable since DSP v1.
///
/// Field order MUST stay in sync with src/frame.h and with
/// `nvspFrontend_Frame` in src/frontend/nvspFrontend.h.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    // voicing and cascade
    pub voice_pitch: f64,
    pub vibrato_pitch_offset: f64,
    pub vibrato_speed: f64,
    pub voice_turbulence_amplitude: f64,
    pub glottal_open_quotient: f64,
    pub voice_amplitude: f64,
    pub aspiration_amplitude: f64,
    pub cf1: f64,
    pub cf2: f64,
    pub cf3: f64,
    pub cf4: f64,
    pub cf5: f64,
    pub cf6: f64,
    pub cf_n0: f64,
    pub cf_np: f64,
    pub cb1: f64,
    pub cb2: f64,
    pub cb3: f64,
    pub cb4: f64,
    pub cb5: f64,
    pub cb6: f64,
    pub cb_n0: f64,
    pub cb_np: f64,
    pub ca_np: f64,
    // fricatives and parallel
    pub frication_amplitude: f64,
    pub pf1: f64,
    pub pf2: f64,
    pub pf3: f64,
    pub pf4: f64,
    pub pf5: f64,
    pub pf6: f64,
    pub pb1: f64,
    pub pb2: f64,
    pub pb3: f64,
    pub pb4: f64,
    pub pb5: f64,
    pub pb6: f64,
    pub pa1: f64,
    pub pa2: f64,
    pub pa3: f64,
    pub pa4: f64,
    pub pa5: f64,
    pub pa6: f64,
    pub parallel_bypass: f64,
    pub pre_formant_gain: f64,
    pub output_gain: f64,
    pub end_voice_pitch: f64,
}

/// `speechPlayer_frameEx_t` — optional per-frame voice quality extensions (DSP v5+).
///
/// Defaults match `speechPlayer_frameEx_defaults` in src/frame.h: everything
/// off, formant end targets NAN (= no ramping), F7/F8 at the Rabiner 1968
/// defaults.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameEx {
    pub creakiness: f64,
    pub breathiness: f64,
    pub jitter: f64,
    pub shimmer: f64,
    pub sharpness: f64,
    pub end_cf1: f64,
    pub end_cf2: f64,
    pub end_cf3: f64,
    pub end_pf1: f64,
    pub end_pf2: f64,
    pub end_pf3: f64,
    pub fujisaki_enabled: f64,
    pub fujisaki_reset: f64,
    pub fujisaki_phrase_amp: f64,
    pub fujisaki_phrase_len: f64,
    pub fujisaki_accent_amp: f64,
    pub fujisaki_accent_dur: f64,
    pub fujisaki_accent_len: f64,
    pub trans_f1_scale: f64,
    pub trans_f2_scale: f64,
    pub trans_f3_scale: f64,
    pub trans_nasal_scale: f64,
    pub trans_amplitude_mode: f64,
    pub cf7: f64,
    pub cb7: f64,
    pub cf8: f64,
    pub cb8: f64,
    pub trans_source_hold_ratio: f64,
    pub trans_voicing_hold_ratio: f64,
    pub frication_tilt_db: f64,
    pub end_voice_amplitude: f64,
    pub amplitude_onset_ms: f64,
}

impl FrameEx {
    /// Same values as `speechPlayer_frameEx_defaults` (src/frame.h).
    pub const DEFAULTS: FrameEx = FrameEx {
        creakiness: 0.0,
        breathiness: 0.0,
        jitter: 0.0,
        shimmer: 0.0,
        sharpness: 0.0,
        end_cf1: NAN,
        end_cf2: NAN,
        end_cf3: NAN,
        end_pf1: NAN,
        end_pf2: NAN,
        end_pf3: NAN,
        fujisaki_enabled: 0.0,
        fujisaki_reset: 0.0,
        fujisaki_phrase_amp: 0.0,
        fujisaki_phrase_len: 0.0,
        fujisaki_accent_amp: 0.0,
        fujisaki_accent_dur: 0.0,
        fujisaki_accent_len: 0.0,
        trans_f1_scale: 0.0,
        trans_f2_scale: 0.0,
        trans_f3_scale: 0.0,
        trans_nasal_scale: 0.0,
        trans_amplitude_mode: 0.0,
        cf7: 6500.0,
        cb7: 720.0,
        cf8: 7500.0,
        cb8: 1250.0,
        trans_source_hold_ratio: 0.0,
        trans_voicing_hold_ratio: 0.0,
        frication_tilt_db: 0.0,
        end_voice_amplitude: NAN,
        amplitude_onset_ms: 0.0,
    };
}

impl Default for FrameEx {
    fn default() -> Self {
        FrameEx::DEFAULTS
    }
}

impl Frame {
    /// Raw parameter view — mirrors the C++ casts to `speechPlayer_frameParam_t*`.
    /// Layout as a contiguous array of 47 f64s is pinned by tests/abi.rs.
    pub fn params(&self) -> &[f64; FRAME_NUM_PARAMS] {
        unsafe { &*(self as *const Frame as *const [f64; FRAME_NUM_PARAMS]) }
    }
    pub fn params_mut(&mut self) -> &mut [f64; FRAME_NUM_PARAMS] {
        unsafe { &mut *(self as *mut Frame as *mut [f64; FRAME_NUM_PARAMS]) }
    }
    pub fn from_le_bytes(bytes: &[u8; 376]) -> Frame {
        const _: () = assert!(std::mem::size_of::<Frame>() == 376);
        unsafe { std::mem::transmute::<[u8; 376], Frame>(*bytes) }
    }
    pub fn to_le_bytes(&self) -> [u8; 376] {
        unsafe { std::mem::transmute::<Frame, [u8; 376]>(*self) }
    }
}

impl FrameEx {
    pub fn params(&self) -> &[f64; FRAMEEX_NUM_PARAMS] {
        unsafe { &*(self as *const FrameEx as *const [f64; FRAMEEX_NUM_PARAMS]) }
    }
    pub fn params_mut(&mut self) -> &mut [f64; FRAMEEX_NUM_PARAMS] {
        unsafe { &mut *(self as *mut FrameEx as *mut [f64; FRAMEEX_NUM_PARAMS]) }
    }
    pub fn from_le_bytes(bytes: &[u8; 256]) -> FrameEx {
        const _: () = assert!(std::mem::size_of::<FrameEx>() == 256);
        unsafe { std::mem::transmute::<[u8; 256], FrameEx>(*bytes) }
    }
}

/// `speechPlayer_voicingTone_t` — DSP-level voice quality parameters (V5).
///
/// The leading u32 header lets old/new callers negotiate layout size
/// (see src/voicingTone.h). Defaults match SPEECHPLAYER_VOICINGTONE_DEFAULTS.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoicingTone {
    pub magic: u32,
    pub struct_size: u32,
    pub struct_version: u32,
    pub dsp_version: u32,
    pub voicing_peak_pos: f64,
    pub voiced_pre_emph_a: f64,
    pub voiced_pre_emph_mix: f64,
    pub high_shelf_gain_db: f64,
    pub high_shelf_fc_hz: f64,
    pub high_shelf_q: f64,
    pub voiced_tilt_db_per_oct: f64,
    pub noise_glottal_mod_depth: f64,
    pub pitch_sync_f1_delta_hz: f64,
    pub pitch_sync_b1_delta_hz: f64,
    pub speed_quotient: f64,
    pub aspiration_tilt_db_per_oct: f64,
    pub cascade_bw_scale: f64,
    pub tremor_depth: f64,
    pub nasal_bw_scale: f64,
    pub f4_freq_scale: f64,
    pub nasal_gain_scale: f64,
    pub chorus_depth: f64,
    pub chorus_detune_hz: f64,
}

impl VoicingTone {
    /// Same values as SPEECHPLAYER_VOICINGTONE_DEFAULTS (src/voicingTone.h).
    pub fn defaults() -> VoicingTone {
        VoicingTone {
            magic: VOICINGTONE_MAGIC,
            struct_size: std::mem::size_of::<VoicingTone>() as u32,
            struct_version: VOICINGTONE_VERSION,
            dsp_version: DSP_VERSION,
            voicing_peak_pos: 0.91,
            voiced_pre_emph_a: 0.92,
            voiced_pre_emph_mix: 0.35,
            high_shelf_gain_db: 4.0,
            high_shelf_fc_hz: 2000.0,
            high_shelf_q: 0.7,
            voiced_tilt_db_per_oct: 0.0,
            noise_glottal_mod_depth: 0.0,
            pitch_sync_f1_delta_hz: 0.0,
            pitch_sync_b1_delta_hz: 0.0,
            speed_quotient: 2.0,
            aspiration_tilt_db_per_oct: 0.0,
            cascade_bw_scale: 0.9,
            tremor_depth: 0.0,
            nasal_bw_scale: 1.0,
            f4_freq_scale: 1.0,
            nasal_gain_scale: 1.0,
            chorus_depth: 0.0,
            chorus_detune_hz: 2.0,
        }
    }
}

impl Default for VoicingTone {
    fn default() -> Self {
        VoicingTone::defaults()
    }
}

#[cfg(test)]
mod layout {
    use super::*;

    // repr(C) with uniform alignment makes these trivially true today; they are
    // here to fail loudly if a field is ever added or reordered by mistake.
    #[test]
    fn frame_layout_is_contiguous_f64s() {
        assert_eq!(std::mem::size_of::<Frame>(), FRAME_NUM_PARAMS * 8);
        assert_eq!(std::mem::align_of::<Frame>(), 8);
        assert_eq!(std::mem::offset_of!(Frame, voice_pitch), 0);
        assert_eq!(
            std::mem::offset_of!(Frame, end_voice_pitch),
            (FRAME_NUM_PARAMS - 1) * 8
        );
    }

    #[test]
    fn frameex_layout_is_contiguous_f64s() {
        assert_eq!(std::mem::size_of::<FrameEx>(), FRAMEEX_NUM_PARAMS * 8);
        assert_eq!(std::mem::align_of::<FrameEx>(), 8);
    }

    #[test]
    fn frameex_defaults_match_c() {
        let d = FrameEx::DEFAULTS;
        assert_eq!(d.creakiness, 0.0);
        assert!(d.end_cf1.is_nan() && d.end_pf3.is_nan());
        assert_eq!(d.cf7, 6500.0);
        assert_eq!(d.cb7, 720.0);
        assert_eq!(d.cf8, 7500.0);
        assert_eq!(d.cb8, 1250.0);
        assert_eq!(d.trans_amplitude_mode, 0.0);
        assert_eq!(d.trans_voicing_hold_ratio, 0.0);
    }

    #[test]
    fn voicing_tone_layout_and_defaults() {
        // 4 u32 header (16 bytes) + 19 doubles
        assert_eq!(std::mem::size_of::<VoicingTone>(), 16 + 19 * 8);
        assert_eq!(std::mem::align_of::<VoicingTone>(), 8);
        assert_eq!(std::mem::offset_of!(VoicingTone, voicing_peak_pos), 16);

        let t = VoicingTone::defaults();
        assert_eq!(t.magic, VOICINGTONE_MAGIC);
        assert_eq!(t.struct_size, 168);
        assert_eq!(t.struct_version, VOICINGTONE_VERSION);
        assert_eq!(t.dsp_version, DSP_VERSION);
        assert_eq!(t.voicing_peak_pos, 0.91);
        assert_eq!(t.cascade_bw_scale, 0.9);
        assert_eq!(t.chorus_detune_hz, 2.0);
    }

    #[test]
    fn sample_is_two_bytes() {
        assert_eq!(std::mem::size_of::<Sample>(), 2);
    }
}
