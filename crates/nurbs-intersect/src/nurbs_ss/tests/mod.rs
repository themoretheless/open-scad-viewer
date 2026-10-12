use super::*;
use crate::coverage_verifier::{verify_complete_report, verify_incomplete_refusal};

const LE3_DEGREE_PAIRS: &[(usize, usize)] = &[
    (1, 1),
    (1, 2),
    (1, 3),
    (2, 1),
    (2, 2),
    (2, 3),
    (3, 1),
    (3, 2),
    (3, 3),
];

fn bicubic(z: f64, shift: f64) -> Surface {
    let row = |y: f64| {
        (0..4)
            .map(|i| vec![i as f64 + shift, y, z])
            .collect::<Vec<_>>()
    };
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![row(0.), row(1.), row(2.), row(3.)],
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}

fn planar_yz(x: f64) -> Surface {
    let mut control_points = Vec::new();
    for iz in 0..4 {
        let z = iz as f64;
        control_points.push((0..4).map(|iy| vec![x, iy as f64, z]).collect());
    }
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}

fn planar_yz_wide(x: f64) -> Surface {
    let mut surface = planar_yz(x);
    for (i, row) in surface.control_points.iter_mut().take(4).enumerate() {
        for (j, point) in row.iter_mut().take(4).enumerate() {
            *point = vec![x, -1. + j as f64 * 5. / 3., -1. + i as f64 * 5. / 3.];
        }
    }
    surface
}

fn bilinear_xy(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![3., 0., z]],
            vec![vec![0., 3., z], vec![3., 3., z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn bilinear_yz(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x, 0., 0.], vec![x, 3., 0.]],
            vec![vec![x, 0., 3.], vec![x, 3., 3.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn planar_45() -> Surface {
    let mut control_points = Vec::new();
    for iu in 0..4 {
        let u = iu as f64;
        control_points.push(
            (0..4)
                .map(|iv| {
                    let v = iv as f64;
                    vec![u, v, u]
                })
                .collect(),
        );
    }
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}

fn bumped_xy() -> Surface {
    let mut s = bicubic(0., 0.);
    s.control_points[1][1][2] = 0.02;
    s
}

#[test]
fn separated_bicubics_are_complete_empty() {
    let report =
        narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 20.), Options::default())
            .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    verify_complete_report(&report, true).unwrap();
    assert!(!report.permits_topology_change());
}

#[test]
fn overlapping_bicubics_refuse_without_complete() {
    let report =
        narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 0.5), Options::default())
            .unwrap();
    verify_incomplete_refusal(&report).unwrap();
}

