use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::{Axis, Surface},
};
#[test]
fn original_degree_six_surface_and_degree_six_pcurve_fit_existing_polynomial_bound() {
    let surface = Surface {
        degree_u: 6,
        degree_v: 6,
        knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
        knots_v: [vec![0.; 7], vec![1.; 7]].concat(),
        control_points: (0..7)
            .map(|i| {
                (0..7)
                    .map(|j| {
                        vec![
                            i as f64 / 6.,
                            j as f64 / 6.,
                            if i == 3 && j == 3 { 0.125 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 7]; 7],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
    let c = surface.iso(Axis::U, 0.5).unwrap();
    let r = curve_surface_agreement::verify(&c, &pc, &surface, false, 1e-7, 8).unwrap();
    assert_eq!(r.status, Status::WithinTolerance, "{r:?}");
    assert!(r.cells <= 8);
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let pc = Curve {
        degree: 6,
        knots: [vec![0.; 7], vec![1.; 7]].concat(),
        control_points: (0..7).map(|i| vec![i as f64 / 6., 0.5]).collect(),
        weights: vec![1., 0.5, 1., 0.5, 1., 0.5, 1.],
        periodic: false,
    };
    let c = Curve {
        control_points: pc
            .control_points
            .iter()
            .map(|p| vec![p[0], p[1], 0.])
            .collect(),
        ..pc.clone()
    };
    let r = curve_surface_agreement::verify(&c, &pc, &plane, false, 1e-7, 8).unwrap();
    assert_eq!(r.status, Status::WithinTolerance, "{r:?}");
    let mut broken = c;
    for p in &mut broken.control_points {
        p[2] = 0.001;
    }
    assert_eq!(
        curve_surface_agreement::verify(&broken, &pc, &plane, false, 1e-7, 8)
            .unwrap()
            .status,
        Status::Mismatch
    );
}
