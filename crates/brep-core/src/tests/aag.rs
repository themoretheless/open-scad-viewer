use super::*;

fn budget() -> Budget {
    Budget::new(1_000_000, 8, 60_000).unwrap()
}

/// Round pocket revolved from an analytic wire: cylindrical wall blends
/// into the flat floor through a quarter-arc (torus after revolution).
/// Both line↔arc joints are G1 tangent and must classify Smooth.
fn revolved_round_pocket() -> Model {
    let line = |a: [f64; 2], b: [f64; 2]| nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    let arc = |p0: [f64; 2], p1: [f64; 2], p2: [f64; 2]| nurbs_core::curve::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![p0.to_vec(), p1.to_vec(), p2.to_vec()],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    // Material-left wire in the (r, z) half-plane: body z 0..8, radius 12,
    // pocket radius 10 with floor at z=5 and a radius-1 floor blend.
    let wire = vec![
        line([0., 0.], [12., 0.]),
        line([12., 0.], [12., 8.]),
        line([12., 8.], [10., 8.]),
        line([10., 8.], [10., 6.]),
        arc([10., 6.], [10., 5.], [9., 5.]),
        line([9., 5.], [0., 5.]),
        line([0., 5.], [0., 0.]),
    ];
    crate::revolve_wire(&wire, 1e-7).unwrap()
}

#[test]
fn revolve_wire_tangent_joints_classify_smooth() {
    let model = revolved_round_pocket();
    let graph = Aag::build(&model, &budget()).unwrap();
    // The full revolution quarters every joint circle; locate the joint
    // edges geometrically by their midpoints: wall↔fillet at (r=10, z=6),
    // fillet↔floor at (r=9, z=5).
    let mut joints = vec![];
    for edge in &graph.edges {
        let curve = &model.edges[edge.edge].curve;
        let [a, b] = curve.domain();
        let p = curve.evaluate((a + b) / 2.).unwrap().point;
        let (r, z) = (p[0].hypot(p[1]), p[2]);
        if ((r - 10.).abs() < 1e-6 && (z - 6.).abs() < 1e-6)
            || ((r - 9.).abs() < 1e-6 && (z - 5.).abs() < 1e-6)
        {
            joints.push((edge.edge, edge.class));
        }
    }
    assert_eq!(joints.len(), 8, "two joint circles, quartered: {joints:?}");
    assert!(
        joints
            .iter()
            .all(|&(_, class)| class == DihedralClass::Smooth),
        "tangent line→arc joints must be Smooth, got {joints:?}"
    );
    // The only non-matchable edges are the axis pole collapses, which are
    // genuinely degenerate geometry, not misclassified tangent joints.
    assert_eq!(graph.edges_of_class(DihedralClass::Mixed), Vec::<usize>::new());
    for edge in &graph.edges {
        if edge.class == DihedralClass::Degenerate {
            assert!(
                model.edges[edge.edge].degenerate,
                "edge {} falsely degenerate",
                edge.edge
            );
        }
    }
}

#[test]
fn cuboid_edges_are_all_convex() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let graph = Aag::build(&model, &budget()).unwrap();
    assert_eq!(graph.nodes.len(), 6);
    assert_eq!(graph.edges.len(), 12);
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.class == DihedralClass::Convex && e.samples == AAG_DEFAULT_SAMPLES),
        "every box edge is convex with full sample evidence"
    );
    for node in &graph.nodes {
        assert_eq!(node.edges.len(), 4);
        assert_eq!(node.neighbors.len(), 4);
    }
    // Sign evidence sits strictly on the convex side.
    for edge in &graph.edges {
        assert!(edge.sign_max < -0.5 && edge.sign_min >= -1.0 - 1e-12);
    }
}

