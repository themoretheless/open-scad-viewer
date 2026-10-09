use nurbs_core::{Result, continuity, surface::Surface};

fn patch(offset: f64) -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..4)
            .map(|u| {
                (0..4)
                    .map(|v| vec![offset + u as f64, v as f64, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
fn main() -> Result<()> {
    let result = continuity::match_surface_jets_checked_report(
        &patch(0.),
        &patch(3.),
        "uMax",
        "uMin",
        2,
        1.,
        false,
        1e-8,
    )?;
    assert!(result.report.regularity_certified());
    let decision = result
        .report
        .decision
        .as_ref()
        .expect("Checked result has a decision");
    assert!(decision.accepted);
    println!(
        "Regular seam; continuous jet error bound: {}",
        decision.error_upper
    );
    let reversed = continuity::match_surface_jets_oriented_report(
        &patch(0.),
        &patch(3.),
        "uMax",
        "uMin",
        1,
        1.,
        true,
    )?;
    assert!(reversed.report.reversed);
    assert!(reversed.report.decision.is_none());
    let prepared = continuity::preparation::checked_report(
        &patch(0.),
        &patch(3.),
        "uMax",
        "uMin",
        false,
        1e-8,
    )?;
    assert!(prepared.report.accepted);
    assert_eq!(prepared.basis.degree, 3);
    let first = nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]])?;
    let second = nurbs_core::curve::Curve::from_polyline(vec![vec![4., 2., 0.], vec![5., 3., 0.]])?;
    let matched = continuity::curve_match::checked_report(&first, &second, "end", "start", 1e-8)?;
    assert!(matched.report.accepted);
    assert_eq!(matched.curve.control_points[0], vec![1., 0., 0.]);
    Ok(())
}
