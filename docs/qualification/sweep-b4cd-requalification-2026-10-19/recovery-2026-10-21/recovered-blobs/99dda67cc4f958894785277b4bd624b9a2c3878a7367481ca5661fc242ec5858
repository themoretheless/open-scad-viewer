use nurbs_core::{curve::Curve, trim_domain::TrimDomain};
use value_codec::json;
fn arc(points: [[f64; 2]; 3]) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: points.into_iter().map(|p| p.to_vec()).collect(),
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    }
}
fn circle(r: f64) -> Vec<Curve> {
    vec![
        arc([[r, 0.], [r, r], [0., r]]),
        arc([[0., r], [-r, r], [-r, 0.]]),
        arc([[-r, 0.], [-r, -r], [0., -r]]),
        arc([[0., -r], [r, -r], [r, 0.]]),
    ]
}
fn main() {
    let lower = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 5., 5., 5., 5.],
        control_points: vec![vec![-2., 0.], vec![-1., -3.], vec![1., -1.], vec![2., 0.]],
        weights: vec![1., 0.6, 1.4, 1.],
        periodic: false,
    };
    let upper = Curve {
        control_points: vec![vec![2., 0.], vec![1., 3.], vec![-1., 2.], vec![-2., 0.]],
        weights: vec![1., 1.3, 0.7, 1.],
        ..lower.clone()
    };
    let ring = vec![
        circle(10.),
        circle(3.)
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect(),
    ];
    let lens = vec![vec![lower, upper]];
    let mut cases = Vec::new();
    for (name, loops) in [("ring", ring), ("rational-cubic", lens)] {
        for (scale, offset) in [(1., 0.), (1e-6, 1e3), (1e6, -1e4)] {
            let mut loops = loops.clone();
            for c in loops.iter_mut().flatten() {
                for p in &mut c.control_points {
                    for x in p {
                        *x = *x * scale + offset;
                    }
                }
            }
            let region = TrimDomain::new(&loops, scale * 1e-7).unwrap();
            let mut queries = Vec::new();
            for i in -12..=12 {
                for j in -12..=12 {
                    let p = [
                        (i as f64 + 0.173) * scale + offset,
                        (j as f64 + 0.291) * scale + offset,
                    ];
                    let report = region.classify([[p[0]; 2], [p[1]; 2]], 10000).unwrap();
                    queries.push(json!({"point":p,"report":report.to_value()}));
                }
            }
            cases.push(
                json!({"name":name,"scale":scale,"offset":offset,"loops":loops,"queries":queries}),
            );
        }
    }
    println!("{}", json!(cases));
}
