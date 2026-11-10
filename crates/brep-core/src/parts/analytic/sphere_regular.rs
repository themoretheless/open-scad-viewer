//! Regular stereographic sphere charts for radii with exact binary thirds.
use super::*;
pub(super) fn build(radius: f64) -> Result<Model> {
    let mut build = Builder::new();
    let equator = build.ring(radius, 0.);
    for sign in [1., -1.] {
        let pole = build.model.vertices.len();
        build.model.vertices.push(Vertex {
            point: [0., 0., sign * radius],
        });
        for quadrant in 0..4 {
            let next = (quadrant + 1) % 4;
            let a = QUADRANTS[quadrant];
            let b = QUADRANTS[next];
            // Bernstein coefficients of t and t^2 in degree 2. Homogeneous
            // numerator is (2u, 2v, +/- (1-u^2-v^2)); denominator 1+u^2+v^2.
            let linear = [0., 0.5, 1.];
            let square = [0., 0., 1.];
            let weights: Vec<Vec<f64>> = (0..3)
                .map(|i| (0..3).map(|j| 1. + square[i] + square[j]).collect())
                .collect();
            let surface = Surface {
                degree_u: 2,
                degree_v: 2,
                knots_u: vec![0., 0., 0., 1., 1., 1.],
                knots_v: vec![0., 0., 0., 1., 1., 1.],
                control_points: (0..3)
                    .map(|i| {
                        (0..3)
                            .map(|j| {
                                let w = weights[i][j];
                                vec![
                                    2. * radius * (linear[i] * a[0] + linear[j] * b[0]) / w,
                                    2. * radius * (linear[i] * a[1] + linear[j] * b[1]) / w,
                                    sign * radius * (1. - square[i] - square[j]) / w,
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights,
                periodic_u: false,
                periodic_v: false,
            };
            let meridian = |direction: [f64; 2]| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![0., 0., sign * radius],
                    vec![radius * direction[0], radius * direction[1], sign * radius],
                    vec![radius * direction[0], radius * direction[1], 0.],
                ],
                weights: vec![1., 1., 2.],
                periodic: false,
            };
            let mut circular_trim = rational_quarter_arc(1., 0., 0);
            circular_trim.control_points.iter_mut().for_each(|p| {
                p.pop();
            });
            let coedges = vec![
                build.coedge(
                    pole,
                    equator[quadrant],
                    meridian(a),
                    line(vec![0., 0.], vec![1., 0.]),
                ),
                build.coedge(
                    equator[quadrant],
                    equator[next],
                    rational_quarter_arc(radius, 0., quadrant),
                    circular_trim,
                ),
                build.coedge(
                    equator[next],
                    pole,
                    reversed(meridian(b)),
                    line(vec![0., 1.], vec![0., 0.]),
                ),
            ];
            let outer = build.wire(coedges);
            build.face(surface, outer, vec![], sign < 0.);
        }
    }
    build.finish()
}