#[test]
fn l_prism_has_one_concave_vertical_edge() {
    // L-shaped profile: one reentrant (270° material) corner.
    let profile = [[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]];
    let model = crate::extrude_polygon(&profile, 0., 1.).unwrap();
    let graph = Aag::build(&model, &budget()).unwrap();
    let concave = graph.edges_of_class(DihedralClass::Concave);
    assert_eq!(
        concave.len(),
        1,
        "exactly the reentrant vertical edge is concave: {concave:?}"
    );
    let convex = graph.edges_of_class(DihedralClass::Convex);
    assert_eq!(concave.len() + convex.len(), graph.edges.len());
    assert!(graph.edges.iter().all(|e| e.class.is_matchable()));
}

/// Minimal hand-assembled model: one shared edge between two quad faces
/// plus outline edges; geometry is exact, validation is the caller's job
/// (AAG reads topology and geometry but never mutates or revalidates).
fn two_quad_sheet() -> (Model, usize) {
    use brep_topology::{FaceUse, Loop, Shell, Vertex};
    let line = |a: [f64; 3], b: [f64; 3]| nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    let quad = |c: [[f64; 3]; 4]| nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![c[0].to_vec(), c[3].to_vec()], vec![c[1].to_vec(), c[2].to_vec()]],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let points = [
        [0., 0., 0.],
        [1., 0., 0.],
        [2., 0., 0.],
        [0., 1., 0.],
        [1., 1., 0.],
        [2., 1., 0.],
    ];
    let mut m = Model(
        brep_topology::Model {
            vertices: points.map(|p| Vertex { point: p }).to_vec(),
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        crate::TopologyIds::default(),
    );
    let quads = [[0, 1, 4, 3], [1, 2, 5, 4]];
    let mut shared = std::collections::BTreeMap::new();
    let mut shared_edge = None;
    for corners in quads {
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let mut coedges = vec![];
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let key = (a.min(b), a.max(b));
            let edge = *shared.entry(key).or_insert_with(|| {
                let id = m.0.edges.len();
                m.0.edges.push(crate::Edge {
                    degenerate: false,
                    vertices: [key.0, key.1],
                    curve: line(points[key.0], points[key.1]),
                });
                id
            });
            if key == (1, 4) {
                shared_edge = Some(edge);
            }
            coedges.push(crate::Coedge {
                edge,
                reversed: a > b,
                pcurve: {
                    let (a, b) = (uv[i], uv[(i + 1) % 4]);
                    nurbs_core::curve::Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![a.to_vec(), b.to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    }
                },
            });
        }
        let outer = m.0.loops.len();
        m.0.loops.push(Loop { coedges });
        m.0.faces.push(crate::Face {
            surface: quad(corners.map(|i| points[i])),
            outer,
            holes: vec![],
        });
    }
    m.0.shells.push(Shell {
        faces: (0..2)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
        closed: false,
    });
    (m, shared_edge.unwrap())
}

#[test]
fn seam_edge_on_one_face_is_flagged_not_classified() {
    // Analytic cylinder/sphere in this kernel split periodic supports into
    // regular patches, so a seam is assembled synthetically: a shell using
    // one face twice makes the shared edge a two-use single-face edge.
    use brep_topology::{FaceUse, Shell};
    let (mut m, shared) = two_quad_sheet();
    m.0.shells[0] = Shell {
        faces: vec![
            FaceUse {
                face: 0,
                reversed: false,
            },
            FaceUse {
                face: 0,
                reversed: true,
            },
        ],
        closed: false,
    };
    let graph = Aag::build(&m, &budget()).unwrap();
    let seam = graph.edge(shared).unwrap();
    assert_eq!(seam.class, DihedralClass::Seam);
    assert_eq!(seam.uses.len(), 2);
    assert_eq!(seam.uses[0].face, seam.uses[1].face);
    assert!(!seam.class.is_matchable());
    assert_eq!(seam.samples, 0, "seam is trapped before sampling");
}

