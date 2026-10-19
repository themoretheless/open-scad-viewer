//! Two distinct rational fillings of the same square, on independent domains.
use super::nurbs;
use nurbs::{curve::Curve, surface::Surface};
pub fn caps() -> [Surface; 2] {
    std::array::from_fn(|i| {
        let (u, v) = if i == 0 {
            ([-3., 5.], [7., 11.])
        } else {
            ([17., 29.], [-7., 13.])
        };
        let r = 0.25;
        let mut s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: [vec![u[0]; 3], vec![u[1]; 3]].concat(),
            knots_v: [vec![v[0]; 3], vec![v[1]; 3]].concat(),
            control_points: (0..3)
                .map(|a| {
                    (0..3)
                        .map(|b| vec![(a as f64 - 1.) * r, (b as f64 - 1.) * r, 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        s.weights[1][1] = if i == 0 { 1.25 } else { 1.5 };
        if i == 1 {
            s.control_points[1][1][0] = r / 8.;
        }
        s
    })
}
pub fn profiles() -> Vec<Curve> {
    let r = 0.25;
    let p = [
        [-r, -r, 0.],
        [r, -r, 0.],
        [r, r, 0.],
        [-r, r, 0.],
        [-r, -r, 0.],
    ];
    (0..4)
        .map(|i| nurbs::primitives::line(p[i], p[i + 1]).unwrap())
        .collect()
}
pub fn trims(s: &Surface, gauge: f64) -> Vec<Vec<Curve>> {
    let (u, v) = (
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    );
    let p = [
        [u[0], v[0]],
        [u[1], v[0]],
        [u[1], v[1]],
        [u[0], v[1]],
        [u[0], v[0]],
    ];
    vec![
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![p[i].to_vec(), p[i + 1].to_vec()],
                weights: vec![gauge; 2],
                periodic: false,
            })
            .collect(),
    ]
}
pub fn laws() -> (Curve, Curve, Curve, Curve, Curve, Curve, Curve) {
    let constant = nurbs::progressive_sweep::constant_vector_law;
    let path = nurbs::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let scale = nurbs::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let axes = constant([1.5, 2., 1.]).unwrap();
    let mut center = nurbs::primitives::line([0.25, 0.125, 0.], [0.5, -0.25, 0.25]).unwrap();
    center.knots = vec![17., 17., 29., 29.];
    (
        path,
        scale,
        axes,
        center,
        constant([0., 0., 1.]).unwrap(),
        constant([1., 0., 0.]).unwrap(),
        constant([0.; 3]).unwrap(),
    )
}
