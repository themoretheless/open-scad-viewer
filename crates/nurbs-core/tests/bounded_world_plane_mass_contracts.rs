use nurbs_core::{
    analysis::{
        bounded_planar_face_mass::bound_projected_face_mass,
        exact_affine_hull::{PlanarProjectionOutcome, project_planar_surface},
        planar_trim_projection::ProjectedTrimmedFace,
    },
    curve::Curve,
    surface::Surface,
};

fn ellipse() -> Vec<Curve> {
    let poles = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    (0..4)
        .map(|quarter| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: poles[2 * quarter..=2 * quarter + 2]
                .iter()
                .map(|&[x, y]| vec![3. + 2. * x, -2. + y])
                .collect(),
            weights: vec![1., 1., 2.],
            periodic: false,
        })
        .collect()
}

fn face() -> ProjectedTrimmedFace {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [1., 5.]
            .into_iter()
            .map(|x| {
                [-3., -1.]
                    .into_iter()
                    .map(|y| vec![x, y, 5. + 0.5 * x + 0.25 * y])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let report = project_planar_surface(&surface, 1_000_000).unwrap();
    let PlanarProjectionOutcome::Projected(projection) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    ProjectedTrimmedFace {
        projection,
        loops: vec![ellipse()],
    }
}

fn contains(bounds: [f64; 2], expected: f64) {
    assert!(
        bounds[0] <= expected && expected <= bounds[1],
        "{bounds:?} excludes {expected}"
    );
}

#[test]
fn encloses_world_centroid_and_all_tensor_entries_of_a_tilted_rational_ellipse() {
    let face = face();
    let mass = bound_projected_face_mass(&face, [3., -2.], [0.02; 3], 8192).unwrap();
    assert!(mass.projected_converged && mass.cells <= 8192);
    let scale = 21_f64.sqrt() / 4.;
    let pi = std::f64::consts::PI;
    contains(mass.signed_area, 2. * pi * scale);
    let centroidal = mass.centroidal.unwrap();
    for (interval, position) in centroidal.point.into_iter().zip([3., -2., 6.]) {
        contains(interval, position);
        assert!(interval[1] - interval[0] <= 0.02);
    }
    let expected = [[2., 0., 1.], [0., 0.5, 0.125], [1., 0.125, 17. / 32.]];
    for (row, expected) in centroidal.second_moments.into_iter().zip(expected) {
        for (interval, moment) in row.into_iter().zip(expected) {
            contains(interval, moment * pi * scale);
        }
    }
}

#[test]
fn withholds_centroid_when_opposite_windings_cancel_area() {
    let mut face = face();
    let opposite: Vec<_> = face.loops[0]
        .iter()
        .rev()
        .map(|curve| curve.reverse().unwrap())
        .collect();
    face.loops.push(opposite);
    let result = bound_projected_face_mass(&face, [3., -2.], [0.1; 3], 4096).unwrap();
    contains(result.signed_area, 0.);
    assert!(result.centroidal.is_none());
    assert!(result.cells <= 4096);
}
