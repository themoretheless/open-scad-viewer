use super::*;

#[test]
fn host_sweep_linear_indexing_variant_rewrites_entry_dispatch_shape() {
    let wgsl = host_sweep_linear_indexing_wgsl();
    assert!(wgsl.starts_with("requires linear_indexing;"));
    assert!(wgsl.contains("@compute @workgroup_size(256)"));
    assert!(wgsl.contains("fn sweep(@builtin(global_invocation_index) index: u32)"));
    assert!(wgsl.contains("let total = params.width * params.height;"));
    assert!(wgsl.contains("let x = index % params.width;"));
    assert!(wgsl.contains("let y = index / params.width;"));
    assert!(!wgsl.contains("fn sweep(@builtin(global_invocation_id) id: vec3<u32>)"));
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_sweep_matches_cpu_on_shifted_plane() {
    // Reference texture 80x80; a source camera translated -0.5 along x sees
    // the plane z=4 shifted left by f*B/z = 80*0.5/4 = 10 px.
    let mut rng = 0xABCDEFu64;
    let mut tex = vec![0f32; 80 * 80];
    for v in &mut tex {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *v = (rng >> 33) as f32 / 2147483648.0;
    }
    let mut src_values = vec![0f32; 80 * 80];
    for y in 0..80 {
        for x in 0..70 {
            src_values[y * 80 + x] = tex[y * 80 + x + 10];
        }
    }
    let ref_image = Image {
        width: 80,
        height: 80,
        rgb: vec![],
        focal: 80.,
    };
    let _ = &ref_image;
    let reference = Camera::identity(80., 40., 40.);
    let source_camera = Camera {
        rotation: ID,
        translation: [-0.5, 0., 0.],
        focal: 80.,
        cx: 40.,
        cy: 40.,
    };
    let ref_gray = GrayImage {
        width: 80,
        height: 80,
        values: tex,
    };
    let src_gray = GrayImage {
        width: 80,
        height: 80,
        values: src_values,
    };
    let build = |step: f64| {
        vec![Source {
            index: 1,
            image: &src_gray,
            camera: &source_camera,
            rotation: ID,
            translation: [-0.5, 0., 0.],
            offsets: std::array::from_fn(|i| {
                [
                    ((i % 3) as f64 - 1.) * step / 80.,
                    ((i / 3) as f64 - 1.) * step / 80.,
                    0.,
                ]
            }),
            rays: [[0.; 3]; 25],
        }]
    };
    let options = |acceleration| DenseOptions {
        acceleration,
        depth_hypotheses: 16,
        patch_radius: 1,
        min_support_views: 1,
        ..Default::default()
    };
    let run = |acceleration| {
        let mut sources = build(2.);
        let mut diagnostics = DenseDiagnostics::default();
        sweep_depth(
            &ref_gray,
            &reference,
            &mut sources,
            40,
            40,
            2.,
            2.,
            8.,
            None,
            &options(acceleration),
            &mut diagnostics,
            &mut |_| Ok(()),
        )
        .unwrap()
        .0
    };
    let cpu = run(crate::Acceleration::Cpu);
    if crate::gpu::sweep::shared().is_none() {
        eprintln!("no GPU adapter; skipping");
        return;
    }
    let gpu = run(crate::Acceleration::Gpu);
    let mut valid = 0;
    let mut agree = 0;
    for i in 0..cpu.len() {
        if cpu[i] > 0. {
            valid += 1;
            if (gpu[i] - cpu[i]).abs() <= 0.15 {
                agree += 1;
            }
        }
    }
    eprintln!(
        "valid={valid} agree={agree} cpu[820]={} gpu[820]={}",
        cpu[820], gpu[820]
    );
    assert!(valid > 400, "plane should be mostly valid: {valid}");
    assert!(
        agree as f64 >= 0.95 * valid as f64,
        "GPU depth rows disagree: {agree}/{valid}"
    );
}

#[cfg(feature = "gpu")]
#[test]
fn host_sweep_split_matches_cpu_sweep() {
    // Same shifted-plane scene as the GPU sweep test: prepare the host
    // payloads, score them natively with the same shader, finish, and the
    // depth map must match the CPU sweep.
    let mut rng = 0xABCDEFu64;
    let mut tex = vec![0f32; 80 * 80];
    for v in &mut tex {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *v = (rng >> 33) as f32 / 2147483648.0;
    }
    let mut src_values = vec![0f32; 80 * 80];
    for y in 0..80 {
        for x in 0..70 {
            src_values[y * 80 + x] = tex[y * 80 + x + 10];
        }
    }
    let reference = Camera::identity(80., 40., 40.);
    let source_camera = Camera {
        rotation: ID,
        translation: [-0.5, 0., 0.],
        focal: 80.,
        cx: 40.,
        cy: 40.,
    };
    let gray = [
        Some(GrayImage {
            width: 80,
            height: 80,
            values: tex,
        }),
        Some(GrayImage {
            width: 80,
            height: 80,
            values: src_values,
        }),
    ];
    let point = |x, y, z| crate::Point {
        position: [x, y, z],
        color: [0; 3],
        observations: vec![(0, 0), (1, 0)],
    };
    let mut points = Vec::new();
    for i in 0..12 {
        let x = (i % 4) as f64 - 1.5;
        let y = (i / 4) as f64 - 1.;
        points.push(point(x * 0.4, y * 0.4, 3.5 + i as f64 * 0.05));
    }
    let sparse = Reconstruction {
        cameras: vec![Some(reference.clone()), Some(source_camera)],
        points,
        input_images: 2,
        reprojection_rmse: 0.,
    };
    let images = [
        Image {
            width: 80,
            height: 80,
            rgb: vec![],
            focal: 80.,
        },
        Image {
            width: 80,
            height: 80,
            rgb: vec![],
            focal: 80.,
        },
    ];
    let options = DenseOptions {
        depth_hypotheses: 16,
        patch_radius: 1,
        min_support_views: 1,
        max_side: 64,
        ..Default::default()
    };
    let (views, prepared) =
        prepare_host_views(&images, &sparse, &options, &mut |_, _, _| true).unwrap();
    assert!(views[0].is_some(), "view 0 prepared");
    let Some(sweep) = crate::gpu::sweep::shared() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let rasters: Vec<Option<(&[f32], usize, usize)>> = gray
        .iter()
        .map(|g| g.as_ref().map(|g| (g.values.as_slice(), g.width, g.height)))
        .collect();
    let atlas = sweep.upload_grays(&rasters);
    // The browser-shared `sweep` entry: raw score rows per view.
    let run_scores = |view: &HostSweepView| -> Option<Vec<f32>> {
        let job = crate::gpu::sweep::SweepJob {
            ref_image: view.ref_image,
            ref_focal: view.ref_focal as f32,
            ref_cx: view.ref_cx as f32,
            ref_cy: view.ref_cy as f32,
            sources: view
                .sources
                .iter()
                .map(|s| crate::gpu::sweep::SourcePayload {
                    image: s.image,
                    rotation: s.rotation.map(|row| row.map(|v| v as f32)),
                    translation: s.translation.map(|v| v as f32),
                    focal: s.focal as f32,
                    cx: s.cx as f32,
                    cy: s.cy as f32,
                })
                .collect(),
            hypotheses: view.hypotheses.iter().map(|&z| z as f32).collect(),
            width: view.map_width,
            height: view.map_height,
            patch_radius: view.patch_radius,
            step: view.step as f32,
            needed: view.needed,
            bins: vec![0; view.map_width * view.map_height * 2],
            min_correlation: 0.,
            uniqueness_margin: 0.,
        };
        let mut batch = sweep.batch(&atlas);
        batch.push_scores(&job);
        batch.try_finish_scores().ok()?.pop()
    };
    let scores: Vec<Option<Vec<f32>>> = views
        .iter()
        .map(|v| v.as_ref().and_then(run_scores))
        .collect();
    let (maps, _) = finish_host_views(
        &images,
        &sparse,
        &options,
        &gray,
        &prepared,
        &scores,
        &mut |_, _, _| true,
    )
    .unwrap();
    // CPU reference sweep for the same view.
    let mut sources = vec![Source {
        index: 1,
        image: gray[1].as_ref().unwrap(),
        camera: &sparse.cameras[1].as_ref().unwrap(),
        rotation: ID,
        translation: [-0.5, 0., 0.],
        offsets: std::array::from_fn(|i| {
            [
                ((i % 3) as f64 - 1.) * 1.25 / 80.,
                ((i / 3) as f64 - 1.) * 1.25 / 80.,
                0.,
            ]
        }),
        rays: [[0.; 3]; 25],
    }];
    let mut diagnostics = DenseDiagnostics::default();
    let (cpu_depth, _) = sweep_depth(
        gray[0].as_ref().unwrap(),
        &reference,
        &mut sources,
        64,
        64,
        1.25,
        3.5 * 0.85,
        4.05 * 1.15,
        None,
        &options,
        &mut diagnostics,
        &mut |_| Ok(()),
    )
    .unwrap();
    let host_depth = &maps[0].depth;
    let mut valid = 0;
    let mut agree = 0;
    for i in 0..cpu_depth.len() {
        if cpu_depth[i] > 0. {
            valid += 1;
            if (host_depth[i] - cpu_depth[i]).abs() <= 0.15 {
                agree += 1;
            }
        }
    }
    assert!(valid > 400, "plane should be mostly valid: {valid}");
    assert!(
        agree as f64 >= 0.95 * valid as f64,
        "host split disagrees: {agree}/{valid}"
    );
}

/// Shifted-plane scene shared by the CUDA parity tests: a random reference
/// texture and the same texture shifted 10 px left, which a camera
/// translated -0.5 along x sees for the plane z = 4 (f*B/z = 80*0.5/4).
#[cfg(feature = "cuda")]
fn shifted_plane() -> (GrayImage, GrayImage, Camera, Camera) {
    let mut rng = 0xABCDEFu64;
    let mut tex = vec![0f32; 80 * 80];
    for v in &mut tex {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *v = (rng >> 33) as f32 / 2147483648.0;
    }
    let mut src_values = vec![0f32; 80 * 80];
    for y in 0..80 {
        for x in 0..70 {
            src_values[y * 80 + x] = tex[y * 80 + x + 10];
        }
    }
    (
        GrayImage {
            width: 80,
            height: 80,
            values: tex,
        },
        GrayImage {
            width: 80,
            height: 80,
            values: src_values,
        },
        Camera::identity(80., 40., 40.),
        Camera {
            rotation: ID,
            translation: [-0.5, 0., 0.],
            focal: 80.,
            cx: 40.,
            cy: 40.,
        },
    )
}

#[cfg(feature = "cuda")]
fn shifted_plane_source<'a>(
    image: &'a GrayImage,
    camera: &'a Camera,
    step: f64,
) -> Vec<Source<'a>> {
    vec![Source {
        index: 1,
        image,
        camera,
        rotation: ID,
        translation: [-0.5, 0., 0.],
        offsets: std::array::from_fn(|i| {
            [
                ((i % 3) as f64 - 1.) * step / 80.,
                ((i / 3) as f64 - 1.) * step / 80.,
                0.,
            ]
        }),
        rays: [[0.; 3]; 25],
    }]
}

