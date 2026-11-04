//! Exact dyadic polynomial warp of original box vertices, edges and faces.
//! Test geometry is authored independently of the SS and material algorithms.
use brep_core::Model;
pub fn bowl() -> Model {
    let mut model = brep_core::cuboid([0., 0., -1.], [1., 1., 0.]).unwrap();
    let shift = |x: f64, y: f64| x * x + y * y - x - y + 0.5 - 0.3125_f64.powi(2);
    for v in &mut model.vertices {
        v.point[2] += shift(v.point[0], v.point[1]);
    }
    for edge in &mut model.edges {
        let p = edge.curve.control_points[0].clone();
        let q = edge.curve.control_points[1].clone();
        let mut start = p.clone();
        start[2] += shift(p[0], p[1]);
        let mut end = q.clone();
        end[2] += shift(q[0], q[1]);
        let mut mid = (0..3)
            .map(|k| start[k] * 0.5 + end[k] * 0.5)
            .collect::<Vec<_>>();
        mid[2] -= ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)) * 0.5;
        edge.curve = nurbs_core::curve::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![start, mid, end],
            weights: vec![1.; 3],
            periodic: false,
        };
    }
    for face in &mut model.faces {
        let old = &face.surface;
        let o = &old.control_points[0][0];
        let u: [f64; 3] = std::array::from_fn(|k| old.control_points[1][0][k] - o[k]);
        let v: [f64; 3] = std::array::from_fn(|k| old.control_points[0][1][k] - o[k]);
        let points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let s = i as f64 * 0.5;
                        let t = j as f64 * 0.5;
                        let square = |k: usize| {
                            o[k] * o[k]
                                + 2. * o[k] * (u[k] * s + v[k] * t)
                                + u[k] * u[k] * if i == 2 { 1. } else { 0. }
                                + v[k] * v[k] * if j == 2 { 1. } else { 0. }
                                + 2. * u[k] * v[k] * s * t
                        };
                        let mut p = (0..3)
                            .map(|k| o[k] + u[k] * s + v[k] * t)
                            .collect::<Vec<_>>();
                        p[2] += square(0) + square(1) - p[0] - p[1] + 0.5 - 0.3125_f64.powi(2);
                        p
                    })
                    .collect()
            })
            .collect();
        face.surface = nurbs_core::surface::Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: points,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
