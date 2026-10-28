use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
};
fn check_composition(degree: usize) {
    // The same independently defined planar parabola in world and UV space.
    let p: Vec<Vec<f64>> = (0_usize..=8)
        .map(|i| vec![i as f64 / 8., (i * i.saturating_sub(1)) as f64 / 56.])
        .collect();
    let pc = Curve {
        degree: 8,
        knots: [vec![0.; 9], vec![1.; 9]].concat(),
        weights: vec![1.; 9],
        periodic: false,
        control_points: p.clone(),
    };
    let c = Curve {
        control_points: p
            .into_iter()
            .map(|mut p| {
                p.push(0.);
                p
            })
            .collect(),
        ..pc.clone()
    };
    let mut s = Surface {
        degree_u: degree,
        degree_v: degree,
        knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        knots_v: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![vec![1.; degree + 1]; degree + 1],
        periodic_u: false,
        periodic_v: false,
        control_points: (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| vec![i as f64 / degree as f64, j as f64 / degree as f64, 0.])
                    .collect()
            })
            .collect(),
    };
    let report = curve_surface_agreement::verify(&c, &pc, &s, false, 1e-7, 100).unwrap();
    assert_eq!(report.status, Status::WithinTolerance);
    assert_eq!(
        report.cells, 1,
        "A correlated whole-curve enclosure should qualify without Cartesian subdivision"
    );
    for row in &mut s.control_points {
        for p in row {
            p[2] = 0.01;
        }
    }
    let mismatch = curve_surface_agreement::verify(&c, &pc, &s, false, 1e-7, 100).unwrap();
    assert_eq!(mismatch.status, Status::Mismatch);
    assert!(mismatch.witness_distance.unwrap()[0] > 1e-7);
}

#[test]
fn degree_eighty_composition_keeps_the_common_parameter_correlated() {
    check_composition(5);
}
#[test]
fn degree_144_composition_keeps_large_binomials_outward() {
    check_composition(9);
}
