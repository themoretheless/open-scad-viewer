//! Original rational multi-span control model for packaged-kernel qualification.
#[cfg(feature = "codec")]
fn main() {
    use brep_topology::CoedgeTrim;
    fn weights(a: &[f64], b: &[f64]) -> [f64; 2] {
        if a.partial_cmp(b) == Some(std::cmp::Ordering::Less) {
            [1., 3.]
        } else {
            [3., 1.]
        }
    }
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    for edge in &mut m.edges {
        edge.curve.weights =
            weights(&edge.curve.control_points[0], &edge.curve.control_points[1]).to_vec();
    }
    for face in &mut m.faces {
        let s = &mut face.surface;
        let u = weights(&s.control_points[0][0], &s.control_points[1][0]);
        let v = weights(&s.control_points[0][0], &s.control_points[0][1]);
        let old = s.control_points.clone();
        let middle = (0..2)
            .map(|j| {
                (0..3)
                    .map(|k| (old[0][j][k] * u[0] + old[1][j][k] * u[1]) / (u[0] + u[1]))
                    .collect::<Vec<_>>()
            })
            .collect();
        s.control_points = vec![old[0].clone(), middle, old[1].clone()];
        s.weights = vec![
            vec![u[0] * v[0], u[0] * v[1]],
            vec![2. * v[0], 2. * v[1]],
            vec![u[1] * v[0], u[1] * v[1]],
        ];
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
    }
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            *c = c
                .clone()
                .with_trim(CoedgeTrim {
                    curve_parameter: [0., 1.],
                    pcurve_parameter: [0., 1.],
                    periodic_lift: [[0; 2]; 2],
                })
                .unwrap();
        }
    }
    m.validate().unwrap();
    let exact = brep_core::boundary_agreement::verify_exact(&m, 1_000_000).unwrap();
    assert!(exact.all_equal && exact.all_joins_exact);
    println!("{}", value_codec::to_string(&m).unwrap());
}

#[cfg(not(feature = "codec"))]
fn main() {panic!("Trimmed rational control model requires codec support")}
