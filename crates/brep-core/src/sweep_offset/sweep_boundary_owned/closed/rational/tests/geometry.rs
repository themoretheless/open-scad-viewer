use super::*;

#[test]
fn original_correlated_center_motion_owns_caps_and_global_g2_without_reference() {
    let (profiles, material, mut motion) = fixture(false);
    for pose in motion.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-0.5 * pose[0][0], -0.5 * pose[0][1], 0.25];
    }
    let b = body(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            limits(),
        )
        .unwrap(),
    );
    assert_eq!(b.report().caps.len(), 2);
    assert!(b.report().source.g2);
    assert_eq!(
        b.report()
            .proof_coordinates
            .as_ref()
            .unwrap()
            .column_factor
            .as_ref()
            .unwrap()
            .dependent,
        3
    );
    assert_eq!(b.joint_certificate(2).unwrap().boundary_error_upper(), 0.);
    let mut no_global = limits();
    no_global.volume.cells = 0;
    assert!(matches!(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            no_global
        )
        .unwrap(),
        RationalClosedAttempt::Refused(_)
    ));
}

#[test]
fn carrier_axis_outside_material_owns_original_global_g2() {
    let (profiles, material, mut motion) = fixture(false);
    for pose in motion.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-0.875 * pose[0][0], -0.875 * pose[0][1], 0.25];
    }
    let b = body(
        construct_rational_closed_material(
            &profiles,
            &material,
            [0.; 3],
            &motion,
            &[[0, 2], [0, 6]],
            1e-8,
            limits(),
        )
        .unwrap(),
    );
    assert_eq!(b.report().material_uv_bounds, Some([[1., 5.], [-2., 2.]]));
    let w = b.report().proof_uv_window;
    assert!(w[0][0] > 0. && w[0][1] < 1.);
    assert_eq!(b.joint_certificate(2).unwrap().boundary_error_upper(), 0.);
    assert_eq!(b.report().caps.len(), 2);
}

#[test]
fn sheared_uv_window_singularity_outside_material_uses_exact_original_plane_chart() {
    let (profiles, mut material, mut motion) = fixture(false);
    for curve in material.trims.iter_mut().flatten() {
        for p in &mut curve.control_points {
            p[0] += p[1];
        }
    }
    material.surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-4., -4., 10., 10.],
        knots_v: vec![-4., -4., 4., 4.],
        control_points: vec![
            vec![vec![0., -4., 0.], vec![-8., 4., 0.]],
            vec![vec![14., -4., 0.], vec![6., 4., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    for pose in motion.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-0.875 * pose[0][0], -0.875 * pose[0][1], 0.25];
    }
    for reversed in [false, true] {
        let mut material = material.clone();
        if reversed {
            material.surface.control_points.reverse();
            material.surface.weights.reverse();
            for c in material.trims.iter_mut().flatten() {
                for p in &mut c.control_points {
                    p[0] = 6. - p[0];
                }
            }
        }
        let b = body(
            construct_rational_closed_material(
                &profiles,
                &material,
                [0.; 3],
                &motion,
                &[[0, 2], [0, 6]],
                1e-8,
                limits(),
            )
            .unwrap(),
        );
        let uv = b.report().material_uv_bounds.unwrap();
        assert!(uv[0][0] < 0.875 && uv[0][1] > 0.875 && uv[1][0] < 0. && uv[1][1] > 0.);
        let chart = b.report().material_plane_chart.as_ref().unwrap();
        assert_eq!(chart.axes, [0, 1]);
        assert!(if reversed {
            chart.determinant[1] < 0.
        } else {
            chart.determinant[0] > 0.
        });
        assert!(chart.bounds[0][0] > 0.875 && chart.bounds[0][1] >= 5.);
        assert_eq!(b.joint_certificate(2).unwrap().boundary_error_upper(), 0.);
        let mut short = limits();
        short.source_exact_work = b.report().work.source_exact_work - 1;
        assert!(matches!(
            construct_rational_closed_material(
                &profiles,
                &material,
                [0.; 3],
                &motion,
                &[[0, 2], [0, 6]],
                1e-8,
                short
            )
            .unwrap(),
            RationalClosedAttempt::Refused(_)
        ));
    }
}
