//! Independent analytic oracles for sampled surface-quality diagnostics.
use nurbs_core::{
    analysis::class_a::{Limits, highlight_lines, isophote_analysis, reflection_lines},
    curve::Curve,
    surface::{Surface, revolve},
};

fn unit_sphere() -> Surface {
    let profile = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        control_points: vec![
            vec![0., 0., -1.], vec![1., 0., -1.], vec![1., 0., 0.],
            vec![1., 0., 1.], vec![0., 0., 1.],
        ],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
}

fn check_latitudes(surface: &Surface, lines: &[Vec<[f64; 2]>], expected: impl Fn(f64) -> f64) {
    assert!(lines.iter().map(Vec::len).sum::<usize>() > 10);
    for uv in lines.iter().flatten() {
        let p = surface.evaluate(uv[0], uv[1]).unwrap().point;
        // Independent implicit unit-sphere equation, unrelated to extracted UV chains.
        assert!((p.iter().map(|v| v * v).sum::<f64>() - 1.).abs() < 1e-10);
        // Marching squares interpolates the scalar level, so this is a sampled
        // resolution check rather than an asserted continuous error bound.
        assert!(expected(p[2]).abs() < 0.01, "wrong latitude at {uv:?}: {p:?}");
    }
}

#[test]
fn highlights_follow_the_analytic_reflection_latitudes() {
    let sphere = unit_sphere();
    let level = 0.17;
    let reports = highlight_lines(&sphere, &[[0., 0., 1.]], [0., 0., 1.], level, [64, 64], 0.35, &Limits::default()).unwrap();
    assert_eq!(reports.len(), 1);
    // Reflection of +Z about either orientation of the unit normal p has
    // Z component 2*z*z - 1. Both hemispheres are legitimate components.
    check_latitudes(&sphere, &reports[0].polylines, |z| 2. * z * z - 1. - level);
}

#[test]
fn reflection_report_retains_the_analytic_level_curve() {
    let sphere = unit_sphere();
    let level = 0.17;
    let (report, jumps) = reflection_lines(&sphere, [0., 0., 1.], [0., 0., 1.], level, [64, 64], 200., &Limits::default()).unwrap();
    check_latitudes(&sphere, &report.polylines, |z| 2. * z * z - 1. - level);
    assert!(jumps.iter().all(|d| d.value.is_finite() && d.value > d.threshold));
}

#[test]
fn isophotes_follow_the_oriented_sphere_normal() {
    let sphere = unit_sphere();
    let levels = [0.23, 0.47, 0.71];
    let report = isophote_analysis(&sphere, [0., 0., 1.], &levels, [64, 64], 0.9, &Limits::default()).unwrap();
    assert_eq!(report.levels.len(), levels.len());
    // This profile goes from south to north and revolution is positive:
    // S_u cross S_v points inward, hence normal dot +Z = -z.
    for (line, level) in report.levels.iter().zip(levels) {
        check_latitudes(&sphere, &line.polylines, |z| -z - level);
    }
    assert!(report.mean_spacing.is_finite() && report.mean_spacing > 0.);
}

fn quadratic_plate(dent: f64) -> Surface {
    let knots = vec![0., 0., 0., 4./3., 8./3., 4., 4., 4.];
    Surface {
        degree_u: 2, degree_v: 2,
        knots_u: knots.clone(), knots_v: knots,
        control_points: (0..5).map(|i| (0..5).map(|j| vec![i as f64, j as f64, if i==2 && j==2 {dent} else {0.}]).collect()).collect(),
        weights: vec![vec![1.;5];5], periodic_u:false, periodic_v:false,
    }
}

#[test]
fn all_hardened_scalar_settings_reject_non_finite_values() {
    use nurbs_core::analysis::class_a::dent_detection;
    let s=unit_sphere();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            reflection_lines(&s,[1.,0.,1.],[0.,0.,1.],0.5,[4,4],bad,&Limits::default()).map(|_|()),
            isophote_analysis(&s,[0.,0.,1.],&[0.25,0.5],[4,4],bad,&Limits::default()).map(|_|()),
            dent_detection(&s,[4,4],bad,0.01,&Limits::default()).map(|_|()),
            dent_detection(&s,[4,4],1.,bad,&Limits::default()).map(|_|()),
        ] {assert_eq!(result.unwrap_err().code,"NURBS_INVALID_INPUT");}
    }
}

#[test]
fn malformed_surfaces_return_errors_through_the_public_api() {
    use nurbs_core::analysis::class_a::dent_detection;
    let mut s=unit_sphere();s.knots_u.clear();
    assert_eq!(highlight_lines(&s,&[[1.,0.,1.]],[0.,0.,1.],0.5,[4,4],0.35,&Limits::default()).unwrap_err().code,"NURBS_INVALID_INPUT");
    assert_eq!(dent_detection(&s,[4,4],1.,0.01,&Limits::default()).unwrap_err().code,"NURBS_INVALID_INPUT");
}

#[test]
fn extreme_finite_sigma_keeps_a_plane_flat_without_unbounded_support() {
    use nurbs_core::analysis::class_a::dent_detection;
    let s=quadratic_plate(0.);
    for sigma in [1e100,f64::MAX] {
        let report=dent_detection(&s,[4,6],sigma,1e-12,&Limits::default()).unwrap();
        assert_eq!(report.max_residual,0.);assert!(report.zones.is_empty());
    }
}

#[test]
fn dent_zones_localize_an_independent_control_net_perturbation() {
    use nurbs_core::analysis::class_a::dent_detection;
    let flat=dent_detection(&quadratic_plate(0.),[24,24],1.5,1e-6,&Limits::default()).unwrap();
    assert!(flat.max_residual<1e-6);assert!(flat.zones.is_empty());
    let dent=dent_detection(&quadratic_plate(0.4),[24,24],1.5,0.02,&Limits::default()).unwrap();
    assert!(!dent.zones.is_empty());
    assert!(dent.zones.iter().any(|zone|(zone.uv[0]-2.).abs()<1. && (zone.uv[1]-2.).abs()<1.));
}
