//! step v3 export algorithm: periodicized_sphere.
use super::*;
pub fn periodicized_sphere(model: &Model) -> Result<Model> {
    let radius = model
        .vertices
        .iter()
        .map(|vertex| {
            vertex
                .point
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let circle = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let circle_weights = [1., w, 1., w, 1., w, 1., w, 1.];
    let profile = [
        [0., radius],
        [radius, radius],
        [radius, 0.],
        [radius, -radius],
        [0., -radius],
    ];
    let profile_weights = [1., w, 1., w, 1.];
    let control_points = circle
        .iter()
        .map(|u| {
            profile
                .iter()
                .map(|v| vec![v[0] * u[0], v[0] * u[1], v[1]])
                .collect()
        })
        .collect();
    let weights = circle_weights
        .iter()
        .map(|wu| profile_weights.iter().map(|wv| wu * wv).collect())
        .collect();
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: quadratic_patch_knots(4),
        knots_v: quadratic_patch_knots(2),
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let mut result = model.clone();
    let source_edges = result.edges.clone();
    let north = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] - radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere north pole is missing"))?;
    let south = model
        .vertices
        .iter()
        .position(|vertex| (vertex.point[2] + radius).abs() < radius * 1e-9)
        .ok_or_else(|| refuse("Sphere south pole is missing"))?;
    let selected = [0, 1, 3, 5, 7, 8];
    result.0.edges = selected
        .iter()
        .map(|index| {
            let mut edge = source_edges[*index].clone();
            if edge.curve.degree == 2 {
                edge.curve.weights = vec![1., w, 1.]
            }
            edge
        })
        .collect();
    let pole_edges = [north, south].map(|vertex| {
        let edge = result.edges.len();
        let point = result.vertices[vertex].point.to_vec();
        result.0.edges.push(Edge {
            degenerate: true,
            vertices: [vertex, vertex],
            curve: Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![point.clone(), point],
                weights: vec![1., 1.],
                periodic: false,
            },
        });
        edge
    });
    let line = |a: [f64; 2], b: [f64; 2]| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    result.0.loops = vec![
        Loop {
            coedges: vec![
                Coedge::new(
                    pole_edges[0],
                    false,
                    line([0., 0.], [4., 0.]),
                ),
                Coedge::new(
                    0,
                    true,
                    line([4., 0.], [4., 1.]),
                ),
                Coedge::new(
                    4,
                    false,
                    line([4., 1.], [3., 1.]),
                ),
                Coedge::new(
                    3,
                    true,
                    line([3., 1.], [2., 1.]),
                ),
                Coedge::new(
                    2,
                    true,
                    line([2., 1.], [1., 1.]),
                ),
                Coedge::new(
                    1,
                    true,
                    line([1., 1.], [0., 1.]),
                ),
                Coedge::new(
                    0,
                    false,
                    line([0., 1.], [0., 0.]),
                ),
            ],
        },
        Loop {
            coedges: vec![
                Coedge::new(
                    1,
                    false,
                    line([0., 1.], [1., 1.]),
                ),
                Coedge::new(
                    2,
                    false,
                    line([1., 1.], [2., 1.]),
                ),
                Coedge::new(
                    3,
                    false,
                    line([2., 1.], [3., 1.]),
                ),
                Coedge::new(
                    4,
                    true,
                    line([3., 1.], [4., 1.]),
                ),
                Coedge::new(
                    5,
                    false,
                    line([4., 1.], [4., 2.]),
                ),
                Coedge::new(
                    pole_edges[1],
                    false,
                    line([4., 2.], [0., 2.]),
                ),
                Coedge::new(
                    5,
                    true,
                    line([0., 2.], [0., 1.]),
                ),
            ],
        },
    ];
    result.0.faces = vec![
        Face {
            surface: surface.clone(),
            outer: 0,
            holes: vec![],
        },
        Face {
            surface,
            outer: 1,
            holes: vec![],
        },
    ];
    result.0.shells[model.bodies[0].outer_shell].faces = vec![
        FaceUse {
            face: 0,
            reversed: false,
        },
        FaceUse {
            face: 1,
            reversed: false,
        },
    ];
    result.rebuild_topology_ids();
    for (face_index, face) in result.faces.iter().enumerate() {
        for coedge in &result.loops[face.outer].coedges {
            for sample in 0..=8 {
                let t = sample as f64 / 8.;
                let uv = coedge.pcurve.evaluate(t)?.point;
                let p = face.surface.evaluate(uv[0], uv[1])?.point;
                let edge = &result.edges[coedge.edge];
                let domain = edge.curve.domain();
                let q = edge
                    .curve
                    .evaluate(if coedge.reversed {
                        domain[1] - t * (domain[1] - domain[0])
                    } else {
                        domain[0] + t * (domain[1] - domain[0])
                    })?
                    .point;
                let error = p
                    .iter()
                    .zip(&q)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt();
                if error > 1e-7 {
                    return Err(refuse(format!(
                        "Periodic sphere face {face_index} edge {} mismatch {error} at {t}: uv={uv:?} surface={p:?} edge={q:?}",
                        coedge.edge
                    )));
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
