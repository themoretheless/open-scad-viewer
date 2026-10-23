use nurbs_core::{curve::Curve, curve_surface_agreement::verify, surface::Surface};
use value_codec::json;
fn main() {
    let s = Surface {
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
    let mut split = s.clone();
    split.knots_u = vec![0., 0., 0.3, 1., 1.];
    split.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 1., 0.]],
        vec![vec![0.3, 0., 0.], vec![0.3, 1., 0.]],
        vec![vec![1., 0., 0.], vec![1., 1., 0.]],
    ];
    split.weights = vec![vec![1.; 2]; 3];
    let mut cases = Vec::new();
    for s in [s, split] {
        for weights in [vec![1., 1., 1.], vec![1., 2., 0.5], vec![0.5, 4., 1.]] {
            for knots in [
                vec![0., 0., 0.5, 1., 1.],
                vec![0.1, 0.1, 0.37, 0.9, 0.9],
                vec![0.1, 0.1, 0.1, 0.9, 0.9, 0.9],
            ] {
                for z in [0., 0.125, -0.25, 0.5, -1.] {
                    let p = Curve {
                        degree: if knots.len() == 6 { 2 } else { 1 },
                        knots: knots.clone(),
                        weights: weights.clone(),
                        control_points: vec![vec![0.125, 0.25], vec![0.5, 0.75], vec![0.875, 0.25]],
                        periodic: false,
                    };
                    let mut c = p.clone();
                    for point in &mut c.control_points {
                        point.push(z);
                    }
                    let tolerance = 1e-9;
                    let r = verify(&c, &p, &s, false, tolerance, 4096).unwrap();
                    cases.push(json!({"curve":c,"pcurve":p,"surface":s,"tolerance":tolerance,
                    "status":format!("{:?}",r.status),"cells":r.cells,"witness":r.witness,"distance":r.witness_distance}));
                }
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
