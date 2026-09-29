use nurbs_core::{radial_bounds::radius_bounds, surface::Surface};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for p in 1..=4 {
        for q in 1..=4 {
            for shift in [0., 1000000.] {
                let s = Surface {
                    degree_u: p,
                    degree_v: q,
                    knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
                    knots_v: [vec![0.; q + 1], vec![1.; q + 1]].concat(),
                    control_points: (0..=p)
                        .map(|i| {
                            (0..=q)
                                .map(|j| {
                                    vec![
                                        shift + i as f64 * 0.75,
                                        j as f64 - 2.,
                                        ((i * 7 + j * 3) % 5) as f64 - 1.,
                                    ]
                                })
                                .collect()
                        })
                        .collect(),
                    weights: (0..=p)
                        .map(|i| {
                            (0..=q)
                                .map(|j| 0.25 + ((i * 3 + j * 5) % 9) as f64)
                                .collect()
                        })
                        .collect(),
                    periodic_u: false,
                    periodic_v: false,
                };
                let origin = [shift + 0.5, -0.75, 2.];
                cases.push(json!({"surface":s,"origin":origin,"bounds":radius_bounds(&s,origin).unwrap().unwrap()}));
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