/// The native CUDA sweep must reproduce the CPU depth map as closely as the
/// wgpu sweep does (same f32 kernel semantics, same tolerance).
#[cfg(feature = "cuda")]
#[test]
fn cuda_sweep_matches_cpu_on_shifted_plane() {
    let (ref_gray, src_gray, reference, source_camera) = shifted_plane();
    let options = |acceleration| DenseOptions {
        acceleration,
        depth_hypotheses: 16,
        patch_radius: 1,
        min_support_views: 1,
        ..Default::default()
    };
    let run = |acceleration| {
        let mut sources = shifted_plane_source(&src_gray, &source_camera, 2.);
        let mut diagnostics = DenseDiagnostics::default();
        sweep_depth(
            &ref_gray,
            &reference,
            &mut sources,
            40,
            40,
            2.,
            2.,
            8.,
            None,
            &options(acceleration),
            &mut diagnostics,
            &mut |_| Ok(()),
        )
        .unwrap()
        .0
    };
    let cpu = run(crate::Acceleration::Cpu);
    if crate::gpu::sweep::cuda::shared().is_none() {
        eprintln!("no CUDA device; skipping");
        return;
    }
    let cuda = run(crate::Acceleration::Cuda);
    let mut valid = 0;
    let mut agree = 0;
    for i in 0..cpu.len() {
        if cpu[i] > 0. {
            valid += 1;
            if (cuda[i] - cpu[i]).abs() <= 0.15 {
                agree += 1;
            }
        }
    }
    assert!(valid > 400, "plane should be mostly valid: {valid}");
    assert!(
        agree as f64 >= 0.95 * valid as f64,
        "CUDA depth rows disagree with CPU: {agree}/{valid}"
    );
}

