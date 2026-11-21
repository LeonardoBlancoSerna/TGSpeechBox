use tgsb_dsp::golden::{golden_dir, load_case, load_manifest};

#[test]
fn golden_dsp_pcm_matches_cpp_reference() {
    let root = golden_dir();
    let rows = load_manifest(&root);
    assert_eq!(rows.len(), 12);

    let mut failures = Vec::new();
    for row in &rows {
        let case = load_case(&root, row);
        assert_eq!(case.frames.len(), row.frame_count, "{} frame count", row.name);
        assert_eq!(case.pcm.len(), row.pcm_samples, "{} pcm count", row.name);

        let actual = case.render();
        let expected = &case.pcm;
        let mut max_diff = 0i32;
        let mut bad = 0usize;
        let mut first_bad = None;
        let len = actual.len().max(expected.len());
        for i in 0..len {
            let a = *actual.get(i).unwrap_or(&0) as i32;
            let e = *expected.get(i).unwrap_or(&0) as i32;
            let d = (a - e).abs();
            if d > max_diff { max_diff = d; }
            if d > 4 {
                bad += 1;
                if first_bad.is_none() {
                    first_bad = Some((i, a, e));
                }
            }
        }
        if bad > 0 || actual.len() != expected.len() {
            let first = match first_bad {
                Some((i, a, e)) => format!(" first_bad[{}] actual={} expected={}", i, a, e),
                None => String::new(),
            };
            failures.push(format!(
                "{}: len {}/{} max_diff {} bad {}{}",
                row.name, actual.len(), expected.len(), max_diff, bad, first
            ));
        } else {
            eprintln!("{}: max_diff {}", row.name, max_diff);
        }
    }
    assert!(failures.is_empty(), "golden mismatches:\n{}", failures.join("\n"));
}
