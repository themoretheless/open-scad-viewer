use nurbs_core::{
    surface::Surface,
    surface_distance::{distance, rectangle_bounds},
};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for scale in [1e-6, 1., 1e6] {
        for offset in [0., 1e6] {
            // Non-clamped, multiple unequal spans, positive rational weights and unequal degrees.
            let a = Surface {
                degree_u: 3,
                degree_v: 2,
                knots_u: vec![-3., -2., -1., 0., 0.3, 1., 2., 3., 4.],
                knots_v: vec![-2., -1., 0., 0.6, 1., 2., 3.],
                control_points: (0..5)
                    .map(|i| {
                        (0..4)
                            .map(|j| {
                                vec![
                                    offset + scale * (i as f64 - 2.),
                                    offset + scale * j as f64,
                                    offset + scale * ((i * j + 2 * i + j) % 7) as f64,
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights: (0..5)
                    .map(|i| {
                        (0..4)
                            .map(|j| 0.5 + ((i + 2 * j) % 5) as f64 * 0.3)
                            .collect()
                    })
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            };
            for domain in [
                [[0., 1.], [0., 1.]],
                [[0.12, 0.23], [0.31, 0.57]],
                [[0.21, 0.86], [0.41, 0.92]],
                [[0.3, 0.3], [0.6, 0.6]],
                [[0.731, 0.731], [0.257, 0.257]],
            ] {
                cases.push(json!({"surface":a,"domain":domain,"bounds":rectangle_bounds(&a,domain).unwrap()}));
            }
        }
    }
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![10., 0., 0.], vec![10., 0., 5.]],
            vec![vec![10., 10., 0.], vec![10., 10., 5.]],
            vec![vec![0., 10., 0.], vec![0., 10., 5.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            *p = vec![15., 15., 2.37];
        }
    }
    let result = distance(&a, &b, 0.001, 20000).unwrap();
    println!(
        "{}",
        json!({"rectangles":cases,"distance":{"a":a,"b":b,"result":result.to_value()}})
    );
}
