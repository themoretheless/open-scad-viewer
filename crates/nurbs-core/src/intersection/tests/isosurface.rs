use super::*;

fn sphere_sdf(r: f64) -> impl Fn([f64; 3]) -> f64 {
    move |p| norm(p) - r
}

fn sphere_grad(p: [f64; 3]) -> [f64; 3] {
    let l = norm(p);
    if l < 1e-12 {
        [1., 0., 0.]
    } else {
        [p[0] / l, p[1] / l, p[2] / l]
    }
}

fn config(depth: u32, mode: VertexMode) -> IsosurfaceConfig {
    IsosurfaceConfig {
        max_depth: depth,
        linearization_tolerance: 1e-4,
        vertex_mode: mode,
        ..IsosurfaceConfig::default()
    }
}

/// FNV-1a over the raw bytes of the output — byte-identical determinism.
/// Uses the shared `guards::Fnv1a` construction (item 1094).
fn hash_output(out: &IsosurfaceOutput) -> u64 {
    use crate::foundation::guards::Fnv1a;
    let mut h = Fnv1a::new();
    for p in &out.mesh.points {
        for v in p {
            h.write(&v.to_le_bytes());
        }
    }
    for t in &out.mesh.triangles {
        for i in t {
            h.write(&i.to_le_bytes());
        }
    }
    for e in &out.sharp_edges {
        for i in e {
            h.write(&i.to_le_bytes());
        }
    }
    h.finish()
}

#[test]
fn sphere_volume_and_vertices_on_surface() {
    let out = polygonize_sdf(
        [-1.4; 3],
        [1.4; 3],
        &sphere_sdf(1.),
        Some(&sphere_grad),
        &config(5, VertexMode::Qef),
    )
    .unwrap();
    assert!(!out.mesh.triangles.is_empty());
    let volume = out.mesh.signed_volume();
    let expected = 4. / 3. * std::f64::consts::PI;
    assert!(
        (volume - expected).abs() / expected < 0.05,
        "volume {volume} vs {expected}"
    );
    for p in &out.mesh.points {
        let r = norm(*p);
        assert!((r - 1.).abs() < 0.06, "vertex radius {r}");
    }
    // Smooth sphere: no sharp edges at the 30° threshold.
    assert!(out.sharp_edges.is_empty());
}

#[test]
fn sphere_without_gradient_uses_finite_differences() {
    let out = polygonize_sdf(
        [-1.4; 3],
        [1.4; 3],
        &sphere_sdf(1.),
        None,
        &config(5, VertexMode::Qef),
    )
    .unwrap();
    let volume = out.mesh.signed_volume();
    let expected = 4. / 3. * std::f64::consts::PI;
    assert!((volume - expected).abs() / expected < 0.05);
}

/// Two overlapping spheres: the intersection circle is a sharp ridge that
/// QEF dual contouring must preserve better than centroid placement.
#[test]
fn qef_preserves_sharp_ridge_better_than_centroid() {
    let d = 0.55f64;
    let ridge_r = (1. - d * d).sqrt();
    let sdf = move |p: [f64; 3]| {
        let s1 = norm([p[0] - d, p[1], p[2]]) - 1.;
        let s2 = norm([p[0] + d, p[1], p[2]]) - 1.;
        s1.max(s2)
    };
    let grad = move |p: [f64; 3]| {
        let unit = |v: [f64; 3]| {
            let l = norm(v).max(1e-12);
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let g1 = unit([p[0] - d, p[1], p[2]]);
        let g2 = unit([p[0] + d, p[1], p[2]]);
        if norm([p[0] - d, p[1], p[2]]) > norm([p[0] + d, p[1], p[2]]) {
            g1
        } else {
            g2
        }
    };
    let ridge_error = |mode: VertexMode| {
        let out = polygonize_sdf(
            [-1.8, -1.2, -1.2],
            [1.8, 1.2, 1.2],
            &sdf,
            Some(&grad),
            &config(5, mode),
        )
        .unwrap();
        let mut errors = Vec::new();
        for p in &out.mesh.points {
            let rho = (p[1] * p[1] + p[2] * p[2]).sqrt();
            if (rho - ridge_r).abs() < 0.15 && p[0].abs() < 0.35 {
                // Distance to the exact ridge circle (x = 0, ρ = ridge_r).
                let mut best = f64::INFINITY;
                for k in 0..64 {
                    let a = k as f64 / 64. * 2. * std::f64::consts::PI;
                    let q = [0., ridge_r * a.cos(), ridge_r * a.sin()];
                    best = best.min(norm([p[0] - q[0], p[1] - q[1], p[2] - q[2]]));
                }
                errors.push(best);
            }
        }
        assert!(errors.len() >= 8, "too few ridge vertices");
        errors.iter().sum::<f64>() / errors.len() as f64
    };
    let qef = ridge_error(VertexMode::Qef);
    let centroid = ridge_error(VertexMode::Centroid);
    assert!(
        qef < centroid,
        "QEF ridge error {qef} should beat centroid {centroid}"
    );
    // The ridge is sharp: it must be detected as sharp edges.
    let out = polygonize_sdf(
        [-1.8, -1.2, -1.2],
        [1.8, 1.2, 1.2],
        &sdf,
        Some(&grad),
        &config(5, VertexMode::Qef),
    )
    .unwrap();
    assert!(!out.sharp_edges.is_empty(), "ridge should be marked sharp");
}

#[test]
fn extraction_is_byte_deterministic() {
    let run = || {
        polygonize_sdf(
            [-1.4; 3],
            [1.4; 3],
            &sphere_sdf(1.),
            Some(&sphere_grad),
            &config(5, VertexMode::Qef),
        )
        .unwrap()
    };
    let (a, b) = (run(), run());
    assert_eq!(hash_output(&a), hash_output(&b));
    assert_eq!(a, b);
    // Marching cubes path too.
    let run_mc = || {
        let mut cfg = config(5, VertexMode::Qef);
        cfg.extraction = Extraction::MarchingCubes;
        polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), Some(&sphere_grad), &cfg).unwrap()
    };
    assert_eq!(hash_output(&run_mc()), hash_output(&run_mc()));
}

