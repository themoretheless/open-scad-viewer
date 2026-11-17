use nurbs_core::{
    analysis::{
        planar_boundary_distance::distance_to_projected_face_boundary,
        planar_trim_projection::{PlanarTrimOutcome, project_coplanar_trim_loops},
    },
    curve::Curve,
    surface::Surface,
};
fn contour(a: f64, b: f64, reverse: bool) -> Vec<Curve> {
    let mut points = vec![vec![a, a], vec![b, a], vec![b, b], vec![a, b], vec![a, a]];
    if reverse {
        points.reverse();
    }
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: points,
        weights: vec![1.; 5],
        periodic: false,
    }]
}

fn face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn projected(reverse: bool) -> nurbs_core::analysis::planar_trim_projection::ProjectedTrimmedFace {
    let report = project_coplanar_trim_loops(
        &[face()],
        &[vec![
            contour(0., 1., reverse),
            contour(0.25, 0.75, !reverse),
        ]],
        1_000_000,
    )
    .unwrap();
    let PlanarTrimOutcome::Projected(mut charts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    charts.remove(0)
}

fn contains(bounds: [f64; 2], value: f64) {
    assert!(
        bounds[0] <= value && value <= bounds[1],
        "{bounds:?} excludes {value}"
    );
}

#[test]
fn certifies_a_normal_offset_from_the_inner_boundary() {
    let face = projected(false);
    let report = distance_to_projected_face_boundary(&face, [0., 0.25, 1.], 1e-5, 16_384).unwrap();
    assert!(report.converged, "{:?}", report.distance);
    contains(report.distance, 3_f64.sqrt() / 4.);
    assert_eq!(report.witness.contour, 1);
    assert!(report.cells <= 16_384);
}

#[test]
fn seeds_a_witness_at_an_exact_outer_boundary_corner() {
    let face = projected(true);
    let report = distance_to_projected_face_boundary(&face, [0., 0., 0.], 1e-10, 8).unwrap();
    assert!(report.converged);
    contains(report.distance, 0.);
    assert_eq!(report.cells, 8);
    assert_eq!(report.witness.contour, 0);
    for coordinate in report.witness.point_bounds {
        contains(coordinate, 0.);
    }
}

#[test]
fn keeps_a_global_enclosure_at_the_initial_cell_budget() {
    let face = projected(false);
    let point = [0., 0.25, 1.];
    assert!(distance_to_projected_face_boundary(&face, point, 1e-9, 7).is_err());
    let report = distance_to_projected_face_boundary(&face, point, 1e-9, 8).unwrap();
    assert!(!report.converged);
    assert_eq!(report.cells, 8);
    contains(report.distance, 3_f64.sqrt() / 4.);
}

#[test]
fn uses_plane_distance_for_a_proved_interior_projection() {
    use nurbs_core::{
        analysis::planar_boundary_distance::{
            PlanarFaceDistanceOptions, distance_to_projected_face,
        },
        trim::trim_domain::Location,
    };
    let face = projected(false);
    // The projection is [1/8, 1/8, 1/4], offset by [-1,-1,1]/4.
    let result = distance_to_projected_face(
        &face,
        [-0.125, -0.125, 0.5],
        PlanarFaceDistanceOptions {
            tolerance: 1e-9,
            winding_tolerance: 1e-9,
            max_winding_cells: 1024,
            max_boundary_cells: 8,
        },
    )
    .unwrap();
    assert_eq!(result.projection_classification.location, Location::Inside);
    assert!(result.converged && result.boundary.is_none());
    contains(result.distance, 3_f64.sqrt() / 4.);
    for (range, coord) in result
        .orthogonal_projection
        .into_iter()
        .zip([0.125, 0.125, 0.25])
    {
        contains(range, coord);
    }
}

#[test]
fn certifies_hole_exclusion_and_keeps_bounds_when_winding_is_unresolved() {
    use nurbs_core::{
        analysis::planar_boundary_distance::{
            PlanarFaceDistanceOptions, distance_to_projected_face,
        },
        trim::trim_domain::Location,
    };
    let face = projected(false);
    let opts = PlanarFaceDistanceOptions {
        tolerance: 1e-5,
        winding_tolerance: 1e-9,
        max_winding_cells: 1024,
        max_boundary_cells: 16_384,
    };
    let point = [0.5, 0.5, 1.];
    let result = distance_to_projected_face(&face, point, opts).unwrap();
    let expected = 3_f64.sqrt() / (4. * 2_f64.sqrt());
    assert_eq!(result.projection_classification.location, Location::Outside);
    assert!(result.converged);
    contains(result.distance, expected);
    assert_eq!(result.boundary.unwrap().witness.contour, 1);
    let result = distance_to_projected_face(
        &face,
        point,
        PlanarFaceDistanceOptions {
            max_winding_cells: 1,
            max_boundary_cells: 8,
            ..opts
        },
    )
    .unwrap();
    assert_eq!(
        result.projection_classification.location,
        Location::Unresolved
    );
    contains(result.distance, expected);
    assert!(!result.converged);
}