#[test]
fn degree_gt3_is_unsupported() {
    let mut s = bicubic(0., 0.);
    s.degree_u = 4;
    s.knots_u = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    s.control_points = vec![
        s.control_points[0].clone(),
        s.control_points[1].clone(),
        s.control_points[2].clone(),
        s.control_points[3].clone(),
        s.control_points[3].clone(),
    ];
    s.weights = vec![vec![1.; 4]; 5];
    let report = narrow_transverse_bicubic(&s, &bicubic(0., 20.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
}

#[test]
fn rational_weights_and_high_degree_refuse() {
    let mut s = bicubic(0., 0.);
    s.weights[0][0] = 2.;
    let report = narrow_transverse_bicubic(&s, &bicubic(0., 20.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
}

#[test]
fn periodic_bezier_le3_refuses() {
    let mut s = bicubic(0., 0.);
    s.periodic_u = true;
    assert!(!is_bezier_le3_positive(&s));
    assert!(as_uniform_bicubic(&s).is_none());
    assert!(
        narrow_transverse_bezier_le3(&s, &bicubic(0., 20.), Options::default()).is_err(),
        "invalid periodic storage must refuse before the product gate"
    );
}

#[test]
fn g6_never_permits_topology_change() {
    let report =
        narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 20.), Options::default())
            .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    assert!(!report.permits_topology_change());
}

#[test]
fn planar_transverse_bicubics_publish_complete_line() {
    let xy = bicubic(0., 0.);
    let yz = planar_yz(1.5);
    let report = narrow_transverse_bicubic(&xy, &yz, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    verify_complete_report(&report, false).unwrap();
    assert!(matches!(report.components[0], G6Component::Line { .. }));
    assert!(!report.permits_topology_change());
}

#[test]
fn nonplanar_bump_may_complete_or_typed_refuse() {
    let a = bumped_xy();
    let b = planar_yz(1.5);
    let report = narrow_transverse_bicubic(&a, &b, Options::default()).unwrap();
    if report.coverage == Coverage::Complete {
        verify_complete_report(&report, false).unwrap();
    } else {
        verify_incomplete_refusal(&report).unwrap();
    }
}

#[test]
fn ep01_elevated_planar_times_bicubic_complete_line() {
    let report =
        narrow_transverse_bicubic(&bilinear_xy(0.), &planar_yz(1.5), Options::default())
            .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    verify_complete_report(&report, false).unwrap();
    assert!(matches!(report.components[0], G6Component::Line { .. }));
}

#[test]
fn ep02_two_elevated_planar_transverse() {
    let report =
        narrow_transverse_bicubic(&bilinear_xy(0.), &bilinear_yz(1.5), Options::default())
            .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    assert!(matches!(report.components[0], G6Component::Line { .. }));
}

#[test]
fn bc01_planar_45_orientation_complete_line() {
    let xy = bicubic(0., 0.);
    let angled = planar_45();
    let report = narrow_transverse_bicubic(&xy, &angled, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    verify_complete_report(&report, false).unwrap();
    assert!(matches!(report.components[0], G6Component::Line { .. }));
}

fn bezier_le3_xy(du: usize, dv: usize, z: f64, shift: f64) -> Surface {
    let nu = du + 1;
    let nv = dv + 1;
    let mut control_points = Vec::with_capacity(nu);
    for iu in 0..nu {
        let u = iu as f64 * 3. / du.max(1) as f64 + shift;
        control_points.push(
            (0..nv)
                .map(|iv| {
                    let v = iv as f64 * 3. / dv.max(1) as f64;
                    vec![u, v, z]
                })
                .collect(),
        );
    }
    Surface {
        degree_u: du,
        degree_v: dv,
        knots_u: [vec![0.; nu], vec![1.; nu]].concat(),
        knots_v: [vec![0.; nv], vec![1.; nv]].concat(),
        control_points,
        weights: vec![vec![1.; nv]; nu],
        periodic_u: false,
        periodic_v: false,
    }
}

fn bezier_le3_yz(du: usize, dv: usize, x: f64) -> Surface {
    let nu = du + 1;
    let nv = dv + 1;
    let mut control_points = Vec::with_capacity(nu);
    for iu in 0..nu {
        let y = iu as f64 * 3. / du.max(1) as f64;
        control_points.push(
            (0..nv)
                .map(|iv| {
                    let z = iv as f64 * 3. / dv.max(1) as f64;
                    vec![x, y, z]
                })
                .collect(),
        );
    }
    Surface {
        degree_u: du,
        degree_v: dv,
        knots_u: [vec![0.; nu], vec![1.; nu]].concat(),
        knots_v: [vec![0.; nv], vec![1.; nv]].concat(),
        control_points,
        weights: vec![vec![1.; nv]; nu],
        periodic_u: false,
        periodic_v: false,
    }
}

fn bumped_bezier_xy(du: usize, dv: usize) -> Surface {
    let mut s = bezier_le3_xy(du, dv, 0., 0.);
    let iu = (du / 2).min(du);
    let iv = (dv / 2).min(dv);
    s.control_points[iu][iv][2] = 0.02;
    s
}

#[test]
fn bezier_le3_matrix_separated_complete_empty() {
    for &(du, dv) in LE3_DEGREE_PAIRS {
        for &(eu, ev) in LE3_DEGREE_PAIRS {
            let report = narrow_transverse_bezier_le3(
                &bezier_le3_xy(du, dv, 0., 0.),
                &bezier_le3_xy(eu, ev, 0., 20.),
                Options::default(),
            )
            .unwrap();
            assert_eq!(
                report.coverage,
                Coverage::Complete,
                "empty expected for ({du}×{dv})×({eu}×{ev})"
            );
            verify_complete_report(&report, true).unwrap();
        }
    }
}

#[test]
fn bezier_le3_matrix_planar_transverse_complete_line() {
    for &(du, dv) in LE3_DEGREE_PAIRS {
        for &(eu, ev) in LE3_DEGREE_PAIRS {
            let report = narrow_transverse_bezier_le3(
                &bezier_le3_xy(du, dv, 0., 0.),
                &bezier_le3_yz(eu, ev, 1.5),
                Options::default(),
            )
            .unwrap();
            assert_eq!(
                report.coverage,
                Coverage::Complete,
                "line expected for ({du}×{dv})×yz({eu}×{ev})"
            );
            verify_complete_report(&report, false).unwrap();
            assert!(matches!(report.components[0], G6Component::Line { .. }));
        }
    }
}

#[test]
fn bezier_le3_nonplanar_certified_curve_matrix() {
    for &(du, dv) in &[(2usize, 2), (2, 3), (3, 3)] {
        let report = narrow_transverse_bezier_le3(
            &bumped_bezier_xy(du, dv),
            &planar_yz_wide(1.5),
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Complete, "degree {du}x{dv}");
        verify_complete_report(&report, false).unwrap();
        assert!(matches!(report.components[0], G6Component::Curve { .. }));
    }
}

#[test]
fn is_bezier_le3_positive_accepts_mixed_degrees() {
    assert!(is_bezier_le3_positive(&bezier_le3_xy(2, 3, 0., 0.)));
    assert!(is_bezier_le3_positive(&bezier_le3_xy(1, 1, 0., 0.)));
    assert!(as_uniform_bicubic(&bezier_le3_xy(2, 3, 0., 0.)).is_some());
    assert!(as_uniform_bicubic(&bezier_le3_xy(2, 2, 0., 0.)).is_some());
}

fn graph_patch(du: usize, dv: usize) -> Surface {
    let mut surface = bezier_le3_xy(du, dv, 0., 0.);
    if du > 1 && dv > 1 {
        surface.control_points[1][1][2] = 0.2;
    }
    surface
}

fn planar_xz_wide(y: f64) -> Surface {
    let mut surface = planar_yz_wide(0.);
    for (i, row) in surface.control_points.iter_mut().take(4).enumerate() {
        for (j, point) in row.iter_mut().take(4).enumerate() {
            *point = vec![-1. + j as f64 * 5. / 3., y, -1. + i as f64 * 5. / 3.];
        }
    }
    surface
}

fn transform_surface(mut surface: Surface) -> Surface {
    for point in surface.control_points.iter_mut().flatten() {
        let [x, y, z] = [point[0], point[1], point[2]];
        *point = vec![
            2. * x + 0.25 * y + 7.,
            -0.5 * x + 1.5 * y - 3.,
            0.2 * x - 0.1 * y + 0.75 * z + 5.,
        ];
    }
    surface
}

#[test]
fn exact_iso_u_v_degree_2_3_survive_affine_transform() {
    let context = ToleranceContext::default_valid();
    for &(du, dv) in &[(2, 2), (2, 3), (3, 2), (3, 3)] {
        for (graph, cutter, axis) in [
            (
                transform_surface(graph_patch(du, dv)),
                transform_surface(planar_yz_wide(1.5)),
                ExactIsoAxis::U,
            ),
            (
                transform_surface(graph_patch(du, dv)),
                transform_surface(planar_xz_wide(1.5)),
                ExactIsoAxis::V,
            ),
        ] {
            let certificate =
                certify_exact_planar_iso_intersection(&graph, &cutter, &context).unwrap();
            assert_eq!(certificate.first_axis, axis);
            assert!(certificate.permits_topology_authorship());
            assert_eq!(
                certificate.curve.degree,
                if axis == ExactIsoAxis::U { dv } else { du }
            );
            assert_eq!(certificate.uv_traces[0].degree, certificate.curve.degree);
        }
    }
}

#[test]
fn iso_certificate_mutations_revoke_authority() {
    let context = ToleranceContext::default_valid();
    let certificate = certify_exact_planar_iso_intersection(
        &graph_patch(3, 3),
        &planar_yz_wide(1.5),
        &context,
    )
    .unwrap();
    assert!(certificate.permits_topology_authorship());
    let mut mutated = certificate.clone();
    mutated.curve.control_points[1][2] += 1e-9;
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = certificate.clone();
    mutated.uv_traces[1].control_points[1][0] += 1e-9;
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = certificate.clone();
    mutated.fixed_parameter = f64::from_bits(mutated.fixed_parameter.to_bits() + 1);
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = certificate.clone();
    mutated.endpoints.first_support.swap(0, 1);
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = certificate.clone();
    mutated.evidence.claims.clear();
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = certificate;
    mutated.strict_transverse = false;
    assert!(!mutated.permits_topology_authorship());
}

#[test]
fn iso_certifier_refuses_rational_periodic_multispan_boundary_corner_and_nonunique() {
    let context = ToleranceContext::default_valid();
    let cutter = planar_yz_wide(1.5);
    let mut rational = graph_patch(3, 3);
    rational.weights[1][1] = 2.;
    assert!(certify_exact_planar_iso_intersection(&rational, &cutter, &context).is_err());

    let mut periodic = graph_patch(3, 3);
    periodic.periodic_u = true;
    assert!(certify_exact_planar_iso_intersection(&periodic, &cutter, &context).is_err());

    let multispan = graph_patch(3, 3)
        .edit_axis(Axis::U, |curve| curve.insert(0.5, 1))
        .unwrap();
    assert!(certify_exact_planar_iso_intersection(&multispan, &cutter, &context).is_err());

    assert!(
        certify_exact_planar_iso_intersection(
            &graph_patch(3, 3),
            &planar_yz_wide(0.),
            &context
        )
        .is_err()
    );
    assert!(
        certify_exact_planar_iso_intersection(&graph_patch(3, 3), &planar_yz(1.5), &context)
            .is_err()
    );

    for coefficients in [[-1., 2., -2., 1.], [0., 0., 0., 0.], [1., 0., 0., 1.]] {
        let mut nonunique = graph_patch(3, 3);
        for (i, value) in coefficients.into_iter().enumerate() {
            for j in 0..=nonunique.degree_v {
                nonunique.control_points[i][j][0] = value;
                nonunique.control_points[i][j][2] = i as f64;
            }
        }
        assert!(
            certify_exact_planar_iso_intersection(&nonunique, &planar_yz_wide(0.), &context)
                .is_err()
        );
    }
}

fn multispan_wave(u_spans: usize, v_spans: usize, rational: bool, swap_uv: bool) -> Surface {
    let u_count = u_spans + 1;
    let v_count = v_spans + 1;
    let mut controls = vec![vec![vec![0.; 3]; v_count]; u_count];
    let mut weights = vec![vec![1.; v_count]; u_count];
    for u in 0..u_count {
        for v in 0..v_count {
            let signed = if u % 2 == 0 { -1. } else { 1. };
            let point = [signed, u as f64, v as f64];
            controls[u][v] = if swap_uv {
                vec![point[0], point[2], point[1]]
            } else {
                point.to_vec()
            };
            if rational {
                weights[u][v] = if u % 2 == 0 { 1. } else { 2. };
            }
        }
    }
    let knots = |spans: usize| {
        let mut result = vec![0., 0.];
        result.extend((1..spans).map(|value| value as f64));
        result.extend([spans as f64; 2]);
        result
    };
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: knots(u_spans),
        knots_v: knots(v_spans),
        control_points: controls,
        weights,
        periodic_u: false,
        periodic_v: false,
    }
}

fn multispan_plane(size: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., size]],
            vec![vec![0., size, 0.], vec![0., size, size]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn multispan_decomposition_covers_2x1_1x2_and_2x2() {
    for (u, v, expected) in [(2, 1, 2), (1, 2, 2), (2, 2, 4)] {
        let decomposition =
            decompose_rational_bezier_spans(&multispan_wave(u, v, false, false), 17, 16)
                .unwrap();
        assert!(decomposition.permits_exact_decomposition());
        assert_eq!(decomposition.spans.len(), expected);
        assert_eq!(
            decomposition
                .spans
                .iter()
                .filter(|span| span.owns_upper[0])
                .count(),
            v
        );
        assert!(decomposition.denominator_lower_bound > 0.);
    }
}

#[test]
fn multispan_two_disjoint_branches_join_across_knot_boundaries() {
    let context = ToleranceContext::default_valid();
    let graph = certify_multispan_ss(
        &multispan_wave(2, 2, false, false),
        &multispan_plane(2.),
        [3, 9],
        &context,
        32,
    )
    .unwrap();
    assert!(graph.permits_topology_authorship());
    assert_eq!(graph.components.len(), 2);
    assert_eq!(graph.certificate.fragment_count, 4);
    assert!(
        graph
            .components
            .iter()
            .all(|component| component.fragments.len() == 2 && !component.closed)
    );
}

#[test]
fn multispan_internal_knot_has_one_half_open_owner() {
    let context = ToleranceContext::default_valid();
    let mut source = multispan_wave(2, 2, false, false);
    for (u, row) in source.control_points.iter_mut().enumerate().take(3) {
        for point in row.iter_mut().take(3) {
            point[0] = u as f64 - 1.;
        }
    }
    let graph =
        certify_multispan_ss(&source, &multispan_plane(2.), [5, 6], &context, 32).unwrap();
    assert_eq!(graph.components.len(), 1);
    assert_eq!(graph.components[0].fragments.len(), 2);
    assert!(
        graph.components[0]
            .fragments
            .iter()
            .all(|fragment| fragment.source_spans[0].u == 1)
    );
}

#[test]
fn multispan_v_reversal_rational_weights_and_rigid_transform() {
    let context = ToleranceContext::default_valid();
    let mut reversed = multispan_wave(2, 2, true, true);
    reversed = reversed
        .edit_axis(Axis::U, |curve| curve.reverse())
        .unwrap()
        .edit_axis(Axis::V, |curve| curve.reverse())
        .unwrap();
    let mut plane = multispan_plane(2.);
    plane = plane
        .edit_axis(Axis::U, |curve| curve.reverse())
        .unwrap()
        .edit_axis(Axis::V, |curve| curve.reverse())
        .unwrap();
    let transform = |mut surface: Surface| {
        for point in surface.control_points.iter_mut().flatten() {
            let [x, y, z] = [point[0], point[1], point[2]];
            *point = vec![-y + 4., x + 7., z - 3.];
        }
        surface
    };
    let graph = certify_multispan_ss(
        &transform(reversed),
        &transform(plane),
        [4, 8],
        &context,
        32,
    )
    .unwrap();
    assert!(graph.permits_topology_authorship());
    assert_eq!(graph.components.len(), 2);
    assert!(graph.certificate.denominator_lower_bound >= 1.);
}

#[test]
fn multispan_fragment_permutation_is_deterministic() {
    let context = ToleranceContext::default_valid();
    let graph = certify_multispan_ss(
        &multispan_wave(2, 2, false, false),
        &multispan_plane(2.),
        [1, 2],
        &context,
        32,
    )
    .unwrap();
    let mut fragments = graph
        .components
        .iter()
        .flat_map(|component| component.fragments.clone())
        .collect::<Vec<_>>();
    fragments.reverse();
    let joined = join_certified_multispan_fragments(
        fragments,
        &context,
        graph.certificate.source_span_count,
        graph.certificate.candidate_span_pairs,
        graph.certificate.denominator_lower_bound,
        32,
    )
    .unwrap();
    assert_eq!(joined.components.len(), graph.components.len());
    for (a, b) in joined.components.iter().zip(&graph.components) {
        assert_eq!(
            joined_endpoint_key(&a.fragments[0], 0).unwrap(),
            joined_endpoint_key(&b.fragments[0], 0).unwrap()
        );
    }
}

#[test]
fn multispan_ambiguity_coincidence_resource_and_certificate_mutations_refuse() {
    let context = ToleranceContext::default_valid();
    let source = multispan_wave(2, 2, false, false);
    let plane = multispan_plane(2.);
    assert_eq!(
        certify_multispan_ss(&source, &plane, [0, 1], &context, 1)
            .unwrap_err()
            .code,
        "BREP_SS_RESOURCE_LIMIT"
    );
    let graph = certify_multispan_ss(&source, &plane, [0, 1], &context, 32).unwrap();
    let duplicate = graph
        .components
        .iter()
        .flat_map(|component| component.fragments.clone())
        .chain(
            graph
                .components
                .iter()
                .flat_map(|component| component.fragments.clone()),
        )
        .collect();
    assert!(
        join_certified_multispan_fragments(
            duplicate,
            &context,
            graph.certificate.source_span_count,
            graph.certificate.candidate_span_pairs,
            graph.certificate.denominator_lower_bound,
            32
        )
        .is_err()
    );
    let mut mutated = graph.clone();
    mutated.certificate.fragment_count -= 1;
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = graph.clone();
    mutated.components[0].fragments[0].pcurves[0].control_points[0][0] += 1e-9;
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = graph.clone();
    mutated.components[0].fragments[0].evidence.signed_slope += 1e-9;
    assert!(!mutated.permits_topology_authorship());
    let mut mutated = graph.clone();
    mutated.certificate.candidate_span_pairs += 1;
    assert!(!mutated.permits_topology_authorship());

    let mut coincident = source.clone();
    for point in coincident.control_points.iter_mut().flatten() {
        point[0] = 0.;
    }
    assert!(certify_multispan_ss(&coincident, &plane, [0, 1], &context, 32).is_err());
    let tangent = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 2.]],
            vec![vec![-1., 1., 0.], vec![-1., 1., 2.]],
            vec![vec![1., 2., 0.], vec![1., 2., 2.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    assert!(certify_multispan_ss(&tangent, &plane, [0, 1], &context, 32).is_err());
    let mut unresolved_multi_root = tangent.clone();
    unresolved_multi_root.degree_u = 3;
    unresolved_multi_root.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    unresolved_multi_root
        .control_points
        .insert(2, vec![vec![-1., 1.5, 0.], vec![-1., 1.5, 2.]]);
    unresolved_multi_root.weights.insert(2, vec![1.; 2]);
    assert!(
        certify_multispan_ss(&unresolved_multi_root, &plane, [0, 1], &context, 32).is_err()
    );
    let mut periodic = source.clone();
    periodic.periodic_u = true;
    assert!(certify_multispan_ss(&periodic, &plane, [0, 1], &context, 32).is_err());
    let mut high_degree = source;
    high_degree = high_degree
        .edit_axis(Axis::U, |curve| curve.elevate(4))
        .unwrap();
    assert!(certify_multispan_ss(&high_degree, &plane, [0, 1], &context, 32).is_err());
}
