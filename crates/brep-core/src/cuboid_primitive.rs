//! Cuboid primitive: preserves the original arithmetic and proof budgets.
use super::*;

/// Construct an exact six-face NURBS box with shared edges, not six loose patches.
pub fn cuboid(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    require(
        (0..3).all(|i| {
            min[i].is_finite()
                && max[i].is_finite()
                && max[i] > min[i]
                && min[i].abs() <= 1e6
                && max[i].abs() <= 1e6
        }),
        "Invalid box bounds",
    )?;
    let vertices = [
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1, 0, 1],
        [1, 1, 1],
        [0, 1, 1],
    ]
    .map(|p| Vertex {
        point: std::array::from_fn(|i| if p[i] == 0 { min[i] } else { max[i] }),
    })
    .to_vec();
    let mut m = Model(
        brep_topology::Model {
            vertices,
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    let mut edges = BTreeMap::new();
    for corners in [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ] {
        let mut coedges = vec![];
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let key = (a.min(b), a.max(b));
            let edge = *edges.entry(key).or_insert_with(|| {
                let id = m.0.edges.len();
                m.0.edges.push(Edge {
                    degenerate: false,
                    vertices: [key.0, key.1],
                    curve: line(
                        m.0.vertices[key.0].point.to_vec(),
                        m.0.vertices[key.1].point.to_vec(),
                    ),
                });
                id
            });
            coedges.push(Coedge {
                edge,
                reversed: a > b,
                pcurve: line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec()),
            });
        }
        let outer = m.0.loops.len();
        m.0.loops.push(Loop { coedges });
        m.0.faces.push(Face {
            surface: Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![
                    vec![
                        m.0.vertices[corners[0]].point.to_vec(),
                        m.0.vertices[corners[3]].point.to_vec(),
                    ],
                    vec![
                        m.0.vertices[corners[1]].point.to_vec(),
                        m.0.vertices[corners[2]].point.to_vec(),
                    ],
                ],
                weights: vec![vec![1., 1.], vec![1., 1.]],
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            holes: vec![],
        });
    }
    m.0.shells.push(Shell {
        faces: (0..6)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
        closed: true,
    });
    m.0.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    m.rebuild_topology_ids();
    m.validate()?;
    Ok(m)
}
