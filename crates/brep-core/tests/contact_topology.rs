use brep_core::{fillet_contact::topology, region_replacement::EdgeRadius, *};
use nurbs_core::surface::Surface;
use std::collections::BTreeMap;
fn bipyramid(n: usize) -> Model {
    let mut m = Model::empty(1e-7).unwrap();
    m.vertices.push(Vertex {
        point: [0., 0., 10.],
    });
    m.vertices.push(Vertex {
        point: [0., 0., -10.],
    });
    for i in 0..n {
        let t = std::f64::consts::TAU * i as f64 / n as f64;
        m.vertices.push(Vertex {
            point: [10. * t.cos(), 10. * t.sin(), 0.],
        });
    }
    let mut edges = BTreeMap::new();
    for i in 0..n {
        let a = 2 + i;
        let b = 2 + (i + 1) % n;
        for ids in [[0, a, b], [1, b, a]] {
            let points = ids.map(|v| m.vertices[v].point);
            let mut cp = vec![vec![vec![0.; 3]; 2]; 2];
            for u in 0..2 {
                for v in 0..2 {
                    for k in 0..3 {
                        cp[u][v][k] = points[0][k]
                            + u as f64 * (points[1][k] - points[0][k])
                            + v as f64 * (points[2][k] - points[0][k]);
                    }
                }
            }
            let surface = Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: cp,
                weights: vec![vec![1.; 2]; 2],
                periodic_u: false,
                periodic_v: false,
            };
            let uv = [[0., 0.], [1., 0.], [0., 1.]];
            let mut coedges = Vec::new();
            for j in 0..3 {
                let x = ids[j];
                let y = ids[(j + 1) % 3];
                let key = (x.min(y), x.max(y));
                let edge = *edges.entry(key).or_insert_with(|| {
                    let e = m.edges.len();
                    let endpoints =
                        vec![m.vertices[x].point.to_vec(), m.vertices[y].point.to_vec()];
                    m.edges.push(Edge {
                        vertices: [x, y],
                        curve: nurbs_core::curve::Curve::from_polyline(endpoints).unwrap(),
                        degenerate: false,
                    });
                    e
                });
                coedges.push(Coedge::new(
                    edge,
                    m.edges[edge].vertices[0] != x,
                    nurbs_core::curve::Curve::from_polyline(vec![
                        uv[j].to_vec(),
                        uv[(j + 1) % 3].to_vec(),
                    ])
                    .unwrap(),
                ));
            }
            let outer = m.loops.len();
            m.loops.push(Loop { coedges });
            m.faces.push(Face {
                surface,
                outer,
                holes: vec![],
            });
        }
    }
    let face_count = m.faces.len();
    m.shells.push(Shell {
        faces: (0..face_count)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
        closed: true,
    });
    m.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
#[test]
fn cyclic_fans_preserve_five_six_and_eight_independent_radii() {
    for n in [5, 6, 8] {
        let m = bipyramid(n);
        let before = m.clone();
        let requested: Vec<_> = m
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.vertices.contains(&0))
            .enumerate()
            .map(|(i, (edge, _))| EdgeRadius {
                edge,
                radius_mm: 2. + i as f64 / 10.,
            })
            .collect();
        let d = topology::discover(&m, 0, &requested, 10000).unwrap();
        let fan = d.fans.iter().find(|f| f.vertex == 0).unwrap();
        assert_eq!(fan.edges.len(), n);
        assert_eq!(fan.faces.len(), n);
        for i in 0..n {
            let face = &m.faces[fan.faces[i]];
            let wire = &m.loops[face.outer];
            assert!(wire.coedges.iter().any(|c| c.edge == fan.edges[i]));
            assert!(
                wire.coedges
                    .iter()
                    .any(|c| c.edge == fan.edges[(i + 1) % n])
            );
            assert_eq!(
                fan.radii_mm[i],
                Some(
                    requested
                        .iter()
                        .find(|r| r.edge == fan.edges[i])
                        .unwrap()
                        .radius_mm
                )
            );
        }
        assert_eq!(m, before);
        assert!(topology::discover(&m, 0, &requested, d.topology_visits - 1).is_err());
        assert!(topology::discover(&m, 0, &requested, d.topology_visits).is_ok());
    }
}
#[test]
fn partial_selection_retains_unselected_fan_roles_and_rejects_duplicate_radii() {
    let m = bipyramid(5);
    let edge = m
        .edges
        .iter()
        .position(|e| e.vertices.contains(&0))
        .unwrap();
    let r = EdgeRadius {
        edge,
        radius_mm: 2.,
    };
    let d = topology::discover(&m, 0, &[r], 10000).unwrap();
    let fan = d.fans.iter().find(|f| f.vertex == 0).unwrap();
    assert_eq!(fan.radii_mm.iter().filter(|r| r.is_some()).count(), 1);
    assert!(topology::discover(&m, 0, &[r, r], 10000).is_err());
    let mut open = m.clone();
    open.shells[0].closed = false;
    assert!(topology::discover(&open, 0, &[r], 10000).is_err());
}
#[test]
fn source_lifts_are_checked_against_actual_supports_not_only_incidence() {
    let m = region_replacement::four_edge_corner_stock().unwrap();
    let request = EdgeRadius {
        edge: 0,
        radius_mm: 2.8,
    };
    let proof = topology::inspect_source_edge(&m, 0, request, 10000, 1024, 100000).unwrap();
    assert!(proof.proven);
    let plan = region_replacement::plan_radius_network(&m, 0, &[request], 10000).unwrap();
    let mut damaged = m.clone();
    for row in &mut damaged.faces[plan.edges[0].supports[0]]
        .surface
        .control_points
    {
        for p in row {
            p[2] += 1.;
        }
    }
    assert!(topology::inspect_source_edge(&damaged, 0, request, 10000, 1024, 100000).is_err());
    assert!(
        topology::propose_edge(
            &damaged,
            0,
            request,
            &Default::default(),
            10000,
            1024,
            100000
        )
        .is_err()
    );
}
#[test]
fn fan_order_is_independent_of_coordinates_and_shell_storage_order() {
    let m = bipyramid(6);
    let request = m
        .edges
        .iter()
        .enumerate()
        .find(|(_, e)| e.vertices.contains(&0))
        .map(|(edge, _)| EdgeRadius {
            edge,
            radius_mm: 2.7,
        })
        .unwrap();
    let a = topology::discover(&m, 0, &[request], 10000).unwrap();
    let mut moved = m.clone();
    moved.shells[0].faces.reverse();
    // Transform all geometric definitions consistently; fan discovery must
    // depend on incidence, not world-coordinate angular order.
    let map = |p: &mut [f64]| {
        let x = p[0];
        p[0] = p[2] + 32.;
        p[2] = -x + 16.;
    };
    for v in &mut moved.vertices {
        map(&mut v.point);
    }
    for e in &mut moved.edges {
        for p in &mut e.curve.control_points {
            map(p);
        }
    }
    for f in &mut moved.faces {
        for row in &mut f.surface.control_points {
            for p in row {
                map(p);
            }
        }
    }
    let b = topology::discover(&moved, 0, &[request], 10000).unwrap();
    assert_eq!(a.fans, b.fans);
}