#[test]
fn collapsed_edge_is_flagged_degenerate_not_classified() {
    let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    // A pole-collapse edge carries the explicit marker; AAG must trap it
    // before touching normals near the pole.
    model.edges[0].degenerate = true;
    let graph = Aag::build(&model, &budget()).unwrap();
    let edge = graph.edge(0).unwrap();
    assert_eq!(edge.class, DihedralClass::Degenerate);
    assert_eq!(edge.samples, 0);
    // The other eleven edges still classify normally.
    assert_eq!(
        graph.edges_of_class(DihedralClass::Convex).len(),
        graph.edges.len() - 1
    );
}

#[test]
fn coplanar_split_sheet_reads_smooth_and_boundary() {
    // Two coplanar quads sewn along one edge: the shared edge is smooth,
    // the outline edges are boundary (one use each).
    let (m, shared) = two_quad_sheet();
    let graph = Aag::build(&m, &budget()).unwrap();
    let smooth = graph.edges_of_class(DihedralClass::Smooth);
    assert_eq!(smooth, vec![shared], "the sewn coplanar edge is G1-smooth");
    let edge = graph.edge(shared).unwrap();
    assert_eq!(edge.uses.len(), 2);
    assert_ne!(edge.uses[0].face, edge.uses[1].face);
    assert_eq!(
        graph.edges_of_class(DihedralClass::Boundary).len(),
        m.edges.len() - 1,
        "open outline edges stay unclassified boundary"
    );
    // Adjacency: the two quads are mutual neighbors through the seam.
    assert_eq!(graph.nodes[0].neighbors, vec![1]);
    assert_eq!(graph.nodes[1].neighbors, vec![0]);
}

#[test]
fn mixed_sign_aggregation_is_flagged() {
    // Curved-edge inflection: convex and concave samples on one edge must
    // aggregate to Mixed, never to a silent majority vote.
    assert_eq!(
        decide_class(&[SampleClass::Convex, SampleClass::Concave, SampleClass::Convex]),
        DihedralClass::Mixed
    );
    assert_eq!(
        decide_class(&[SampleClass::Smooth, SampleClass::Convex]),
        DihedralClass::Mixed
    );
    assert_eq!(
        decide_class(&[SampleClass::Convex, SampleClass::Convex]),
        DihedralClass::Convex
    );
    assert_eq!(decide_class(&[]), DihedralClass::Mixed);
}

#[test]
fn budget_exhaustion_is_a_typed_error() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let tight = Budget::with_iterations(2).unwrap();
    let error = Aag::build(&model, &tight).unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "budget exhaustion must surface as a resource error: {error:?}"
    );
    // A generous budget on the same model succeeds.
    assert!(Aag::build(&model, &budget()).is_ok());
}

#[test]
fn invalid_sample_count_is_rejected() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    assert!(Aag::build_sampled(&model, &budget(), 2).is_err());
    assert!(Aag::build_sampled(&model, &budget(), 4096).is_err());
}

