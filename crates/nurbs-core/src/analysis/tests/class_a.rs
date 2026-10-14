use super::*;
use crate::curve::Curve;
use crate::surface::revolve;

fn sphere() -> Surface {
    // Единичная сфера; полюса на концах профиля (сетка с inset их избегает).
    let profile = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![0., 0., -1.],
            vec![1., 0., -1.],
            vec![1., 0., 0.],
            vec![1., 0., 1.],
            vec![0., 0., 1.],
        ],
        weights: vec![
            1.,
            std::f64::consts::FRAC_1_SQRT_2,
            1.,
            std::f64::consts::FRAC_1_SQRT_2,
            1.,
        ],
        periodic: false,
    };
    revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
}

/// Билинейный прямоугольник в плоскости z=0: (x,y) ∈ [0,1]×[0,1].
fn flat_patch(origin: [f64; 3], ux: [f64; 3], uy: [f64; 3]) -> Surface {
    let p = |a: f64, b: f64| {
        vec![
            origin[0] + a * ux[0] + b * uy[0],
            origin[1] + a * ux[1] + b * uy[1],
            origin[2] + a * ux[2] + b * uy[2],
        ]
    };
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![p(0., 0.), p(0., 1.)],
            vec![p(1., 0.), p(1., 1.)],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

/// Пластина 5×5 (три спана на ось) со сдвинутой по z центральной
/// контрольной точкой — дефект локализуется базисными функциями.
fn dented_patch(dent: f64) -> Surface {
    let knots = vec![0., 0., 0., 4. / 3., 8. / 3., 4., 4., 4.];
    let mut cp = vec![];
    for i in 0..5 {
        let mut row = vec![];
        for j in 0..5 {
            let z = if i == 2 && j == 2 { dent } else { 0. };
            row.push(vec![i as f64, j as f64, z]);
        }
        cp.push(row);
    }
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points: cp,
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}

fn limits() -> Limits {
    Limits::default()
}

#[test]
fn sphere_highlight_lines_are_clean() {
    let s = sphere();
    let lights = vec![
        [1., 0., 1.],
        [0., 1., 1.],
        [-1., 0., 1.],
        [0., -1., 1.],
    ];
    let reports =
        highlight_lines(&s, &lights, [0., 0., 1.], 0.5, [64, 64], 0.35, &limits()).unwrap();
    assert_eq!(reports.len(), 4);
    for r in &reports {
        assert!(!r.polylines.is_empty(), "sphere should show highlight lines");
        assert!(r.defects.is_empty(), "sphere highlight defects: {:?}", r.defects);
    }
}

#[test]
fn sphere_reflection_lines_have_no_jumps() {
    let s = sphere();
    let (report, jumps) =
        reflection_lines(&s, [1., 0., 1.], [0., 0., 1.], 0.5, [64, 64], 200., &limits())
            .unwrap();
    assert!(!report.polylines.is_empty());
    assert!(jumps.is_empty(), "sphere reflection jumps: {:?}", jumps);
}

#[test]
fn sphere_isophotes_are_clean_and_evenly_ordered() {
    let s = sphere();
    // Уровни вблизи "экватора" освещения: там шаг почти равномерный.
    let report = isophote_analysis(
        &s,
        [0., 0., 1.],
        &[0.35, 0.45, 0.55],
        [48, 48],
        0.9,
        &limits(),
    )
    .unwrap();
    assert!(report.mean_spacing > 0.);
    assert!(
        report.spacing_unevenness < 0.9,
        "unevenness {}",
        report.spacing_unevenness
    );
    for level in &report.levels {
        assert!(level.defects.is_empty(), "sphere isophote kinks: {:?}", level.defects);
    }
}

#[test]
fn dent_detector_finds_shifted_control_point_zone() {
    let flat = dented_patch(0.);
    let flat_report = dent_detection(&flat, [24, 24], 1.5, 1e-6, &limits()).unwrap();
    assert!(
        flat_report.max_residual < 1e-6,
        "flat plate must be dent-free, got {}",
        flat_report.max_residual
    );
    let dented = dented_patch(0.4);
    let report = dent_detection(&dented, [24, 24], 1.5, 0.02, &limits()).unwrap();
    assert!(!report.zones.is_empty(), "dent must be detected");
    let top = &report.zones[0];
    // Вмятина у контрольной точки (2,2) пластины [0,4]² (домен узлов).
    assert!(
        (top.uv[0] - 2.).abs() < 1. && (top.uv[1] - 2.).abs() < 1.,
        "dent zone {:?} should sit near the shifted control point",
        top
    );
}

/// Два полуцилиндра: A покрывает углы 0..180°, B — 180..360°.
/// Общий шов — образующая при 180°; нормали совпадают (G1), кривизны тоже (G2).
fn half_cylinders() -> (Surface, Surface) {
    let pa = Curve::from_polyline(vec![vec![1., 0., 0.], vec![1., 0., 1.]]).unwrap();
    let pb = Curve::from_polyline(vec![vec![-1., 0., 0.], vec![-1., 0., 1.]]).unwrap();
    (
        revolve(&pa, [0.; 3], [0., 0., 1.], 180.).unwrap(),
        revolve(&pb, [0.; 3], [0., 0., 1.], 180.).unwrap(),
    )
}

/// Ищем пару границ, чьи средние точки совпадают.
fn matching_edges(a: &Surface, b: &Surface) -> (SeamEdge, SeamEdge) {
    let edges = [SeamEdge::UMin, SeamEdge::UMax, SeamEdge::VMin, SeamEdge::VMax];
    let mid = |s: &Surface, e: SeamEdge| {
        let (frac, _) = e.point(0.5);
        let (du, dv) = surface_domains(s);
        s.evaluate(
            du[0] + (du[1] - du[0]) * frac[0],
            dv[0] + (dv[1] - dv[0]) * frac[1],
        )
        .unwrap()
        .point
    };
    let mut best = (f64::INFINITY, edges[0], edges[0]);
    for ea in edges {
        for eb in edges {
            let d = norm(sub(mid(a, ea), mid(b, eb)));
            if d < best.0 {
                best = (d, ea, eb);
            }
        }
    }
    assert!(best.0 < 1e-9, "no shared boundary found, gap {}", best.0);
    (best.1, best.2)
}

#[test]
fn smooth_g1_seam_passes_thresholds() {
    let (a, b) = half_cylinders();
    let (ea, eb) = matching_edges(&a, &b);
    let report = seam_zebra(&a, ea, &b, eb, 17, &limits()).unwrap();
    assert!(report.max_position_gap < 1e-9, "gap {}", report.max_position_gap);
    assert!(
        report.max_normal_angle < 1e-6,
        "normal angle {}",
        report.max_normal_angle
    );
    let continuity = classify_seam(&report, 1e-6, 1e-3, 0.05);
    assert_eq!(continuity, SeamContinuity::G2);
    let validation = validate_seam_class(&report, &ClassATolerances::default()).unwrap();
    assert_eq!(validation.class, SurfaceClass::A);
    assert!(validation.violations.is_empty());
    let json = validation.to_json();
    assert!(json.starts_with("{\"class\":\"A\""));
    assert!(json.ends_with("]}"));
}

#[test]
fn kinked_seam_is_g0_but_not_g1() {
    // Две полуплоскости с малым двугранным углом (~5.7°) вдоль y-оси.
    let angle = 0.1_f64;
    let a = flat_patch([0., 0., 0.], [1., 0., 0.], [0., 1., 0.]);
    let b = flat_patch(
        [1., 0., 0.],
        [angle.cos(), 0., angle.sin()],
        [0., 1., 0.],
    );
    let report = seam_zebra(&a, SeamEdge::UMax, &b, SeamEdge::UMin, 9, &limits()).unwrap();
    assert!(report.max_position_gap < 1e-12);
    assert!(
        (report.max_normal_angle - angle).abs() < 1e-9,
        "normal angle {} vs {}",
        report.max_normal_angle,
        angle
    );
    assert_eq!(
        classify_seam(&report, 1e-6, 1e-3, 0.05),
        SeamContinuity::G0
    );
    // B-порог по нормалям 0.05 рад < 0.1 рад → класс C.
    let validation = validate_seam_class(&report, &ClassATolerances::default()).unwrap();
    assert_eq!(validation.class, SurfaceClass::C);
    assert!(
        validation
            .violations
            .iter()
            .any(|v| v.kind == DefectKind::SeamNormalAngle)
    );
    let json = validation.to_json();
    assert!(json.contains("\"type\":\"normal_angle\""));
    assert!(json.contains("\"threshold\":0.01"));
}

#[test]
fn budgets_reject_oversized_grids() {
    let s = sphere();
    let tight = Limits {
        max_grid_axis: 8,
        ..Limits::default()
    };
    let err = highlight_lines(&s, &[[0., 0., 1.]], [0., 0., 1.], 0.9, [64, 64], 0.35, &tight);
    assert!(err.is_err());
}

#[test]
fn invalid_inputs_are_rejected() {
    let s = sphere();
    assert!(highlight_lines(&s, &[], [0., 0., 1.], 0.9, [8, 8], 0.35, &limits()).is_err());
    assert!(highlight_lines(&s, &[[0., 0., 0.]], [0., 0., 1.], 0.9, [8, 8], 0.35, &limits())
        .is_err());
    assert!(isophote_analysis(&s, [0., 0., 1.], &[2., 3.], [8, 8], 0.5, &limits()).is_err());
}

#[test]
fn rejects_non_finite_level_and_kink_angle() {
    let s = sphere();
    // NaN-уровень изолинии.
    let err = highlight_lines(&s, &[[0., 0., 1.]], [0., 0., 1.], f64::NAN, [8, 8], 0.35, &limits())
        .unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("isoline level"), "{err}");
    // Inf-порог излома.
    let err = highlight_lines(&s, &[[0., 0., 1.]], [0., 0., 1.], 0.5, [8, 8], f64::INFINITY, &limits())
        .unwrap_err();
    assert!(err.contains("kink angle"), "{err}");
}

