use nurbs_core::{ray_surface::intersections, surface::Surface};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for q in 1..=3 {
        for weights in [[1., 1., 1.], [1., 2., 4.], [1., 0.25, 2.]] {
            for shifted in [false, true] {
                for shear in [0., 0.25, -0.5] {
                    let ku = if shifted {
                        vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17]
                    } else {
                        vec![0., 0., 0., 1., 1., 1.]
                    };
                    let kv = if shifted {
                        [vec![-0.71; q + 1], vec![0.29; q + 1]].concat()
                    } else {
                        [vec![0.; q + 1], vec![1.; q + 1]].concat()
                    };
                    let s = Surface {
                        degree_u: 2,
                        degree_v: q,
                        knots_u: ku,
                        knots_v: kv,
                        control_points: (0..3)
                            .map(|i| {
                                (0..=q)
                                    .map(|j| {
                                        vec![
                                            [0.16, -0.34, 0.16][i] / weights[i]
                                                + shear * i as f64 * 0.5,
                                            j as f64 / q as f64,
                                            i as f64 * 0.5,
                                        ]
                                    })
                                    .collect()
                            })
                            .collect(),
                        weights: (0..3).map(|i| vec![weights[i]; q + 1]).collect(),
                        periodic_u: false,
                        periodic_v: false,
                    };
                    let origin = [-shear, 0.37, -1.];
                    let direction = [shear, 0., 1.];
                    let r = intersections(&s, origin, direction, 1e-7, 10000).unwrap();
                    assert!(r.complete, "q={q}, shear={shear}, shifted={shifted}");
                    let roots = r
                        .roots
                        .iter()
                        .map(|r| json!({"uv":r.uv,"parameter":r.parameter}))
                        .collect::<Vec<_>>();
                    cases.push(json!({"surface":s,"origin":origin,"direction":direction,"roots":roots,"cells":r.cells,"complete":r.complete}));
                }
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
