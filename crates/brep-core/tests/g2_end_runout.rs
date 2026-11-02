use brep_core::region_replacement as r;
use nurbs_core::{
    surface::Surface,
    surface_blend::{self, Options, Trace},
};
fn supports() -> [Surface; 2] {
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    for p in b.control_points.iter_mut().flatten() {
        p[2] = p[1];
        p[1] = 4.;
    }
    [a, b]
}
fn request(width: f64) -> ([Trace; 2], Options) {
    (
        [
            Trace {
                at: 1. - width / 4.,
                along: [0.25, 0.75],
            },
            Trace {
                at: width / 4.,
                along: [0.25, 0.75],
            },
        ],
        Options {
            gains: [width / 4.; 2],
            ..Default::default()
        },
    )
}
#[test]
fn finite_g2_end_transition_has_whole_field_bound_and_immutable_supports() {
    let s = supports();
    let before = s.clone();
    let (traces, options) = request(2.);
    let limits = r::ContactQualificationLimits::default();
    let out = r::build_g2_end_runout(
        [&s[0], &s[1]],
        traces,
        options,
        [[1, 0, 0], [0, 1, 1]],
        1e-7,
        1e-8,
        limits,
    )
    .unwrap();
    assert_eq!(s, before);
    assert_eq!(out.panels().len(), 1);
    assert_eq!(out.transition().support_seams.len(), 2);
    assert!(out.transition().complete && out.transition().within_tolerance);
    assert!(
        out.fields()
            .iter()
            .all(|f| f.absolute_curvature_upper.unwrap() <= 10.)
    );
    assert!(out.interiors().iter().all(|i| i.certified));
    let (c, e, d, h) = out.work();
    assert!(
        c <= limits.cells
            && e <= limits.exact_work
            && d <= limits.domain_cells
            && h <= limits.shape_cells
    );
    eprintln!(
        "runout upper={:?} work={:?}",
        out.fields()[0].absolute_curvature_upper,
        out.work()
    );
}
#[test]
fn one_tenth_mm_g2_seams_cannot_hide_excessive_interior_curvature() {
    let s = supports();
    let before = s.clone();
    let (t, o) = request(0.1);
    let seam_only =
        surface_blend::g2_traces([&s[0], &s[1]], t, o, [1e-7, 1e-5, 1e-4], 100_000, 1_000_000)
            .unwrap();
    assert!(seam_only.complete && seam_only.within_tolerance);
    // A rigorous point witness is enough to disprove a whole-field upper
    // bound. A diagonal Rayleigh quotient of the symmetric world shape
    // tensor is bounded by the absolute principal-curvature maximum.
    let patch = &seam_only.construction.patches[0];
    let u = (patch.knots_u[patch.degree_u] + patch.knots_u[patch.control_points.len()]) * 0.5;
    let witness = nurbs_core::surface_shape_operator::at(
        patch,
        [u, 0.125],
        [nurbs_core::curve_differential::Side::Automatic; 2],
    )
    .unwrap()
    .tensor
    .unwrap();
    assert!(
        (0..3).any(|k| witness[k][k][0] > 10. || witness[k][k][1] < -10.),
        "{witness:?}"
    );

    let err = match r::build_g2_end_runout(
        [&s[0], &s[1]],
        t,
        o,
        [[1, 0, 0], [0, 1, 1]],
        1e-7,
        1e-8,
        r::ContactQualificationLimits::default(),
    ) {
        Ok(_) => panic!("Seam smoothness cannot grant the 10/mm field bound"),
        Err(e) => e,
    };
    assert!(err.message.contains("curvature"), "{err:?}");
    assert_eq!(s, before);
}
#[test]
fn exhausted_budget_and_degenerate_projection_cannot_publish_panels() {
    let s = supports();
    let (t, o) = request(2.);
    let l = r::ContactQualificationLimits::default();
    let mut invalid = s[0].clone();
    for p in invalid.control_points.iter_mut().flatten() {
        p.pop();
    }
    assert!(
        r::build_g2_end_runout(
            [&invalid, &s[1]],
            t,
            o,
            [[1, 0, 0], [0, 1, 1]],
            1e-7,
            1e-8,
            l
        )
        .is_err()
    );

    assert!(
        r::build_g2_end_runout(
            [&s[0], &s[1]],
            t,
            o,
            [[1, 0, 0], [0, 1, 1]],
            1e-7,
            1e-8,
            r::ContactQualificationLimits { cells: 8, ..l }
        )
        .is_err()
    );
    assert!(
        r::build_g2_end_runout([&s[0], &s[1]], t, o, [[1, 0, 0], [1, 0, 0]], 1e-7, 1e-8, l)
            .is_err()
    );
}

#[test]
fn rational_curved_original_supports_keep_their_full_g2_and_field_checks() {
    let mut s = supports();
    for support in &mut s {
        let a = support.control_points[0].clone();
        let b = support.control_points[1].clone();
        let middle = a
            .iter()
            .zip(&b)
            .map(|(a, b)| vec![2., 0.5 * (a[1] + b[1]) + 0.25, 0.5 * (a[2] + b[2])])
            .collect();
        support.control_points = vec![a, middle, b];
        support.degree_u = 2;
        support.knots_u = vec![0., 0., 0., 1., 1., 1.];
        support.weights = vec![vec![1.; 2], vec![1.25; 2], vec![1.; 2]];
    }
    let before = s.clone();
    let (t, o) = request(2.);
    let out = r::build_g2_end_runout(
        [&s[0], &s[1]],
        t,
        o,
        [[1, 0, 0], [0, 1, 1]],
        1e-7,
        1e-8,
        r::ContactQualificationLimits::default(),
    )
    .unwrap();
    assert_eq!(s, before);
    assert!(
        out.transition()
            .support_seams
            .iter()
            .all(|(_, _, p)| p.positions.proven)
    );
    assert!(
        out.fields()
            .iter()
            .all(|f| f.absolute_curvature_upper.unwrap() <= 10.)
    );
    eprintln!(
        "rational_runout upper={:?} work={:?}",
        out.fields()[0].absolute_curvature_upper,
        out.work()
    );
}
