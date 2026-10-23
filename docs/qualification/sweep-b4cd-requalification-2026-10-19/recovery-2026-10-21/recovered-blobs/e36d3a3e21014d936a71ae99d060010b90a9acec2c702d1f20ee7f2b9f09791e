use nurbs_core::{curve::Curve, curve_surface_agreement::verify, surface::Surface};
use value_codec::json;
fn main() {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3., 4.],
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; 4],
        control_points: [[0., 0.], [1., 0.], [0., 1.], [0., 0.]]
            .into_iter()
            .map(|xy| vec![vec![xy[0], xy[1], 0.], vec![xy[0], xy[1], 1.]])
            .collect(),
        periodic_u: true,
        periodic_v: false,
    };
    let mut cases = Vec::new();
    for shift in [-9., -6., -3., 0., 3., 6., 9.] {
        for reversed in [false, true] {
            for z in [-0.125, 0., 0.125] {
                let mut c = Curve {
                    degree: 1,
                    knots: vec![0., 0., 0.5, 1., 1.],
                    weights: vec![1.; 3],
                    control_points: vec![
                        vec![0., 0.5, 0.5 + z],
                        vec![0., 0., 0.5 + z],
                        vec![0.5, 0., 0.5 + z],
                    ],
                    periodic: false,
                };
                if reversed {
                    c.control_points.reverse();
                }
                let p = Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    weights: vec![1.; 2],
                    periodic: false,
                    control_points: vec![vec![2.5 + shift, 0.5], vec![3.5 + shift, 0.5]],
                };
                let r = verify(&c, &p, &surface, reversed, 1e-9, 4096).unwrap();
                cases.push(json!({"curve":c,"pcurve":p,"surface":surface,"reversed":reversed,"tolerance":1e-9,
            "status":format!("{:?}",r.status),"cells":r.cells,"witness":r.witness,"distance":r.witness_distance}));
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