/// Both device backends run the same sweep-and-select text, so their
/// per-pixel selections must agree bin for bin (f32 contraction only moves
/// borderline scores) and unsupported option sets must decline natively.
#[cfg(feature = "cuda")]
#[test]
fn cuda_and_wgpu_sweep_selections_agree() {
    let (ref_gray, src_gray, reference, source_camera) = shifted_plane();
    let sources = shifted_plane_source(&src_gray, &source_camera, 2.);
    let options = DenseOptions {
        acceleration: crate::Acceleration::Cuda,
        depth_hypotheses: 16,
        patch_radius: 1,
        min_support_views: 1,
        ..Default::default()
    };
    let (near, far) = (2., 8.);
    let hypotheses = hypothesis_grid(near, far, options.depth_hypotheses);
    let job = gpu_job(
        0,
        &reference,
        &sources,
        40,
        40,
        2.,
        &hypotheses[..options.depth_hypotheses],
        None,
        near,
        1. / near - 1. / far,
        &options,
    );
    let rasters: Vec<Option<(&[f32], usize, usize)>> = vec![
        Some((ref_gray.values.as_slice(), ref_gray.width, ref_gray.height)),
        Some((src_gray.values.as_slice(), src_gray.width, src_gray.height)),
    ];
    let Some(cuda) = crate::gpu::sweep::cuda::shared() else {
        eprintln!("no CUDA device; skipping");
        return;
    };
    let cuda_atlas = cuda.upload_grays(&rasters).unwrap();
    let cuda_selection = cuda.run(&cuda_atlas, &job).unwrap();
    // Patch radius 3 (49 taps) exceeds the kernel's 25-tap capacity: the
    // native path declines instead of approximating the shader.
    let wide = DenseOptions {
        patch_radius: 3,
        ..options.clone()
    };
    let wide_job = gpu_job(
        0,
        &reference,
        &sources,
        40,
        40,
        2.,
        &hypotheses[..options.depth_hypotheses],
        None,
        near,
        1. / near - 1. / far,
        &wide,
    );
    assert!(
        cuda.run(&cuda_atlas, &wide_job).is_none(),
        "oversized patches must fall back"
    );
    let Some(sweep) = crate::gpu::sweep::shared() else {
        eprintln!("no wgpu adapter; skipping the cross-backend comparison");
        return;
    };
    let atlas = sweep.upload_grays(&rasters);
    let mut batch = sweep.batch(&atlas);
    batch.push(&job);
    let wgpu_selection = batch.finish().pop().unwrap();
    assert_eq!(cuda_selection.len(), wgpu_selection.len());
    let mut valid = 0;
    let mut agree = 0;
    for (cuda_pixel, wgpu_pixel) in cuda_selection.iter().zip(&wgpu_selection) {
        match (cuda_pixel, wgpu_pixel) {
            (Some(a), Some(b)) => {
                valid += 1;
                if a.bin == b.bin && (a.score - b.score).abs() <= 1e-3 {
                    agree += 1;
                }
            }
            (None, None) => {}
            _ => valid += 1,
        }
    }
    assert!(valid > 400, "plane should be mostly valid: {valid}");
    assert!(
        agree as f64 >= 0.98 * valid as f64,
        "CUDA and wgpu selections disagree: {agree}/{valid}"
    );
}

