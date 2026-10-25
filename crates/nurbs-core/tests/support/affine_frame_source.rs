//! Original rational positive affine laws on independent authored domains.
//! Endpoint Cartesian jets through order two vanish: first/last three
//! controls are constant even though the rational weights vary.
use super::nurbs;
use nurbs::curve::Curve;
fn law(points: Vec<Vec<f64>>, domain: [f64; 2]) -> Curve {
    Curve {
        degree: 6,
        knots: [vec![domain[0]; 7], vec![domain[1]; 7]].concat(),
        control_points: points,
        weights: vec![1., 1.5, 2., 1.25, 2., 1.5, 1.],
        periodic: false,
    }
}
pub fn laws() -> (Curve, Curve, Curve) {
    let mut uniform = vec![vec![1., 0., 0.]; 7];
    uniform[3][0] = 1.125;
    let mut axes = vec![vec![1.; 3]; 7];
    axes[3] = vec![2., 1.5, 3.];
    let mut center = vec![vec![0.; 3]; 7];
    center[3] = vec![1. / 8192., -1. / 16384., 1. / 32768.];
    (
        law(uniform, [7., 19.]),
        law(axes, [-7., 13.]),
        law(center, [17., 29.]),
    )
}
