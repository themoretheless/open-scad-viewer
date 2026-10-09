use super::*;

fn section(contours: Vec<Vec<[f64; 2]>>) -> LayerSection {
    LayerSection {
        z_mm: 0.,
        contours,
    }
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-9 * b.abs().max(1.), "{a} != {b}");
}

#[test]
fn rectangle_principal_axes_match_handbook() {
    // 40 x 20 rectangle centered at the origin, CCW. The maximum moment
    // is about the vertical axis (material spreads along x).
    let r = principal_axes(&section(vec![vec![
        [-20., -10.],
        [20., -10.],
        [20., 10.],
        [-20., 10.],
    ]]))
    .unwrap();
    close(r.i1_mm4, 20. * 40f64.powi(3) / 12.);
    close(r.i2_mm4, 40. * 20f64.powi(3) / 12.);
    close(r.angle_rad, std::f64::consts::FRAC_PI_2);
    close(r.area_mm2, 800.);
    close(r.r1_mm, (r.i1_mm4 / 800.).sqrt());
    close(r.w1_mm3, r.i1_mm4 / 20.);
    close(r.w2_mm3, r.i2_mm4 / 10.);
    assert!(!r.isotropic);
    assert_eq!(r.centroid, [0., 0.]);
}

#[test]
fn rotated_rectangle_keeps_moments_and_reports_rotation() {
    let angle = std::f64::consts::FRAC_PI_4;
    let (c, s) = (angle.cos(), angle.sin());
    let rect = [[-20., -10.], [20., -10.], [20., 10.], [-20., 10.]]
        .map(|[x, y]| [x * c - y * s, x * s + y * c]);
    let r = principal_axes(&section(vec![rect.to_vec()])).unwrap();
    close(r.i1_mm4, 20. * 40f64.powi(3) / 12.);
    close(r.i2_mm4, 40. * 20f64.powi(3) / 12.);
    // The maximum-moment axis is perpendicular to the long direction.
    close(r.angle_rad, -angle);
}

#[test]
fn symmetric_angle_section_points_at_45_degrees() {
    // Equal-leg L, symmetric about y = x.
    let l = vec![vec![
        [0., 0.],
        [10., 0.],
        [10., 2.],
        [2., 2.],
        [2., 10.],
        [0., 10.],
    ]];
    let r = principal_axes(&section(l.clone())).unwrap();
    close(r.angle_rad, std::f64::consts::FRAC_PI_4);
    // Invariants against the axes-aligned integrals.
    let base = crate::analyze_section(
        &section(l),
        &crate::LoadCase {
            moment_x_nmm: 0.,
            moment_y_nmm: 0.,
            axial_n: 0.,
            allowable_mpa: 1.,
        },
    )
    .unwrap();
    close(r.i1_mm4 + r.i2_mm4, base.ixx_mm4 + base.iyy_mm4);
    close(
        r.i1_mm4 * r.i2_mm4,
        base.ixx_mm4 * base.iyy_mm4 - base.ixy_mm4.powi(2),
    );
    assert!(r.i1_mm4 > r.i2_mm4);
}

#[test]
fn square_is_isotropic_and_tube_hole_subtracts() {
    let square = principal_axes(&section(vec![vec![
        [-5., -5.],
        [5., -5.],
        [5., 5.],
        [-5., 5.],
    ]]))
    .unwrap();
    assert!(square.isotropic);
    close(square.i1_mm4, 10. * 10f64.powi(3) / 12.);
    close(square.i2_mm4, square.i1_mm4);
    // Tube 100 x 50 with a 90 x 40 hole (opposite winding).
    let tube = principal_axes(&section(vec![
        vec![[-50., -25.], [50., -25.], [50., 25.], [-50., 25.]],
        vec![[-45., -20.], [-45., 20.], [45., 20.], [45., -20.]],
    ]))
    .unwrap();
    close(tube.i1_mm4, (50. * 100f64.powi(3) - 40. * 90f64.powi(3)) / 12.);
    close(tube.i2_mm4, (100. * 50f64.powi(3) - 90. * 40f64.powi(3)) / 12.);
    close(tube.area_mm2, 5000. - 3600.);
    assert!(!tube.isotropic);
}

#[test]
fn open_section_torsion_sums_strip_powers() {
    // I-beam approximation: two flanges 100 x 5, web 95 x 4.
    let j = torsion_constant_open(&[
        Strip {
            a: [0., 0.],
            b: [100., 0.],
            thickness_mm: 5.,
        },
        Strip {
            a: [0., 0.],
            b: [100., 0.],
            thickness_mm: 5.,
        },
        Strip {
            a: [0., 0.],
            b: [0., 95.],
            thickness_mm: 4.,
        },
    ])
    .unwrap();
    close(j, 2. * 100. * 125. / 3. + 95. * 64. / 3.);
}

#[test]
fn closed_cell_torsion_matches_bredt() {
    // Rectangular tube, centerline 100 x 50, wall 5.
    let j = torsion_constant_closed(
        &[[0., 0.], [100., 0.], [100., 50.], [0., 50.]],
        &[5.; 4],
    )
    .unwrap();
    close(j, 4. * 5000. * 5000. / 60.);
    // A closed tube dwarfs the same walls opened up.
    let open = torsion_constant_open(&[Strip {
        a: [0., 0.],
        b: [300., 0.],
        thickness_mm: 5.,
    }])
    .unwrap();
    assert!(j > 100. * open);
}

