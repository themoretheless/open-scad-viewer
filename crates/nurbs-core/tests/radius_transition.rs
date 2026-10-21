use nurbs_core::{
    curve::Curve,
    skin_radius,
    surface::{Axis, Surface},
    surface_blend, surface_collar,
};
fn cylinder(radius: f64, z: [f64; 2]) -> Surface {
    let points = [[radius, 0.], [radius, radius], [0., radius]];
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: points
            .iter()
            .map(|p| z.iter().map(|&z| vec![p[0], p[1], z]).collect())
            .collect(),
        weights: vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn different_radius_quarter_skins_join_with_whole_original_g2_and_radius_law() {
    let width = 0.1;
    let a = cylinder(0.1, [-width, 0.]);
    let b = cylinder(0.3, [width, 2. * width]);
    let built = surface_blend::g2(&a, &b, Default::default()).unwrap();
    assert_eq!(built.patches.len(), 1);
    let surface = &built.patches[0];
    let mut cells = 0;
    let mut coefficients = 0;
    for (original, at, direction, reverse) in [(&a, 1., [0., -1.], false), (&b, 0., [0., 1.], true)]
    {
        let pcurve = Curve::from_polyline(vec![vec![0., at], vec![1., at]]).unwrap();
        let candidate = if reverse {
            surface.edit_axis(Axis::V, |c| c.reverse()).unwrap()
        } else {
            surface.clone()
        };
        let proof = surface_collar::inspect_g2_opposite_cross(
            original,
            &pcurve,
            direction,
            &candidate,
            [1e-7, 1e-5, 1e-4],
            100000 - cells,
            2000000 - coefficients,
        )
        .unwrap();
        println!(
            "seam {:?} cells={} coefficients={} upper={:?}",
            proof.decision, proof.cells, proof.coefficient_work, proof.upper
        );
        assert_eq!(
            proof.orientation,
            Some(if reverse {
                nurbs_core::surface_g2::Orientation::Opposite
            } else {
                nurbs_core::surface_g2::Orientation::Same
            })
        );
        cells += proof.cells;
        coefficients += proof.coefficient_work;
        assert_eq!(
            proof.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
    }
    let edge_uv = Curve::from_polyline(vec![vec![0., 1.], vec![1., 1.]]).unwrap();
    assert!(
        surface_collar::inspect_g2_opposite_cross(
            &a,
            &edge_uv,
            [0., -1. - f64::EPSILON],
            surface,
            [1e-7, 1e-5, 1e-4],
            100000,
            2000000
        )
        .is_err()
    );
    let center = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., width]]).unwrap();
    let radius = Curve {
        degree: 5,
        knots: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
        control_points: [0.1, 0.1, 0.1, 0.3, 0.3, 0.3]
            .iter()
            .map(|&r| vec![r, 0., 0.])
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    let proof = skin_radius::inspect(
        surface,
        &center,
        &radius,
        Axis::V,
        1e-7,
        2000000 - coefficients,
    )
    .unwrap();
    println!(
        "radius {:?} work={}",
        proof.radial_error_upper_mm, proof.coefficient_work
    );
    assert!(proof.within_tolerance);
    let injective =
        nurbs_core::surface_linear_monotonicity::inspect_candidate(surface, 100000 - cells)
            .unwrap();
    println!("injective={:?}", injective);
    assert!(injective.certified);
}