#[test]
fn taubin_preserves_volume_better_than_laplacian() {
    let make = || {
        polygonize_sdf(
            [-1.4; 3],
            [1.4; 3],
            &sphere_sdf(1.),
            Some(&sphere_grad),
            &config(5, VertexMode::Qef),
        )
        .unwrap()
        .mesh
    };
    let mut taubin_mesh = make();
    let taubin = taubin_smooth(&mut taubin_mesh, 0.5, -0.53, 20).unwrap();
    let mut lap_mesh = make();
    let lap = laplacian_smooth(&mut lap_mesh, 0.5, 20).unwrap();
    assert!(
        taubin.volume_drift.abs() < lap.volume_drift.abs(),
        "Taubin drift {} vs Laplacian {}",
        taubin.volume_drift,
        lap.volume_drift
    );
    assert!(
        lap.volume_drift < 0.,
        "pure Laplacian must shrink, drift {}",
        lap.volume_drift
    );
}

#[test]
fn small_component_is_removed() {
    let sdf = |p: [f64; 3]| {
        let big = norm(p) - 1.;
        let small = norm([p[0] - 3., p[1], p[2]]) - 0.25;
        big.min(small)
    };
    let grad = |p: [f64; 3]| {
        if norm(p) - 1. < norm([p[0] - 3., p[1], p[2]]) - 0.25 {
            sphere_grad(p)
        } else {
            sphere_grad([p[0] - 3., p[1], p[2]])
        }
    };
    let mut out = polygonize_sdf(
        [-1.4, -1.4, -1.4],
        [4.4, 1.4, 1.4],
        &sdf,
        Some(&grad),
        &config(5, VertexMode::Qef),
    )
    .unwrap();
    let report = remove_small_components(&mut out.mesh, 0.5).unwrap();
    assert_eq!(report.components_before, 2);
    assert_eq!(report.components_removed, 1);
    assert_eq!(report.removed_volumes.len(), 1);
    assert!(report.removed_volumes[0] < 0.5);
    let remaining = out.mesh.signed_volume();
    let expected = 4. / 3. * std::f64::consts::PI;
    assert!(
        (remaining - expected).abs() / expected < 0.05,
        "remaining volume {remaining}"
    );
}

#[test]
fn budgets_reject_excessive_requests() {
    // Depth beyond the hard cap is a resource error.
    let cfg = config(HARD_MAX_DEPTH + 1, VertexMode::Qef);
    let err = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), None, &cfg).unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT);
    // A cell budget too small for the refinement is a resource error.
    let mut cfg = config(6, VertexMode::Qef);
    cfg.max_cells = 64;
    let err = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), None, &cfg).unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT);
    // Taubin iteration budget.
    let mut mesh = IsoMesh {
        points: vec![[0.; 3], [1., 0., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2]],
    };
    assert!(taubin_smooth(&mut mesh, 0.5, -0.53, MAX_SMOOTH_ITERATIONS + 1).is_err());
}

#[test]
fn marching_cubes_sphere_volume() {
    let mut cfg = config(5, VertexMode::Centroid);
    cfg.extraction = Extraction::MarchingCubes;
    let out = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), Some(&sphere_grad), &cfg)
        .unwrap();
    let volume = out.mesh.signed_volume();
    let expected = 4. / 3. * std::f64::consts::PI;
    assert!(
        (volume - expected).abs() / expected < 0.06,
        "MC volume {volume} vs {expected}"
    );
}

// -- guards (items 1065, 1093, 1094) -------------------------------------

#[test]
fn non_finite_domain_is_rejected_as_input_error() {
    for (min, max) in [
        ([f64::NAN; 3], [1.; 3]),
        ([-1.; 3], [f64::INFINITY; 3]),
        ([0., f64::NEG_INFINITY, 0.], [1.; 3]),
    ] {
        let err = polygonize_sdf(min, max, &sphere_sdf(1.), None, &config(4, VertexMode::Qef))
            .unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }
}

#[test]
fn adaptive_refinement_guard_preserves_budget_error() {
    // The BudgetGuard backing max_cells keeps exhaustion a typed resource
    // error mentioning the refinement stage or the legacy cell message.
    let mut cfg = config(6, VertexMode::Qef);
    cfg.max_cells = 8;
    let err = polygonize_sdf([-1.4; 3], [1.4; 3], &sphere_sdf(1.), None, &cfg).unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    assert!(
        err.contains("isosurface_adaptive_refinement") || err.contains("cell budget"),
        "{err}"
    );
}

#[test]
fn hash_output_uses_shared_fnv1a_and_stays_deterministic() {
    use crate::foundation::guards::Fnv1a;
    let out = polygonize_sdf(
        [-1.4; 3],
        [1.4; 3],
        &sphere_sdf(1.),
        Some(&sphere_grad),
        &config(4, VertexMode::Qef),
    )
    .unwrap();
    assert_eq!(hash_output(&out), hash_output(&out));
    // Empty output hashes to the bare offset basis of the shared hasher.
    assert_eq!(hash_output(&IsosurfaceOutput::default()), Fnv1a::new().finish());
}
