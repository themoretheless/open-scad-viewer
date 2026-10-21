    use super::*;

    fn descriptors(seed: u64, n: usize) -> Vec<[f32; 128]> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                let mut d = [0f32; 128];
                for v in &mut d {
                    state = state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    *v = ((state >> 33) as f32 / 2147483648.0).fract();
                }
                d
            })
            .collect()
    }

    fn cpu_reference(
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> (Vec<(usize, f32, f32)>, Vec<(usize, f32)>) {
        let mut best_a = vec![(usize::MAX, f32::INFINITY, f32::INFINITY); da.len()];
        let mut best_b = vec![(usize::MAX, f32::INFINITY); db.len()];
        for (i, x) in da.iter().enumerate() {
            for (j, y) in db.iter().enumerate() {
                let mut d = 0f32;
                for k in 0..128 {
                    let v = x[k] - y[k];
                    d += v * v;
                }
                if d < best_a[i].1 {
                    best_a[i].2 = best_a[i].1;
                    best_a[i].1 = d;
                    best_a[i].0 = j;
                } else if d < best_a[i].2 {
                    best_a[i].2 = d;
                }
                if d < best_b[j].1 {
                    best_b[j] = (i, d);
                }
            }
        }
        (best_a, best_b)
    }

    #[test]
    fn gpu_matching_reuses_staging_and_handles_empty_grids_after_nonempty_calls() {
        let Some(context) = GpuContext::new() else {
            assert!(std::env::var_os("COMPUTE_REQUIRE_GPU").is_none());
            return;
        };
        let matcher = GpuMatcher::new(&context);
        for count in [9, 3, 12, 1] {
            let da = descriptors(17, count);
            let db = descriptors(31, count + 2);
            let (rows, cols) = matcher.try_match_descriptors(&da, &db).unwrap();
            let (expected_rows, expected_cols) = cpu_reference(&da, &db);
            assert_eq!(rows.len(), da.len());
            assert_eq!(cols.len(), db.len());
            for (actual, expected) in rows.iter().zip(expected_rows) {
                assert_eq!(actual.j, expected.0);
                assert!((actual.d1 - expected.1).abs() <= 1e-4 * expected.1.max(1.));
            }
            for (actual, expected) in cols.iter().zip(expected_cols) {
                assert_eq!(actual.i, expected.0);
                assert!((actual.d1 - expected.1).abs() <= 1e-4 * expected.1.max(1.));
            }
            let (rows, cols) = matcher.try_match_descriptors(&da, &[]).unwrap();
            assert!(cols.is_empty());
            assert_eq!(rows.len(), da.len());
            assert!(
                rows.iter()
                    .all(|r| r.j == usize::MAX && r.d1 == f32::INFINITY && r.d2 == f32::INFINITY)
            );
            let (rows, cols) = matcher.try_match_descriptors(&[], &db).unwrap();
            assert!(rows.is_empty());
            assert_eq!(cols.len(), db.len());
            assert!(
                cols.iter()
                    .all(|c| c.i == usize::MAX && c.d1 == f32::INFINITY)
            );
        }
    }

    #[test]
    fn gpu_matching_agrees_with_cpu_on_synthetic_descriptors() {
        let Some((da, db)) = match_pair(&descriptors(7, 300), &descriptors(11, 280))
            .map(|_| (descriptors(7, 300), descriptors(11, 280)))
        else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let (rows, cols) = match_pair(&da, &db).expect("adapter was available");
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        let mut row_exact = 0;
        let mut row_close = 0;
        for (gpu, cpu) in rows.iter().zip(&ref_a) {
            if gpu.j == cpu.0
                && gpu.d1.to_bits() == cpu.1.to_bits()
                && gpu.d2.to_bits() == cpu.2.to_bits()
            {
                row_exact += 1;
            }
            // GPU float contraction may shift borderline distances; the selected
            // indices must agree and distances must stay within f32 contraction noise.
            if gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.) {
                row_close += 1;
            }
        }
        let col_exact = cols
            .iter()
            .zip(&ref_b)
            .filter(|(gpu, cpu)| gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        assert_eq!(row_close, ref_a.len(), "row selections must agree with CPU");
        assert_eq!(
            col_exact,
            ref_b.len(),
            "column selections must agree with CPU"
        );
        eprintln!("bit-exact rows: {row_exact}/{}", ref_a.len());
    }

    #[test]
    fn gpu_matcher_picks_the_backend_tuned_workgroup_size() {
        let Some(context) = GpuContext::new() else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let matcher = GpuMatcher::new(&context);
        let expected = gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        assert_eq!(matcher.workgroup_size(), expected);
        if context.backend == wgpu::Backend::Metal {
            assert_eq!(matcher.workgroup_size(), WG_METAL);
        } else {
            assert_eq!(matcher.workgroup_size(), WG_DEFAULT);
        }
    }

    /// Both the Metal-tuned (128) and default (256) workgroup variants must
    /// produce results that agree with the CPU reference, regardless of
    /// which one the test machine's actual backend happens to select — this
    /// is what protects the templated shader from a size-specific bug.
    #[test]
    fn gpu_matching_agrees_with_cpu_for_both_workgroup_tunings() {
        let Some(context) = GpuContext::new() else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let da = descriptors(23, 300);
        let db = descriptors(29, 280);
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        for &wg in &[WG_METAL, WG_DEFAULT] {
            let matcher = GpuMatcher::with_workgroup_size(&context, wg);
            let (rows, cols) = matcher.match_descriptors(&da, &db);
            let row_close = rows
                .iter()
                .zip(&ref_a)
                .filter(|(gpu, cpu)| {
                    gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.)
                })
                .count();
            let col_close = cols
                .iter()
                .zip(&ref_b)
                .filter(|(gpu, cpu)| {
                    gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.)
                })
                .count();
            assert_eq!(
                row_close,
                ref_a.len(),
                "WG={wg}: row selections must agree with CPU"
            );
            assert_eq!(
                col_close,
                ref_b.len(),
                "WG={wg}: column selections must agree with CPU"
            );
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_matching_agrees_with_cpu_on_synthetic_descriptors() {
        let da = descriptors(31, 300);
        let db = descriptors(37, 280);
        let Some((rows, cols)) = match_pair_accelerated(&da, &db, crate::Acceleration::Cuda) else {
            eprintln!("no CUDA/wgpu adapter; skipping");
            return;
        };
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        let row_close = rows
            .iter()
            .zip(&ref_a)
            .filter(|(gpu, cpu)| gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        let col_close = cols
            .iter()
            .zip(&ref_b)
            .filter(|(gpu, cpu)| gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        assert_eq!(row_close, ref_a.len(), "row selections must agree with CPU");
        assert_eq!(
            col_close,
            ref_b.len(),
            "column selections must agree with CPU"
        );
    }