#[test]
fn authored_transition_factory_qualifies_rotated_small_width_and_rechecks_damage() {
    use nurbs_core::radius_transition::{self as transition, Options};
    for o in [
        Options::default(),
        Options {
            origin_mm: [2., -3., 1.],
            axis: [0.37_f64.sin(), 0., 0.37_f64.cos()],
            radial: [0.37_f64.cos(), 0., -0.37_f64.sin()],
            ..Default::default()
        },
    ] {
        let built = transition::build(o).unwrap();
        println!(
            "factory admitted={} cells={} coefficients={} radius={:?} seams={:?}",
            built.admitted,
            built.cells,
            built.coefficient_work,
            built.radius.radial_error_upper_mm,
            built.seams.iter().map(|s| s.upper).collect::<Vec<_>>()
        );
        assert!(built.admitted);
        assert!(built.width_error_upper_mm <= o.tolerances[0]);
        assert!(built.station_plane_error_upper_mm <= o.tolerances[0]);
        assert!(
            built
                .circular_radius_error_upper_mm
                .is_some_and(|e| e <= o.tolerances[0])
        );
        assert!(built.cells <= o.max_cells && built.coefficient_work <= o.max_coefficients);
        assert_eq!(built.radius_law.control_points[0][0], 0.1);
        assert_eq!(built.radius_law.control_points[5][0], 0.3);
        let saved = built.surface.clone();
        assert!(transition::inspect_candidate(&saved, o).unwrap().admitted);
        let mut damaged = saved.clone();
        damaged.control_points[1][2][0] += 0.001;
        assert!(!transition::inspect_candidate(&damaged, o).is_ok_and(|r| r.admitted));
        assert_eq!(built.surface, saved);
    }
}
#[test]
fn transition_bad_input_and_missing_work_never_admit_a_skin() {
    use nurbs_core::radius_transition::{self as t, Options};
    for o in [
        Options {
            width_mm: 0.,
            ..Default::default()
        },
        Options {
            radii_mm: [0., 0.3],
            ..Default::default()
        },
        Options {
            axis: [0.; 3],
            ..Default::default()
        },
        Options {
            radial: [0., 0., 1.],
            ..Default::default()
        },
        Options {
            max_cells: 1,
            ..Default::default()
        },
        Options {
            max_coefficients: 24,
            ..Default::default()
        },
    ] {
        assert!(!t::build(o).is_ok_and(|r| r.admitted));
    }
}

#[test]
fn an_interior_radius_defect_cannot_inherit_good_endpoint_g2() {
    use nurbs_core::radius_transition::{self as t, Options};
    let o = Options::default();
    let good = t::build(o).unwrap();
    let mut damaged = good.surface.edit_axis(Axis::V, |c| c.elevate(7)).unwrap();
    damaged.control_points[1][3][0] += 0.001;
    let checked = t::inspect_candidate(&damaged, o).unwrap();
    assert!(
        checked
            .seams
            .iter()
            .all(|s| s.decision == nurbs_core::surface_join::Decision::WithinTolerance)
    );
    assert!(!checked.radius.within_tolerance && !checked.admitted);
}

#[test]
fn distance_to_center_does_not_replace_the_authored_station_plane() {
    use nurbs_core::radius_transition::{self as t, Options};
    let o = Options::default();
    let good = t::build(o).unwrap();
    let mut tilted = good.surface.clone();
    let angle = 0.4_f64;
    for row in &mut tilted.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            let center_z = o.width_mm * j as f64 / 5.;
            let x = p[0];
            let z = p[2] - center_z;
            p[0] = angle.cos() * x + angle.sin() * z;
            p[2] = center_z - angle.sin() * x + angle.cos() * z;
        }
    }
    let checked = t::inspect_candidate(&tilted, o).unwrap();
    assert!(checked.radius.within_tolerance);
    assert!(checked.station_plane_error_upper_mm > o.tolerances[0]);
    assert!(!checked.admitted);
}

#[test]
fn a_signed_affine_cross_gain_preserves_physical_curvature_and_rejects_damage() {
    let a = cylinder(0.1, [-0.1, 0.]);
    let b = cylinder(0.3, [0.1, 0.2]);
    let built = surface_blend::g2(&a, &b, Default::default()).unwrap();
    let uv = Curve::from_polyline(vec![vec![0., 1.], vec![1., 1.]]).unwrap();
    for gain in [-0.5, -0.3] {
        let proof = surface_collar::inspect_g2_cross_frame(
            &a,
            &uv,
            [0., gain],
            &built.patches[0],
            gain,
            [1e-7, 1e-5, 1e-4],
            100000,
            2000000,
        )
        .unwrap();
        assert_eq!(
            proof.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert_eq!(
            proof.orientation,
            Some(nurbs_core::surface_g2::Orientation::Same)
        );
        let mut damaged = built.patches[0].clone();
        damaged.control_points[1][2][0] += 0.001;
        let bad = surface_collar::inspect_g2_cross_frame(
            &a,
            &uv,
            [0., gain],
            &damaged,
            gain,
            [1e-7, 1e-5, 1e-4],
            100000,
            2000000,
        )
        .unwrap();
        assert_ne!(
            bad.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
    }
    for gain in [0., f64::NAN] {
        assert!(
            surface_collar::inspect_g2_cross_frame(
                &a,
                &uv,
                [0., -0.5],
                &built.patches[0],
                gain,
                [1e-7, 1e-5, 1e-4],
                100000,
                2000000
            )
            .is_err()
        );
    }
}