#[test]
fn sparse_intervals_cover_anchors_deterministically() {
    let camera = Camera::identity(100., 32., 32.);
    let point = |x, y, z, views: Vec<(usize, usize)>| crate::Point {
        position: [x, y, z],
        color: [0; 3],
        observations: views,
    };
    let sparse = Reconstruction {
        cameras: vec![Some(camera.clone())],
        points: vec![
            point(-0.5, -0.5, 4., vec![(0, 0)]),
            point(0.5, 0.5, 5., vec![(0, 1)]),
            // Not observed by the reference view: never splatted.
            point(0., 0., 2., vec![(1, 0)]),
        ],
        input_images: 1,
        reprojection_rmse: 0.,
    };
    assert!(sparse_intervals(&sparse, 2, &camera, 64, 64, 1., 3., 6.).is_none());
    let a = sparse_intervals(&sparse, 0, &camera, 64, 64, 1., 3., 6.).unwrap();
    let b = sparse_intervals(&sparse, 0, &camera, 64, 64, 1., 3., 6.).unwrap();
    assert_eq!(a, b);
    // The window around each anchor brackets its padded depth interval.
    let pixel = camera.project([-0.5, -0.5, 4.]).unwrap();
    let i = pixel[1] as usize * 64 + pixel[0] as usize;
    assert!(a.0[i] <= 4. && a.1[i] >= 4., "{:?}", (a.0[i], a.1[i]));
    // Far-away pixels without anchors keep the zero (global range) marker.
    assert_eq!(a.0[0], 0.);
    assert_eq!(a.1[0], 0.);
}
#[test]
fn coarse_intervals_bracket_lifted_depth() {
    let coarse = [0., 4., 5., 0.];
    let (lo, hi) = coarse_intervals(&coarse, 2, 4, 4, 3., 6., 64);
    // Pixel (1, 1) lifts coarse cell (0, 0): empty, keeps the zero marker.
    assert_eq!(lo[5], 0.);
    // Pixel (2, 0) lifts coarse cell (1, 0) with depth 4.
    assert!(lo[2] < 4. && hi[2] > 4., "{:?}", (lo[2], hi[2]));
    assert!(lo[2] >= 3. && hi[2] <= 6.);
}
#[test]
fn centered_correlation_rejects_blank_and_preserves_exposure() {
    assert!(Patch::new([1.; 9]).is_none());
    let a = [0., 0.2, 0.7, 0.3, 0.9, 0.1, 0.8, 0.5, 0.2];
    assert!(
        (Patch::new(a)
            .unwrap()
            .correlate(a.map(|x| x * 0.6 + 0.2))
            .unwrap()
            - 1.)
            .abs()
            < 1e-10
    );
}
#[test]
fn sampled_correlation_is_bitwise_equal_to_separate_reductions() {
    let mut state = 0x573a_691d_eb4f_25c1u64;
    let mut random = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / ((1u64 << 53) as f64)
    };
    for len in [9, 25] {
        for trial in 0..1000 {
            let reference = std::array::from_fn(|_| random());
            let mut values = std::array::from_fn::<_, 25, _>(|_| random());
            if trial % 4 == 0 {
                // Include low-variance rejection near the numerical threshold.
                values = values.map(|v| 0.5 + v * 0.001);
            }
            let patch = Patch::from_values(reference, len).unwrap();
            let values_slice = &values[..len];
            let sum = values_slice.iter().sum::<f64>();
            let energy =
                values_slice.iter().map(|x| x * x).sum::<f64>() - sum * sum / len as f64;
            let expected = if energy < 0.0001 {
                None
            } else {
                let covariance = patch.centered[..len]
                    .iter()
                    .zip(values_slice)
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                Some((covariance / (patch.energy * energy).sqrt()).clamp(-1., 1.))
            };
            assert_eq!(
                patch.correlate(values).map(f64::to_bits),
                expected.map(f64::to_bits),
                "length={len} trial={trial}"
            );
        }
    }
}