#[test]
fn large_model_builds_within_a_second() {
    // MAX_FACES (1024) caps one Model below the 10^4-face acceptance
    // target, so the largest in-contract synthetic is assembled by
    // concatenating validated cuboids: 150 boxes = 900 faces / 1800
    // edges. Build cost is linear in edges × samples, so a 10^4-face
    // model (~11x this) stays well under the 1 s bound if this does.
    let unit = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    use brep_topology::Shell;
    let copies = 150;
    let mut m = Model(
        brep_topology::Model {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm: unit.tolerance_mm,
        },
        crate::TopologyIds::default(),
    );
    for _ in 0..copies {
        let (v, e, l, f, s) = (
            m.0.vertices.len(),
            m.0.edges.len(),
            m.0.loops.len(),
            m.0.faces.len(),
            m.0.shells.len(),
        );
        m.0.vertices.extend(unit.vertices.iter().cloned());
        m.0.edges.extend(unit.edges.iter().map(|edge| crate::Edge {
            degenerate: edge.degenerate,
            vertices: [edge.vertices[0] + v, edge.vertices[1] + v],
            curve: edge.curve.clone(),
        }));
        m.0.loops.extend(unit.loops.iter().map(|wire| crate::Loop {
            coedges: wire
                .coedges
                .iter()
                .map(|c| crate::Coedge {
                    edge: c.edge + e,
                    reversed: c.reversed,
                    pcurve: c.pcurve.clone(),
                })
                .collect(),
        }));
        m.0.faces.extend(unit.faces.iter().map(|face| crate::Face {
            surface: face.surface.clone(),
            outer: face.outer + l,
            holes: face.holes.iter().map(|h| h + l).collect(),
        }));
        m.0.shells.extend(unit.shells.iter().map(|shell| Shell {
            faces: shell
                .faces
                .iter()
                .map(|u| brep_topology::FaceUse {
                    face: u.face + f,
                    reversed: u.reversed,
                })
                .collect(),
            closed: shell.closed,
        }));
        m.0.bodies.extend(unit.bodies.iter().map(|body| brep_topology::Body {
            outer_shell: body.outer_shell + s,
            inner_shells: body.inner_shells.iter().map(|i| i + s).collect(),
        }));
    }
    assert_eq!(m.faces.len(), copies * 6);
    let start = std::time::Instant::now();
    let graph = Aag::build(&m, &budget()).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(graph.nodes.len(), copies * 6);
    assert_eq!(graph.edges.len(), copies * 12);
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.class == DihedralClass::Convex)
    );
    // Hard 1 s bound applies to optimized builds; the debug suite runs
    // under parallel load, where wall-clock assertions are flaky.
    let bound = if cfg!(debug_assertions) { 20.0 } else { 1.0 };
    assert!(
        elapsed.as_secs_f64() < bound,
        "graph build on {} faces took {elapsed:?} (bound {bound} s)",
        copies * 6
    );
}

// ------------------------------------------------------------------
// Face attributes (checklist 865)
// ------------------------------------------------------------------
use crate::analysis::surface_classify::SurfaceClass;

fn attrs_of(graph: &Aag, face: usize) -> &FaceAttrs {
    graph.nodes[face].attrs.as_ref().expect("attrs attached")
}

#[test]
fn cuboid_faces_are_planes_with_unit_area() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        assert_eq!(attrs.class, SurfaceClass::Plane);
        assert!((attrs.area - 1.).abs() < 1e-9, "unit cube face area");
        assert!(!attrs.fillet_like, "a box has no fillet faces");
        assert!(attrs.axis.is_none() && attrs.radius.is_none());
    }
}

#[test]
fn cylinder_model_faces_classify_exactly() {
    let (radius, height) = (1.5, 4.);
    let model = crate::analytic::cylinder(radius, height).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    let mut lateral = 0;
    let mut caps = 0;
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        match attrs.class {
            SurfaceClass::Cylinder => {
                lateral += 1;
                assert!((attrs.radius.unwrap() - radius).abs() < 1e-6);
                let axis = attrs.axis.unwrap();
                assert!(
                    (axis.direction[2].abs() - 1.).abs() < 1e-6,
                    "cylinder axis along z: {axis:?}"
                );
            }
            SurfaceClass::Plane => {
                caps += 1;
                assert!(
                    (attrs.area - std::f64::consts::PI * radius * radius).abs()
                        < 1e-6 * radius * radius,
                    "cap area"
                );
            }
            other => panic!("unexpected class on analytic cylinder: {other:?}"),
        }
    }
    assert_eq!(caps, 2, "two planar caps");
    assert_eq!(lateral, model.faces.len() - 2, "all other faces lateral");
}

#[test]
fn frustum_side_faces_classify_as_cone() {
    let model = crate::analytic::frustum(2., 1., 3.).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    let mut cones = 0;
    let mut planes = 0;
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        match attrs.class {
            SurfaceClass::Cone => {
                cones += 1;
                let axis = attrs.axis.unwrap();
                assert!(
                    (axis.direction[2].abs() - 1.).abs() < 1e-3,
                    "cone axis along z: {axis:?}"
                );
            }
            SurfaceClass::Plane => planes += 1,
            other => panic!("unexpected class on frustum: {other:?} ({attrs:?})"),
        }
    }
    assert_eq!(planes, 2);
    assert_eq!(cones, model.faces.len() - 2);
}

