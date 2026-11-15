use nurbs_core::{
    analysis::exact_affine_hull::{SurfaceAffineHull, classify_surface_affine_hull},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face(points: [[f64; 3]; 4]) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: points
            .chunks_exact(2)
            .map(|row| row.iter().map(|p| p.to_vec()).collect())
            .collect(),
        weights: vec![vec![1., 2.], vec![4., 0.5]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn classify(patch: &Surface) -> SurfaceAffineHull {
    classify_surface_affine_hull(patch, WORK).unwrap().hull
}

#[test]
fn distinguishes_point_line_plane_and_a_tiny_out_of_plane_pole() {
    let a = [0., 0., 0.];
    let b = [1., 0., 2.];
    let c = [0., 1., 3.];
    assert_eq!(classify(&face([a; 4])), SurfaceAffineHull::Point(a));
    assert_eq!(
        classify(&face([a, b, [2., 0., 4.], a])),
        SurfaceAffineHull::Line([a, b])
    );
    assert_eq!(
        classify(&face([a, b, c, [1., 1., 5.]])),
        SurfaceAffineHull::Plane([a, b, c])
    );
    let x = [1., 0., 0.];
    let y = [0., 1., 0.];
    let tiny = [1., 1., 1e-250];
    assert_eq!(
        classify(&face([a, x, y, tiny])),
        SurfaceAffineHull::Spatial([a, x, y, tiny])
    );
}

#[test]
fn excludes_poles_with_only_exterior_support() {
    let mut patch = face([[0., 0., 0.], [0., 1., 1.], [1., 0., 1.], [1., 1., 2.]]);
    patch.knots_u = vec![-1., 0., 0., 1., 2., 2.];
    patch
        .control_points
        .insert(0, vec![vec![100., 100., -100.]; 2]);
    patch.weights.insert(0, vec![1., 2.]);
    patch
        .control_points
        .push(vec![vec![2., 0., 2.], vec![2., 1., 3.]]);
    patch.weights.push(vec![0.5, 2.]);
    assert_eq!(
        classify(&patch),
        SurfaceAffineHull::Plane([[0., 0., 0.], [0., 1., 1.], [1., 0., 1.]])
    );
}

#[test]
fn low_work_budget_withholds_the_hull() {
    let patch = face([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]]);
    let report = classify_surface_affine_hull(&patch, 1).unwrap();
    assert_eq!(report.hull, SurfaceAffineHull::ResourceLimit);
    assert!(report.work_used <= 1);
    assert!(classify_surface_affine_hull(&patch, 0).is_err());
}