#[test]
fn marching_squares_budget_is_a_named_stage() {
    // Шахматное поле: каждая ячейка неоднозначна → 2 сегмента на ячейку.
    let (nu, nv) = (4, 4);
    let mut values = Vec::with_capacity((nu + 1) * (nv + 1));
    for j in 0..=nv {
        for i in 0..=nu {
            values.push(if (i + j) % 2 == 0 { 1. } else { -1. });
        }
    }
    let field = Field {
        nu,
        nv,
        inset: 0.,
        domain_u: [0., 1.],
        domain_v: [0., 1.],
        values,
    };
    let tight = Limits {
        max_segments: 4,
        ..Limits::default()
    };
    let err = field.marching_segments(0., &tight).unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    assert!(err.contains("class-a.marching-squares"), "{err}");
    // С полным бюджетом та же выборка проходит.
    let segments = field.marching_segments(0., &limits()).unwrap();
    assert_eq!(segments.len(), 2 * nu * nv);
}

#[test]
fn chaining_budget_limits_polyline_growth() {
    // Цепочка из трёх коллинеарных сегментов при бюджете 2 точки.
    let segments = vec![
        ([0., 0.], [1., 0.]),
        ([1., 0.], [2., 0.]),
        ([2., 0.], [3., 0.]),
    ];
    let tight = Limits {
        max_polyline_points: 2,
        ..Limits::default()
    };
    let err = chain_segments(segments.clone(), 0.1, &tight).unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    let lines = chain_segments(segments, 0.1, &limits()).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 4);
}