#[test]
fn source_rejection_keeps_partial_sample_count_and_u64_range() {
    let image = GrayImage::new(&Image {
        width: 16,
        height: 16,
        rgb: (0..16 * 16).flat_map(|i| [(i % 256) as u8; 3]).collect(),
        focal: 1.,
    });
    let camera = Camera::identity(1., 0., 0.);
    let patch = Patch::new([0., 0.2, 0.7, 0.3, 0.9, 0.1, 0.8, 0.5, 0.2]).unwrap();
    let mut source = Source {
        index: 0,
        image: &image,
        camera: &camera,
        rotation: ID,
        translation: [0.; 3],
        offsets: [[0.; 3]; 25],
        rays: [[1., 1., 1.]; 25],
    };
    for invalid in [[-1., 1., 1.], [1., 1., -1.], [f64::NAN, 1., 1.]] {
        source.rays[4] = invalid;
        let mut count = u32::MAX as u64 - 2;
        assert!(source.score(&patch, 1., &mut count).is_none());
        assert_eq!(count, u32::MAX as u64 + 2);
    }
    source.rays[4] = [1., 1., 1.];
    let mut count = 0;
    assert!(source.score(&patch, 1., &mut count).is_none()); // Uniform patch.
    assert_eq!(count, 9);
}

#[test]
fn source_selection_rejects_pure_rotation_and_prefers_supported_baseline() {
    let camera = Camera::identity(100., 32., 32.);
    let mut close = camera.clone();
    close.translation = [-0.0001, 0., 0.];
    let mut useful = camera.clone();
    useful.translation = [-0.5, 0., 0.];
    let sparse = Reconstruction {
        cameras: vec![
            Some(camera.clone()),
            Some(close),
            Some(useful),
            Some(camera),
        ],
        points: (0..20)
            .map(|i| crate::Point {
                position: [i as f64 * 0.01, 0., 4.],
                color: [0; 3],
                observations: vec![(0, i), (1, i), (2, i), (3, i)],
            })
            .collect(),
        input_images: 4,
        reprojection_rmse: 0.,
    };
    assert_eq!(select_views(&sparse, 0, 3), vec![2]);
}