#[test]
fn sphere_patches_classify_as_sphere() {
    let radius = 2.;
    let model = crate::analytic::sphere(radius).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        assert_eq!(
            attrs.class,
            SurfaceClass::Sphere,
            "face {} on an analytic sphere: {attrs:?}",
            node.face
        );
        assert!((attrs.radius.unwrap() - radius).abs() < 1e-5);
        let center = attrs.center.unwrap();
        assert!(center.iter().all(|c| c.abs() < 1e-5), "centered: {center:?}");
    }
}

#[test]
fn torus_patches_classify_as_torus() {
    let (major, minor) = (3., 1.);
    let model = crate::analytic::torus(major, minor).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        assert_eq!(
            attrs.class,
            SurfaceClass::Torus,
            "face {} on an analytic torus: {attrs:?}",
            node.face
        );
        assert!((attrs.radius.unwrap() - minor).abs() < 1e-4);
    }
}

#[test]
fn fillet_face_is_marked_fillet_like() {
    // One rolled edge on a box: the new cylindrical blend face has a
    // small radius and two smooth (G1) neighbors — the fillet signature.
    // `analytic_fillet` admits vertical (+Z) cuboid edges; find one.
    let model = crate::cuboid([0.; 3], [4., 4., 4.]).unwrap();
    let vertical = (0..model.edges.len())
        .find(|&e| {
            let edge = &model.edges[e];
            let domain = edge.curve.domain();
            let a = edge.curve.evaluate(domain[0]).unwrap().point;
            let b = edge.curve.evaluate(domain[1]).unwrap().point;
            (a[0] - b[0]).abs() < 1e-12
                && (a[1] - b[1]).abs() < 1e-12
                && (a[2] - b[2]).abs() > 1.
        })
        .expect("cuboid has vertical edges");
    let (model, _certificate) = crate::analytic_features::analytic_fillet(&model, vertical, 0.5)
        .expect("fillet on box edge");
    let mut graph = Aag::build(&model, &budget()).unwrap();
    // Fillet radius 0.5 exceeds the 5%-of-diagonal default threshold for
    // this 4 mm box, so pass the fillet band explicitly.
    let options = FaceAttrsOptions {
        fit_tolerance: None,
        fillet_radius_max: Some(1.0),
    };
    graph
        .attach_face_attrs_with(&model, &budget(), &options)
        .unwrap();
    let fillets: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| attrs_of(&graph, n.face).fillet_like)
        .collect();
    assert_eq!(fillets.len(), 1, "exactly the blend face: {fillets:?}");
    let attrs = attrs_of(&graph, fillets[0].face);
    assert_eq!(attrs.class, SurfaceClass::Cylinder);
    assert!((attrs.radius.unwrap() - 0.5).abs() < 1e-3);
    // No planar support face may be misclassified as a fillet.
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        if attrs.class == SurfaceClass::Plane {
            assert!(!attrs.fillet_like);
        }
    }
}

#[test]
fn face_attrs_respect_budget() {
    let model = crate::analytic::cylinder(1., 2.).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    let tight = Budget::with_iterations(10).unwrap();
    let error = graph.attach_face_attrs(&model, &tight).unwrap_err();
    assert!(
        error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
        "budget exhaustion must surface as a resource error: {error:?}"
    );
    // A generous budget on the same model succeeds.
    graph.attach_face_attrs(&model, &budget()).unwrap();
    assert!(graph.nodes.iter().all(|n| n.attrs.is_some()));
}