#[test]
fn validation_rejects_bad_geometry() {
    assert!(principal_axes(&section(vec![])).is_err());
    assert!(principal_axes(&section(vec![vec![[0., 0.], [1., 0.]]])).is_err());
    assert!(torsion_constant_open(&[]).is_err());
    assert!(
        torsion_constant_open(&[Strip {
            a: [0., 0.],
            b: [0., 0.],
            thickness_mm: 1.,
        }])
        .is_err()
    );
    assert!(
        torsion_constant_open(&[Strip {
            a: [0., 0.],
            b: [1., 0.],
            thickness_mm: -1.,
        }])
        .is_err()
    );
    assert!(torsion_constant_closed(&[[0., 0.], [1., 0.]], &[1.; 2]).is_err());
    assert!(
        torsion_constant_closed(&[[0., 0.], [1., 0.], [1., 1.]], &[1.; 2]).is_err()
    );
    // Collinear centerline: zero area.
    assert!(
        torsion_constant_closed(&[[0., 0.], [1., 0.], [2., 0.]], &[1.; 3]).is_err()
    );
}

fn strip(a: [f64; 2], b: [f64; 2], t: f64) -> Strip {
    Strip {
        a,
        b,
        thickness_mm: t,
    }
}

#[test]
fn thin_walled_i_beam_matches_closed_forms() {
    // Web 100 x 4 between flange centerlines, flanges 100 x 5; the web
    // endpoints split the flange strips at mid-span (T junctions).
    let r = thin_walled_open(&[
        strip([0., -50.], [0., 50.], 4.),
        strip([-50., -50.], [50., -50.], 5.),
        strip([-50., 50.], [50., 50.], 5.),
    ])
    .unwrap();
    close(r.area_mm2, 1400.);
    close(r.centroid[0], 0.);
    close(r.centroid[1], 0.);
    close(r.i1_mm4, 4. * 100f64.powi(3) / 12. + 2. * 5. * 100. * 2500.);
    close(r.i2_mm4, 2. * 5. * 100f64.powi(3) / 12.);
    close(r.j_mm4, (2. * 100. * 125. + 100. * 64.) / 3.);
    // Doubly symmetric: shear center coincides with the centroid, and
    // Cw = Iy·h²/4 with h the flange-centerline distance.
    assert!(r.shear_center[0].abs() < 1e-6);
    assert!(r.shear_center[1].abs() < 1e-6);
    close(r.cw_mm6, r.i2_mm4 * 100. * 100. / 4.);
    // Vertical shear rides the web; horizontal rides the flanges.
    assert!(r.shear_area_2_mm2 > 330. && r.shear_area_2_mm2 < 400.);
    assert!(r.shear_area_1_mm2 > 100. && r.shear_area_1_mm2 < 1400.);
}

#[test]
fn thin_walled_channel_matches_textbook() {
    // Centerline b = 50 flanges, h = 100 web, uniform t = 5, opening +x.
    let (b, h, t) = (50., 100., 5.);
    let r = thin_walled_open(&[
        strip([b, h / 2.], [0., h / 2.], t),
        strip([0., h / 2.], [0., -h / 2.], t),
        strip([0., -h / 2.], [b, -h / 2.], t),
    ])
    .unwrap();
    close(r.area_mm2, t * (2. * b + h));
    close(r.centroid[0], 2. * b * t * (b / 2.) / r.area_mm2);
    close(r.centroid[1], 0.);
    // Shear center outside the web, opposite the opening: e = 3b²/(h+6b).
    close(r.shear_center[0], -3. * b * b / (h + 6. * b));
    assert!(r.shear_center[1].abs() < 1e-6);
    // Cw = t·b³h²/12·(2h+3b)/(h+6b).
    close(
        r.cw_mm6,
        t * b.powi(3) * h * h / 12. * (2. * h + 3. * b) / (h + 6. * b),
    );
    close(r.j_mm4, (2. * b + h) * t.powi(3) / 3.);
}

#[test]
fn thin_walled_angle_shear_center_at_corner() {
    let r = thin_walled_open(&[
        strip([0., 50.], [0., 0.], 5.),
        strip([0., 0.], [50., 0.], 5.),
    ])
    .unwrap();
    // Both legs run through the corner: the shear center sits there and
    // the sectorial coordinate vanishes (no warping stiffness).
    assert!((r.shear_center[0]).abs() < 1e-6, "{}", r.shear_center[0]);
    assert!((r.shear_center[1]).abs() < 1e-6);
    assert!(r.cw_mm6.abs() < 1e-3, "{}", r.cw_mm6);
    close(r.centroid[0], 12.5);
    close(r.centroid[1], 12.5);
    close(r.angle_rad, std::f64::consts::FRAC_PI_4);
}

#[test]
fn thin_walled_validates_graph_topology() {
    assert!(thin_walled_open(&[]).is_err());
    assert!(thin_walled_open(&[strip([0., 0.], [0., 0.], 1.)]).is_err());
    assert!(thin_walled_open(&[strip([0., 0.], [1., 0.], -1.)]).is_err());
    // A single straight strip has zero lateral inertia.
    assert!(thin_walled_open(&[strip([0., 0.], [100., 0.], 5.)]).is_err());
    // Closed loop: square of strips.
    assert!(thin_walled_open(&[
        strip([0., 0.], [100., 0.], 5.),
        strip([100., 0.], [100., 100.], 5.),
        strip([100., 100.], [0., 100.], 5.),
        strip([0., 100.], [0., 0.], 5.),
    ])
    .is_err());
    // Two disjoint strips.
    assert!(thin_walled_open(&[
        strip([0., 0.], [0., 100.], 5.),
        strip([10., 0.], [60., 0.], 5.),
    ])
    .is_err());
}
