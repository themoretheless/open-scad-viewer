//! Nonplanar polynomial crease with zero 2-jets at two interior end stations.
//! Constructive qualification input, not a general constant-radius fillet.
use brep_core::{
    Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex, face_cut,
    fillet_contact::Contact, region_replacement as r,
};
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, ToPrimitive};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
use std::collections::BTreeMap;
fn choose(n: usize, k: usize) -> i64 {
    if k > n {
        0
    } else {
        (0..k.min(n - k)).fold(1, |a, i| a * (n - i) as i64 / (i + 1) as i64)
    }
}
fn mul(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut c = vec![0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            c[i + j] += x * y;
        }
    }
    c
}
fn height() -> Vec<f64> {
    let mut a = vec![1];
    let mut b = vec![1];
    for _ in 0..4 {
        a = mul(&a, &[-1, 4]);
        b = mul(&b, &[3, -4]);
    }
    let power = mul(&a, &b);
    (0..=8)
        .map(|i| {
            let q = (0..=i)
                .map(|k| {
                    Q::new(
                        (power[k] * choose(i, k) * 280).into(),
                        (choose(8, k) * (1 << 18)).into(),
                    )
                })
                .sum::<Q>();
            let value = q.to_f64().unwrap();
            assert_eq!(Q::from_f64(value).unwrap(), q);
            value
        })
        .collect()
}
fn surface(cp: Vec<Vec<Vec<f64>>>, du: usize, dv: usize, u: [f64; 2], v: [f64; 2]) -> Surface {
    Surface {
        degree_u: du,
        degree_v: dv,
        knots_u: [vec![u[0]; du + 1], vec![u[1]; du + 1]].concat(),
        knots_v: [vec![v[0]; dv + 1], vec![v[1]; dv + 1]].concat(),
        weights: vec![vec![1.; dv + 1]; du + 1],
        control_points: cp,
        periodic_u: false,
        periodic_v: false,
    }
}
struct Build {
    m: Model,
    edges: BTreeMap<(usize, usize), usize>,
    uses: Vec<FaceUse>,
}
impl Build {
    fn face(
        &mut self,
        s: Surface,
        nodes: &[usize],
        uv: &[[f64; 2]],
        curves: Vec<Curve>,
        reversed: bool,
    ) {
        let mut coedges = Vec::new();
        for i in 0..nodes.len() {
            let a = nodes[i];
            let b = nodes[(i + 1) % nodes.len()];
            let key = (a.min(b), a.max(b));
            let edge = if let Some(&e) = self.edges.get(&key) {
                e
            } else {
                let e = self.m.edges.len();
                self.m.edges.push(Edge {
                    vertices: [a, b],
                    curve: curves[i].clone(),
                    degenerate: false,
                });
                self.edges.insert(key, e);
                e
            };
            let rev = self.m.edges[edge].vertices != [a, b];
            coedges.push(Coedge::new(
                edge,
                rev,
                Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % uv.len()].to_vec()])
                    .unwrap(),
            ));
        }
        let outer = self.m.loops.len();
        self.m.loops.push(Loop { coedges });
        let face = self.m.faces.len();
        self.m.faces.push(Face {
            surface: s,
            outer,
            holes: vec![],
        });
        self.uses.push(FaceUse { face, reversed });
    }
    fn rect(&mut self, s: Surface, nodes: [usize; 4], reversed: bool) {
        let u = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let v = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        let uv = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
        let curves = vec![
            s.iso(Axis::V, v[0]).unwrap(),
            s.iso(Axis::U, u[1]).unwrap(),
            s.iso(Axis::V, v[1]).unwrap().reverse().unwrap(),
            s.iso(Axis::U, u[0]).unwrap().reverse().unwrap(),
        ];
        self.face(s, &nodes, &uv, curves, reversed);
    }
}
pub fn fixture() -> (Model, r::EdgeRadius, Surface, [Contact; 2]) {
    let h = height();
    let he = h[0];
    assert_eq!(he, h[8]);
    let q = (0..=8)
        .map(|i| {
            let i = i as f64;
            (2. * i * i - 9. * i + 7.) / 64.
        })
        .collect::<Vec<_>>();
    let points = [
        [-1., 0., q[0]],
        [3., 0., q[8]],
        [-1., -1., q[0] - he],
        [3., -1., q[8] - he],
        [-1., 1., q[0] - he],
        [3., 1., q[8] - he],
        [-1., -1., -4.],
        [3., -1., -4.],
        [-1., 1., -4.],
        [3., 1., -4.],
    ];
    let m = Model(
        brep_topology::Model {
            vertices: points.into_iter().map(|point| Vertex { point }).collect(),
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        Default::default(),
    );
    let mut b = Build {
        m,
        edges: BTreeMap::new(),
        uses: vec![],
    };
    for (side, sign) in [-1., 1.].into_iter().enumerate() {
        let roof = surface(
            (0..=8)
                .map(|i| {
                    let x = -1. + i as f64 * 0.5;
                    vec![vec![x, 0., q[i]], vec![x, sign, q[i] - h[i]]]
                })
                .collect(),
            8,
            1,
            [-1., 3.],
            [0., 1.],
        );
        b.rect(
            roof,
            if side == 0 {
                [0, 1, 3, 2]
            } else {
                [0, 1, 5, 4]
            },
            side == 0,
        );
    }
    for (side, sign) in [-1., 1.].into_iter().enumerate() {
        let wall = surface(
            (0..=8)
                .map(|i| {
                    let x = -1. + i as f64 * 0.5;
                    vec![vec![x, sign, -4.], vec![x, sign, q[i] - h[i]]]
                })
                .collect(),
            8,
            1,
            [-1., 3.],
            [0., 1.],
        );
        b.rect(
            wall,
            if side == 0 {
                [6, 7, 3, 2]
            } else {
                [8, 9, 5, 4]
            },
            side == 1,
        );
    }
    let floor = surface(
        vec![
            vec![vec![-1., -1., -4.], vec![-1., 1., -4.]],
            vec![vec![3., -1., -4.], vec![3., 1., -4.]],
        ],
        1,
        1,
        [-1., 3.],
        [-1., 1.],
    );
    b.rect(floor, [6, 7, 9, 8], true);
    for (side, x) in [-1., 3.].into_iter().enumerate() {
        let s = surface(
            vec![
                vec![vec![x, -1., -4.], vec![x, -1., 1.]],
                vec![vec![x, 1., -4.], vec![x, 1., 1.]],
            ],
            1,
            1,
            [-1., 1.],
            [-4., 1.],
        );
        let nodes = if side == 0 {
            [6, 8, 4, 0, 2]
        } else {
            [7, 9, 5, 1, 3]
        };
        let qe = if side == 0 { q[0] } else { q[8] };
        let uv = [
            [-1., -4.],
            [1., -4.],
            [1., qe - he],
            [0., qe],
            [-1., qe - he],
        ];
        let curves = (0..5)
            .map(|i| {
                Curve::from_polyline(vec![
                    points[nodes[i]].to_vec(),
                    points[nodes[(i + 1) % 5]].to_vec(),
                ])
                .unwrap()
            })
            .collect();
        b.face(s, &nodes, &uv, curves, side == 0);
    }
    b.m.shells.push(Shell {
        faces: b.uses,
        closed: true,
    });
    b.m.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    b.m.rebuild_topology_ids();
    b.m.validate().unwrap();
    let source = b.m;
    let request = r::EdgeRadius {
        edge: 0,
        radius_mm: 1.,
    };
    let w = 0.25;
    let hc = (0..=8)
        .map(|i| if i == 4 { 1. / 256. } else { 0. })
        .collect::<Vec<_>>();
    let qc = (0_usize..=8)
        .map(|i| (i * (i.saturating_sub(1))) as f64 / 128.)
        .collect::<Vec<_>>();
    let phi = [1., 0.5, 0., 0.5, 1.];
    let skin = surface(
        (0..=8)
            .map(|i| {
                (0..=4)
                    .map(|j| {
                        vec![
                            i as f64 / 4.,
                            -w + w * j as f64 / 2.,
                            qc[i] - hc[i] * w * phi[j],
                        ]
                    })
                    .collect()
            })
            .collect(),
        8,
        4,
        [0., 2.],
        [0., 1.],
    );
    let plan = r::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let contacts = std::array::from_fn(|side| {
        let face = plan.edges[0].supports[side];
        assert_eq!(face, side);
        let curve = Curve {
            degree: 8,
            knots: [vec![0.; 9], vec![2.; 9]].concat(),
            control_points: (0..=8)
                .map(|i| {
                    vec![
                        i as f64 / 4.,
                        if side == 0 { -w } else { w },
                        qc[i] - hc[i] * w,
                    ]
                })
                .collect(),
            weights: vec![1.; 9],
            periodic: false,
        };
        let pc = Curve::from_polyline(vec![vec![0., w], vec![2., w]]).unwrap();
        let fp = Curve::from_polyline(vec![vec![0., side as f64], vec![2., side as f64]]).unwrap();
        let positions = nurbs_core::surface_g2_pcurves::verify_positions(
            &curve,
            [&pc, &fp],
            [&source.faces[face].surface, &skin],
            [false; 2],
            source.tolerance_mm,
            100000,
            1000000,
        )
        .unwrap();
        assert!(positions.proven, "original contact {side} not lifted");
        Contact {
            curve,
            support_pcurve: pc,
            fillet_pcurve: fp,
            positions,
        }
    });
    (source, request, skin, contacts)
}
pub fn paths(
    source: &Model,
    request: r::EdgeRadius,
    contacts: &[Contact; 2],
    roots: &brep_core::edge_partition::contact_interval::ContactInterval,
) -> Vec<face_cut::ContactFaceCut> {
    let plan = r::plan_radius_network(source, 0, &[request], 10000).unwrap();
    plan.edges[0]
        .supports
        .iter()
        .enumerate()
        .map(|(side, &face)| {
            let mut points = roots
                .endpoints()
                .map(|v| roots.model().vertices[v].point.to_vec());
            if roots.branch().contacts[side].contact_reversed {
                points.reverse();
            }
            let c = &contacts[side].curve;
            face_cut::ContactFaceCut {
                face,
                curves: vec![
                    Curve::from_polyline(vec![points[0].clone(), c.control_points[0].clone()])
                        .unwrap(),
                    c.clone(),
                    Curve::from_polyline(vec![
                        c.control_points.last().unwrap().clone(),
                        points[1].clone(),
                    ])
                    .unwrap(),
                ],
                keep_uv: vec![[1., 0.75]],
            }
        })
        .collect()
}
pub fn patch(retention: &face_cut::ContactRetention, skin: &Surface) -> r::OwnedPatchSet {
    let plan = retention.plan();
    let model = &plan.model;
    let a = &plan.regions[0].cut_edges;
    let b = &plan.regions[1].cut_edges;
    let root = retention.roots().endpoints();
    let a0 = model.edges[a[0]].vertices[1];
    let a1 = model.edges[a[2]].vertices[0];
    let b0 = model.edges[b[0]].vertices[1];
    let b1 = model.edges[b[2]].vertices[0];
    let nodes = [a0, a1, root[1], b1, b0, root[0]];
    let uv = [[0., 0.], [2., 0.], [2., 0.5], [2., 1.], [0., 1.], [0., 0.5]];
    let edges = [a[1], a[2], b[2], b[1], b[0], a[0]];
    let coedges = edges
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let pair = [nodes[i], nodes[(i + 1) % 6]];
            let rev = model.edges[e].vertices != pair;
            assert!(
                model.edges[e].vertices == pair || model.edges[e].vertices == [pair[1], pair[0]]
            );
            Coedge::new(
                e,
                rev,
                Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 6].to_vec()]).unwrap(),
            )
        })
        .collect();
    r::OwnedPatchSet {
        vertices: vec![],
        edges: vec![],
        patches: vec![r::OwnedSewPatch {
            surface: skin.clone(),
            loops: vec![coedges],
            reversed: false,
        }],
    }
}
