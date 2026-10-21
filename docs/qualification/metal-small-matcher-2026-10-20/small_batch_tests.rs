use super::*;
use std::{
    hint::black_box,
    time::{Duration, Instant},
};
fn inputs(n: usize, salt: usize) -> Vec<[f32; 128]> {
    (0..n)
        .map(|i| std::array::from_fn(|j| ((i * 13 + j * 7 + salt) % 31) as f32 * 0.25))
        .collect()
}
fn baseline(c: &GpuContext) -> GpuMatcher {
    let mut m = GpuMatcher::new(c);
    m.disable_small_updates = true;
    m
}
fn bits(out: &(Vec<RowBest>, Vec<ColBest>)) -> (Vec<(usize, u32, u32)>, Vec<(usize, u32)>) {
    (
        out.0
            .iter()
            .map(|r| (r.j, r.d1.to_bits(), r.d2.to_bits()))
            .collect(),
        out.1.iter().map(|r| (r.i, r.d1.to_bits())).collect(),
    )
}
#[test]
fn small_batch_matches_separate_inputs_across_reuse_growth_and_fallback() {
    let c = GpuContext::new();
    assert!(c.is_some() || std::env::var_os("COMPUTE_REQUIRE_GPU").is_none());
    let Some(c) = c else { return };
    let a = baseline(&c);
    let b = GpuMatcher::new(&c);
    for (rows, cols) in [
        (1, 1),
        (3, 4),
        (1, 2),
        (2, 1),
        (4, 4),
        (1, 1),
        (0, 2),
        (8, 16),
    ] {
        let da = inputs(rows, 3);
        let db = inputs(cols, 11);
        assert_eq!(
            bits(&a.match_descriptors(&da, &db)),
            bits(&b.match_descriptors(&da, &db))
        );
        if rows > 0 && cols > 0 {
            let small = b.buffers.borrow();
            let small = small.as_ref().unwrap();
            assert_eq!(
                small.small_input.is_some(),
                (small.row_capacity + small.col_capacity) * 512
                    + c.device
                        .limits()
                        .min_storage_buffer_offset_alignment
                        .max(16) as usize
                    <= 4096
            );
        }
    }
}
#[test]
#[ignore = "paired full-workflow qualification; requires idle own builds and output path"]
fn paired_full_matcher() {
    #[cfg(target_os = "macos")]
    unsafe {
        unsafe extern "C" {
            fn pthread_set_qos_class_self_np(q: u32, p: i32) -> i32;
        }
        assert_eq!(pthread_set_qos_class_self_np(0x19, 0), 0);
    }
    let c = GpuContext::new().expect("GPU required");
    assert_eq!(c.backend, wgpu::Backend::Metal);
    let base = baseline(&c);
    let base2 = baseline(&c);
    let candidate = GpuMatcher::new(&c);
    let mut csv = String::from("case,rows,cols,round,a_ns,b_ns\n");
    for (rows, cols) in [(1, 1), (2, 2), (3, 4), (4, 3), (4, 4), (16, 16)] {
        let da = inputs(rows, 3);
        let db = inputs(cols, 11);
        let reference = bits(&base.match_descriptors(&da, &db));
        assert_eq!(reference, bits(&candidate.match_descriptors(&da, &db)));
        assert_eq!(reference, bits(&base2.match_descriptors(&da, &db)));
        for control in [false, true] {
            let other = if control { &base2 } else { &candidate };
            let sample = |b: bool| {
                let m = if b { other } else { &base };
                let start = Instant::now();
                for _ in 0..16 {
                    black_box(
                        m.try_match_descriptors(black_box(&da), black_box(&db))
                            .unwrap(),
                    );
                }
                start.elapsed().as_nanos() as f64 / 16.
            };
            let warm = Instant::now();
            while warm.elapsed() < Duration::from_millis(150) {
                sample(false);
                sample(true);
            }
            for round in 0..32 {
                let order = if round % 2 == 0 {
                    [false, true, true, false]
                } else {
                    [true, false, false, true]
                };
                let s = order.map(sample);
                let (a, b) = if round % 2 == 0 {
                    ((s[0] + s[3]) / 2., (s[1] + s[2]) / 2.)
                } else {
                    ((s[1] + s[2]) / 2., (s[0] + s[3]) / 2.)
                };
                csv.push_str(&format!(
                    "{},{rows},{cols},{round},{a},{b}\n",
                    if control { "aa" } else { "batch" }
                ));
            }
        }
    }
    let path = std::env::var("METAL_SMALL_MATCHER_CSV").expect("explicit output path required");
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap()
        .write_all(csv.as_bytes())
        .unwrap();
}
