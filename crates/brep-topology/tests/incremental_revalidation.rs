//! Oracle: incremental revalidation via `try_edit_scoped` must accept/reject
//! exactly like a full `validate_topology_with_vertices` on the candidate,
//! with the same error code and message.
use brep_topology::{Coedge, EditScope, Edge, Face, FaceUse, Loop, Model, Shell, TopologySnapshot, Vertex, Body};

type M = Model<(), (), ()>;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn bool(&mut self) -> bool {
        self.next() & 1 == 0
    }
    fn coord(&mut self) -> f64 {
        (self.next() % 20000) as f64 / 1000. - 10.
    }
}

/// Append a valid closed tetrahedron shell (plus its body) to the model.
fn append_tetra(m: &mut M, rng: &mut Rng) {
    let bv = m.vertices.len();
    let be = m.edges.len();
    let bl = m.loops.len();
    let bf = m.faces.len();
    for _ in 0..4 {
        m.vertices.push(Vertex {
            point: [rng.coord(), rng.coord(), rng.coord()],
        });
    }
    for [a, b] in [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]] {
        m.edges.push(Edge {
            degenerate: false,
            vertices: [bv + a, bv + b],
            curve: (),
        });
    }
    // Faces (0,2,1), (0,1,3), (0,3,2), (1,2,3): every edge appears twice with
    // opposite traversal directions, so the shell is closed and manifold.
    let loops: [[(usize, bool); 3]; 4] = [
        [(1, false), (3, true), (0, true)],
        [(0, false), (4, false), (2, true)],
        [(2, false), (5, true), (1, true)],
        [(3, false), (5, false), (4, true)],
    ];
    for coedges in loops {
        m.loops.push(Loop {
            coedges: coedges
                .into_iter()
                .map(|(edge, reversed)| Coedge {
                    edge: be + edge,
                    reversed,
                    pcurve: (),
                })
                .collect(),
        });
    }
    for i in 0..4 {
        m.faces.push(Face {
            surface: (),
            outer: bl + i,
            holes: vec![],
        });
    }
    m.shells.push(Shell {
        faces: (0..4)
            .map(|i| FaceUse {
                face: bf + i,
                reversed: false,
            })
            .collect(),
        closed: true,
    });
    m.bodies.push(Body {
        outer_shell: m.shells.len() - 1,
        inner_shells: vec![],
    });
}

fn random_model(rng: &mut Rng) -> M {
    let mut m = Model {
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        tolerance_mm: 1e-7,
    };
    for _ in 0..1 + rng.below(4) {
        append_tetra(&mut m, rng);
    }
    m.validate_topology().unwrap();
    m
}

/// One random honest edit; returns the scope describing what it touched.
fn random_edit(m: &mut M, rng: &mut Rng) -> EditScope {
    let mut scope = EditScope::none();
    for _ in 0..1 + rng.below(3) {
        match rng.below(13) {
            // Tolerance only: no topology entries needed.
            0 => m.tolerance_mm = if rng.bool() { 2e-6 } else { 1e-9 },
            // Move a vertex within the admission bounds.
            1 => {
                let i = rng.below(m.vertices.len());
                m.vertices[i].point = [rng.coord(), rng.coord(), rng.coord()];
                scope = scope.vertex(i);
            }
            // Move a vertex beyond the admission bound (|x| <= 50).
            2 => {
                let i = rng.below(m.vertices.len());
                m.vertices[i].point = [500., 0., 0.];
                scope = scope.vertex(i);
            }
            // Rewire an edge, possibly breaking loop chains.
            3 => {
                let i = rng.below(m.edges.len());
                let a = rng.below(m.vertices.len());
                m.edges[i].vertices[0] = a;
                scope = scope.edge(i);
            }
            // Toggle an edge's degenerate flag with matching pole vertices.
            4 => {
                let i = rng.below(m.edges.len());
                let v = m.edges[i].vertices[0];
                m.edges[i].degenerate = !m.edges[i].degenerate;
                if m.edges[i].degenerate {
                    m.edges[i].vertices = [v, v];
                }
                scope = scope.edge(i);
            }
            // Flip a coedge direction.
            5 => {
                let i = rng.below(m.loops.len());
                let j = rng.below(m.loops[i].coedges.len());
                m.loops[i].coedges[j].reversed = !m.loops[i].coedges[j].reversed;
                scope = scope.loop_(i);
            }
            // Repoint a coedge at another edge, possibly out of range.
            6 => {
                let i = rng.below(m.loops.len());
                let j = rng.below(m.loops[i].coedges.len());
                m.loops[i].coedges[j].edge = rng.below(m.edges.len() + 2);
                scope = scope.loop_(i);
            }
            // Repoint a face at another loop, possibly reused/out of range.
            7 => {
                let i = rng.below(m.faces.len());
                m.faces[i].outer = rng.below(m.loops.len() + 2);
                scope = scope.face(i);
            }
            // Toggle shell closed.
            8 => {
                let i = rng.below(m.shells.len());
                m.shells[i].closed = !m.shells[i].closed;
                scope = scope.shell(i);
            }
            // Flip a face use or duplicate one inside a shell.
            9 => {
                let i = rng.below(m.shells.len());
                let j = rng.below(m.shells[i].faces.len());
                if rng.bool() {
                    m.shells[i].faces[j].reversed = !m.shells[i].faces[j].reversed;
                } else {
                    let u = m.shells[i].faces[j].clone();
                    m.shells[i].faces.push(u);
                }
                scope = scope.shell(i);
            }
            // Repoint a body at another shell.
            10 => {
                let i = rng.below(m.bodies.len());
                m.bodies[i].outer_shell = rng.below(m.shells.len());
            }
            // Append a fresh tetrahedron: appended indices are implicit.
            11 => append_tetra(m, rng),
            // Structural removal: forces full validation regardless of scope.
            _ => {
                let i = rng.below(m.vertices.len());
                m.vertices.remove(i);
            }
        }
    }
    scope
}

fn outcome(r: math_core::Result<()>) -> std::result::Result<(), (String, String)> {
    r.map_err(|e| (e.code.to_string(), e.message))
}

#[test]
fn incremental_revalidation_matches_full_validation() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for case in 0..3000 {
        let model = random_model(&mut rng);
        let mut candidate = model.clone();
        let scope = random_edit(&mut candidate, &mut rng);
        let expected = outcome(candidate.validate_topology_with_vertices(|p: &[f64; 3]| {
            math_core::ensure(
                p.iter().all(|x| x.is_finite() && x.abs() <= 50.),
                "BREP_INVALID_TOPOLOGY",
                "Invalid vertex coordinates",
            )
        }));
        let snapshot = TopologySnapshot::new(model, |p: &[f64; 3]| {
            math_core::ensure(
                p.iter().all(|x| x.is_finite() && x.abs() <= 50.),
                "BREP_INVALID_TOPOLOGY",
                "Invalid vertex coordinates",
            )
        })
        .unwrap();
        let edited = candidate.clone();
        let declared = scope.clone();
        let result = snapshot.try_edit_scoped(move |m| {
            *m = edited;
            Ok(declared)
        });
        match (expected, result) {
            (Ok(()), Ok(_)) => {}
            (Err(e), Err(got)) => {
                assert_eq!((got.code.to_string(), got.message.clone()), e, "case {case}");
            }
            (expected, result) => panic!(
                "case {case}: full={:?} incremental={:?}",
                expected,
                result.map(|_| ()).map_err(|e| (e.code.to_string(), e.message))
            ),
        }
    }
}
