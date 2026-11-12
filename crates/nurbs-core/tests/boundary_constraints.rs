use nurbs_core::{
    constrained_surface::{Constraint, Options, boundary},
    patches,
    surface_join::{Boundary, Decision},
};
fn plane() -> nurbs_core::surface::Surface {
    patches::bezier(
        (0..7)
            .map(|i| {
                (0..7)
                    .map(|j| vec![i as f64 / 6., j as f64 / 6., 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap()
}
#[test]
fn interior_edit_preserves_entire_four_boundary_two_jets() {
    let source = plane();
    let before = source.clone();
    let constraints = [Constraint::Point {
        uv: [0.5, 0.5],
        point: [0.5, 0.5, 0.1],
    }];
    let qualified = boundary::solve_preserving_g2(
        &source,
        &source,
        &[
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ],
        &constraints,
        Options::default(),
        [1e-8, 1e-6, 1e-6],
        10000,
    )
    .unwrap();
    assert_eq!(source, before);
    assert!((qualified.surface().evaluate(0.5, 0.5).unwrap().point[2] - 0.1).abs() < 1e-8);
    assert_eq!(qualified.seams().len(), 4);
    for (_, seam) in qualified.seams() {
        assert_eq!(seam.decision, Decision::WithinTolerance);
        assert!(seam.regularity_proven);
    }
    for t in [0., 0.017, 0.31, 0.5, 0.932, 1.] {
        for uv in [[0., t], [1., t], [t, 0.], [t, 1.]] {
            let a = qualified.surface().evaluate(uv[0], uv[1]).unwrap();
            let b = source.evaluate(uv[0], uv[1]).unwrap();
            assert_eq!(a.point, b.point);
            assert_eq!(a.first_derivatives(), b.first_derivatives());
            assert_eq!(a.second_derivatives(), b.second_derivatives());
        }
    }
}
#[test]
fn conflicting_boundary_jets_and_bad_work_are_refused() {
    let source = plane();
    assert!(
        boundary::solve_preserving_g2(
            &source,
            &source,
            &[Boundary::UMin],
            &[Constraint::Point {
                uv: [0., 0.5],
                point: [0., 0.5, 1.]
            }],
            Options::default(),
            [1e-8; 3],
            1000
        )
        .is_err()
    );
    assert!(
        boundary::solve_preserving_g2(
            &source,
            &source,
            &[Boundary::UMin],
            &[],
            Options::default(),
            [1e-8; 3],
            0
        )
        .is_err()
    );
    let mut other = source.clone();
    other.weights[0][0] = 2.;
    assert!(
        boundary::solve_preserving_g2(
            &source,
            &other,
            &[Boundary::UMin],
            &[],
            Options::default(),
            [1e-8; 3],
            1000
        )
        .is_err()
    );
    assert!(
        nurbs_core::constrained_surface::solve(
            &source,
            &[Constraint::ControlPoint {
                index: [999, 0],
                point: [0.; 3]
            }],
            Options::default()
        )
        .is_err()
    );
}

#[test]
fn interior_constraints_keep_full_g2_on_four_original_noniso_support_traces() {
    use nurbs_core::{
        corner_blend::{self, mapped::Support},
        curve::Curve,
        surface::Surface,
        surface_blend,
    };
    let source = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![1., -1., 0.]],
            vec![vec![1., 1., 0.], vec![2., 0., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let curves = [
        ([0.25, 0.5], [0.5, 0.25]),
        ([0.5, 0.75], [0.75, 0.5]),
        ([0.25, 0.5], [0.5, 0.75]),
        ([0.5, 0.25], [0.75, 0.5]),
    ]
    .map(|(a, b)| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap());
    let directions = [
        [-0.125, -0.125],
        [0.125, 0.125],
        [-0.125, 0.125],
        [0.125, -0.125],
    ];
    let supports = std::array::from_fn(|i| Support {
        surface: &source,
        pcurve: &curves[i],
        direction: directions[i],
    });
    let corner_options = corner_blend::Options {
        boundaries: [surface_blend::Boundary::VStart; 4],
        reverse_along: [false; 4],
        gains: [2.; 4],
        compatibility_tolerance: 1e-9,
        max_controls: 256,
    };
    let work = boundary::MappedWork {
        max_cells: 100000,
        max_coefficients: 2000000,
        max_exact: 1000000,
    };
    let result = boundary::solve_from_mapped_supports(
        supports,
        corner_options,
        &[Constraint::Point {
            uv: [0.5; 2],
            point: [1., 0., 0.03],
        }],
        Options::default(),
        [1e-7, 1e-5, 1e-4],
        work,
    )
    .unwrap();
    assert!(result.admission().admitted && result.admission().direct_admitted);
    assert_eq!(result.admission().direct_original_seams.len(), 4);
    assert!(result.admission().corner.interior.certified);
    let replay = result.reverify(work).unwrap();
    assert!(replay.admitted && replay.direct_admitted);
    assert_eq!(result.original_supports()[0].surface, &source);
    assert!((result.surface().evaluate(0.5, 0.5).unwrap().point[2] - 0.03).abs() < 1e-8);
}
