use brep_core::profile_certificate as p;
use nurbs_core::{curve::Curve, surface::Surface};
use std::sync::Arc;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 2],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn arch(height: f64) -> Vec<Curve> {
    vec![
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![1.; 3],
            control_points: vec![vec![0., 0.], vec![0.5, height], vec![1., 0.]],
            periodic: false,
        },
        Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap(),
    ]
}
fn topology(wire: Vec<Curve>) -> Arc<p::OwnedOriginalPeriodicTopology> {
    let regions = Arc::new(
        p::assemble_original_filled_regions(
            &[wire],
            &[1],
            1e-12,
            Default::default(),
            Default::default(),
        )
        .unwrap(),
    );
    let chart = Arc::new(
        p::own_original_chart_vertices(
            regions.owner().cycles().graph().clone(),
            [[0., 1.], [0., 1.]],
            [false, false],
            Default::default(),
        )
        .unwrap(),
    );
    let dictionary =
        Arc::new(p::own_original_seam_dictionary(regions, chart, Default::default()).unwrap());
    let seams = Arc::new(p::bind_original_seams_to_surface(dictionary, &plane()).unwrap());
    Arc::new(p::own_original_periodic_topology(seams).unwrap())
}
#[test]
fn exact_interior_extremum_admits_curve_even_when_controls_are_outside_chart() {
    let t = topology(arch(2.));
    let coverage = p::own_original_trim_chart_coverage(t, Default::default()).unwrap();
    coverage.reverify_source().unwrap();
    assert_eq!(coverage.cells().len(), 2);
    assert!(coverage.cells().iter().any(|c| c.critical_roots[3] > 0));
}
#[test]
fn endpoint_admission_cannot_hide_a_tiny_actual_interior_excursion() {
    let t = topology(arch(2. + 2f64.powi(-30)));
    assert!(p::own_original_trim_chart_coverage(t, Default::default()).is_err());
}
#[test]
fn rational_weights_use_actual_homogeneous_inequality_extrema() {
    let mut wire = arch(1.5);
    wire[0].weights = vec![1., 2., 1.];
    let coverage = p::own_original_trim_chart_coverage(topology(wire), Default::default()).unwrap();
    coverage.reverify_source().unwrap();
    assert!(
        coverage
            .cells()
            .iter()
            .any(|c| c.critical_roots.iter().sum::<usize>() > 0)
    );
}
