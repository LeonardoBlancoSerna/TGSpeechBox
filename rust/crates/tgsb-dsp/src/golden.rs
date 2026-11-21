/*
TGSpeechBox Rust port — golden corpus loader and renderer.
Ported from rust/harness/golden_dump.cpp (MIT).
*/

use std::fs;
use std::path::{Path, PathBuf};

use tgsb_frame::{Frame, FrameEx, Sample, VoicingTone};

use crate::SpeechWaveGenerator;

const GOLDEN_MAGIC: &[u8; 8] = b"TGSBF01\0";
const CHUNK: usize = 2048;

#[derive(Clone, Debug)]
pub struct GoldenFrame {
    pub has_frame: bool,
    pub has_ex: bool,
    pub duration_ms: f64,
    pub fade_ms: f64,
    pub user_index: i32,
    pub frame: Option<Frame>,
    pub ex: Option<FrameEx>,
}

#[derive(Clone, Debug)]
pub struct GoldenCase {
    pub name: String,
    pub lang: String,
    pub speed: f64,
    pub sample_rate: i32,
    pub time_stretch: f64,
    pub voicing_tone: bool,
    pub frames: Vec<GoldenFrame>,
    pub pcm: Vec<i16>,
}

#[derive(Clone, Debug)]
pub struct ManifestRow {
    pub name: String,
    pub lang: String,
    pub speed: f64,
    pub sample_rate: i32,
    pub time_stretch: f64,
    pub voicing_tone: bool,
    pub frame_count: usize,
    pub pcm_samples: usize,
}

pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/golden")
}

pub fn load_manifest(root: &Path) -> Vec<ManifestRow> {
    let text = fs::read_to_string(root.join("manifest.tsv")).expect("read manifest.tsv");
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        if line.is_empty() { continue; }
        let f: Vec<&str> = line.split('\t').collect();
        assert!(f.len() == 8, "bad manifest row: {line}");
        rows.push(ManifestRow {
            name: f[0].to_owned(),
            lang: f[1].to_owned(),
            speed: f[2].parse().unwrap(),
            sample_rate: f[3].parse().unwrap(),
            time_stretch: f[4].parse().unwrap(),
            voicing_tone: f[5] == "1",
            frame_count: f[6].parse().unwrap(),
            pcm_samples: f[7].parse().unwrap(),
        });
    }
    rows
}

pub fn load_case(root: &Path, row: &ManifestRow) -> GoldenCase {
    let frames = load_frames(&root.join(format!("{}.frames", row.name)));
    let pcm_bytes = fs::read(root.join(format!("{}.pcm", row.name))).expect("read golden pcm");
    assert_eq!(pcm_bytes.len() % 2, 0);
    let pcm = pcm_bytes.chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    GoldenCase {
        name: row.name.clone(),
        lang: row.lang.clone(),
        speed: row.speed,
        sample_rate: row.sample_rate,
        time_stretch: row.time_stretch,
        voicing_tone: row.voicing_tone,
        frames,
        pcm,
    }
}

fn load_frames(path: &Path) -> Vec<GoldenFrame> {
    let b = fs::read(path).expect("read golden frames");
    assert!(b.len() >= 20, "truncated golden frames: {path:?}");
    assert_eq!(&b[0..8], GOLDEN_MAGIC, "bad golden frames magic: {path:?}");
    let count = u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize;
    let frame_params = u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as usize;
    let ex_params = u32::from_le_bytes([b[16], b[17], b[18], b[19]]) as usize;
    assert_eq!(frame_params, 47);
    assert_eq!(ex_params, 32);

    let mut at = 20;
    let mut frames = Vec::with_capacity(count);
    for _ in 0..count {
        let flags = b[at]; at += 1;
        let duration_ms = f64::from_le_bytes(take8(&b, &mut at));
        let fade_ms = f64::from_le_bytes(take8(&b, &mut at));
        let user_index = i32::from_le_bytes(take4(&b, &mut at));
        let frame = if flags & 1 != 0 {
            let mut bytes = [0u8; 376];
            bytes.copy_from_slice(&b[at..at + 376]); at += 376;
            Some(Frame::from_le_bytes(&bytes))
        } else {
            None
        };
        let ex = if flags & 2 != 0 {
            let mut bytes = [0u8; 256];
            bytes.copy_from_slice(&b[at..at + 256]); at += 256;
            Some(FrameEx::from_le_bytes(&bytes))
        } else {
            None
        };
        frames.push(GoldenFrame { has_frame: frame.is_some(), has_ex: ex.is_some(), duration_ms, fade_ms, user_index, frame, ex });
    }
    assert_eq!(at, b.len(), "trailing bytes in {path:?}");
    frames
}

fn take4(b: &[u8], at: &mut usize) -> [u8; 4] {
    let out = [b[*at], b[*at + 1], b[*at + 2], b[*at + 3]];
    *at += 4;
    out
}

fn take8(b: &[u8], at: &mut usize) -> [u8; 8] {
    let out = [
        b[*at], b[*at + 1], b[*at + 2], b[*at + 3],
        b[*at + 4], b[*at + 5], b[*at + 6], b[*at + 7],
    ];
    *at += 8;
    out
}

fn ms_to_samples(ms: f64, sample_rate: i32) -> u32 {
    if ms <= 0.0 { return 0; }
    let s = (ms * sample_rate as f64) / 1000.0;
    if s <= 0.0 { return 0; }
    (s + 0.5) as u32
}

impl GoldenCase {
    pub fn render(&self) -> Vec<i16> {
        let mut gen = SpeechWaveGenerator::new(self.sample_rate);
        if self.time_stretch != 1.0 {
            gen.set_time_stretch(self.time_stretch);
        }
        if self.voicing_tone {
            gen.set_voicing_tone(VoicingTone::defaults());
        }
        for rec in &self.frames {
            let min_samples = ms_to_samples(rec.duration_ms, self.sample_rate);
            let fade_samples = ms_to_samples(rec.fade_ms, self.sample_rate);
            gen.queue_frame(
                rec.frame,
                rec.ex,
                min_samples,
                fade_samples,
                rec.user_index,
                false,
            );
        }
        let mut out = Vec::new();
        let mut buf = vec![Sample { value: 0 }; CHUNK];
        loop {
            let n = gen.generate(CHUNK, &mut buf);
            out.extend(buf[..n].iter().map(|s| s.value));
            if n < CHUNK { break; }
        }
        out
    }
}