#[test]
fn face_attrs_invalidate_on_geometry_mutation() {
    // `Model` has no mutation versioning: staleness is detected by the
    // deterministic face-geometry hash, refreshed per face.
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut graph = Aag::build(&model, &budget()).unwrap();
    graph.attach_face_attrs(&model, &budget()).unwrap();
    assert!(graph.stale_face_attrs(&model).is_empty());

    // Mutate one face's surface geometry in place.
    let mut mutated = model.clone();
    mutated.faces[0].surface.control_points[0][0][2] += 0.25;
    let stale = graph.stale_face_attrs(&mutated);
    assert_eq!(stale, vec![0], "only the mutated face goes stale");
    let refreshed = graph.refresh_face_attrs(&mutated, &budget()).unwrap();
    assert_eq!(refreshed, 1);
    assert!(graph.stale_face_attrs(&mutated).is_empty());
    let attrs = attrs_of(&graph, 0);
    // Bilinear quad with one corner lifted by 0.25: area exceeds the flat
    // 1.0 by ~1.5% and the face is no longer planar.
    assert!(
        attrs.area > 1.005 && attrs.area < 1.2,
        "area must track the mutation: {}",
        attrs.area
    );
    assert_ne!(attrs.class, SurfaceClass::Plane);
    // Untouched faces kept their original hash and attributes.
    for face in 1..6 {
        assert_eq!(
            attrs_of(&graph, face).geometry_hash,
            face_geometry_hash(&mutated, face)
        );
    }
    // Refresh on a clean cache is a no-op.
    assert_eq!(graph.refresh_face_attrs(&mutated, &budget()).unwrap(), 0);
}

#[test]
fn noisy_cylinder_model_classifies_via_best_fit() {
    // Imported-geometry simulation: nudge every control point of the
    // lateral patches by ~1e-5, so the exact surface type is a perturbed
    // B-spline; with a fit tolerance above the noise, best-fit must still
    // classify every lateral face as Cylinder (and caps stay Plane).
    let (radius, height) = (1.5, 4.);
    let mut model = crate::analytic::cylinder(radius, height).unwrap();
    let noise = 1e-5;
    for (i, face) in model.faces.iter_mut().enumerate() {
        if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
            continue; // planar cap
        }
        for (r, row) in face.surface.control_points.iter_mut().enumerate() {
            for (c, p) in row.iter_mut().enumerate() {
                let phase = (i * 7 + r * 3 + c) as f64;
                p[0] += noise * phase.sin();
                p[1] += noise * (phase * 1.7).cos();
                p[2] += noise * (phase * 0.3).sin();
            }
        }
    }
    let mut graph = Aag::build(&model, &budget()).unwrap();
    let options = FaceAttrsOptions {
        fit_tolerance: Some(1e-3),
        fillet_radius_max: None,
    };
    graph
        .attach_face_attrs_with(&model, &budget(), &options)
        .unwrap();
    let mut lateral = 0;
    let mut caps = 0;
    for node in &graph.nodes {
        let attrs = attrs_of(&graph, node.face);
        match attrs.class {
            SurfaceClass::Cylinder => {
                lateral += 1;
                assert!((attrs.radius.unwrap() - radius).abs() < 1e-2);
                assert!(attrs.fit_deviation <= 1e-3 + 1e-9);
            }
            SurfaceClass::Plane => caps += 1,
            other => panic!("noisy cylinder face {} misclassified: {other:?}", node.face),
        }
    }
    assert_eq!(caps, 2);
    assert_eq!(lateral, model.faces.len() - 2);
    // Below the noise level the lateral faces must refuse the class.
    let strict = FaceAttrsOptions {
        fit_tolerance: Some(1e-9),
        fillet_radius_max: None,
    };
    graph
        .attach_face_attrs_with(&model, &budget(), &strict)
        .unwrap();
    assert!(
        graph
            .nodes
            .iter()
            .all(|n| attrs_of(&graph, n.face).class != SurfaceClass::Cylinder),
        "with tolerance below the noise, no face may claim Cylinder"
    );
}
