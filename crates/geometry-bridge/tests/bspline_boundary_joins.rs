//! Full native protocol, actual periodic B-rep ownership and strict refusals.
use brep_core::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use geometry_bridge::dispatch;
use nurbs_core::{curve::Curve, surface::Surface};
use value_codec::json;

fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1.; 2],
        periodic: false,
    }
}
fn periodic_torus() -> Model {
    let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    let knots = (0..9).map(|i| i as f64).collect::<Vec<_>>();
    let surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: knots.clone(),
        knots_v: knots.clone(),
        control_points: (0..6)
            .map(|i| {
                (0..6)
                    .map(|j| {
                        vec![
                            (4. + xy[j % 4][0]) * xy[i % 4][0],
                            (4. + xy[j % 4][0]) * xy[i % 4][1],
                            xy[j % 4][1],
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 6]; 6],
        periodic_u: true,
        periodic_v: true,
    };
    let curve = |axis| Curve {
        degree: 2,
        knots: knots.clone(),
        control_points: (0..6)
            .map(|i| {
                if axis == 0 {
                    vec![4.5 * xy[i % 4][0], 4.5 * xy[i % 4][1], 0.5]
                } else {
                    vec![
                        (4. + xy[i % 4][0]) * 0.5,
                        (4. + xy[i % 4][0]) * 0.5,
                        xy[i % 4][1],
                    ]
                }
            })
            .collect(),
        weights: vec![1.; 6],
        periodic: true,
    };
    let uv = [[2., 2.], [6., 2.], [6., 6.], [2., 6.]];
    let mut model = Model(
        brep_topology::Model {
            vertices: vec![Vertex {
                point: [2.25, 2.25, 0.5],
            }],
            edges: vec![
                Edge {
                    vertices: [0, 0],
                    curve: curve(0),
                    degenerate: false,
                },
                Edge {
                    vertices: [0, 0],
                    curve: curve(1),
                    degenerate: false,
                },
            ],
            loops: vec![Loop {
                coedges: (0..4)
                    .map(|i| {
                        Coedge::new(
                            i % 2,
                            i >= 2,
                            line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec()),
                        )
                    })
                    .collect(),
            }],
            faces: vec![Face {
                surface,
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model
}
#[test]
fn doubly_periodic_bspline_body_has_owned_world_traces_and_all_g2_seams() {
    let model = periodic_torus();
    model.validate().unwrap();
    let before = model.clone();
    let request = json!({"op":"brep_complete_boundary_smoothness_audit","model":model,"order":2,
        "maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,
        "seams":[],"capFaces":[],"solidCertified":true});
    let r = dispatch(request.clone()).unwrap();
    assert_eq!(r["extractionComplete"], true, "{r:?}");
    assert_eq!(r["edgeCorrespondenceCertified"], true, "{r:?}");
    assert_eq!(r["vertexLinksComplete"], true);
    assert_eq!(r["edgeIds"].as_array().unwrap().len(), model.edges.len());
    assert_eq!(r["fullBoundaryG1Certified"], true, "{r:?}");
    assert_eq!(r["fullBoundaryG2Certified"], true, "{r:?}");
    assert_eq!(r["solidCertified"], false); // G2 is not global material validity
    assert_eq!(model, before);
    let work = r["exactWork"].as_u64().unwrap();
    assert!(work <= 2000000);
    let mut short = request.clone();
    short["maxExactWork"] = json!(work - 1);
    assert_eq!(dispatch(short).unwrap()["fullBoundaryG2Certified"], false);
    let mut short = request;
    short["maxAgreementWork"] = json!(r["agreementWork"].as_u64().unwrap() - 1);
    assert_eq!(dispatch(short).unwrap()["fullBoundaryG2Certified"], false);
}
#[test]
fn periodic_flags_approximate_knots_and_mutated_world_edges_cannot_forge_g2() {
    let original = periodic_torus();
    for mode in 0..3 {
        let mut model = original.clone();
        match mode {
            0 => model.faces[0].surface.knots_u[1] = f64::from_bits(1f64.to_bits() + 1),
            1 => model.edges[0].curve.control_points[2][2] += 0.125,
            _ => model.faces[0].surface.weights[2][2] = 2.,
        }
        // Keep periodic storage structurally valid when changing a weight.
        if mode == 2 {
            model.faces[0].surface.weights[2][2] = 1.;
            for i in [0, 4] {
                for j in [0, 4] {
                    model.faces[0].surface.weights[i][j] = 2.;
                }
            }
        }
        let result = dispatch(
            json!({"op":"brep_complete_boundary_smoothness_audit","model":model,"order":2,
            "maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,
            "fullBoundaryG2Certified":true,"seams":[],"capFaces":[]}),
        );
        match result {
            Ok(r) => {
                assert_eq!(r["fullBoundaryG2Certified"], false, "mode {mode}: {r:?}");
                assert_eq!(r["solidCertified"], false);
            }
            Err(e) => assert_eq!(e.code, "BREP_INVALID_TOPOLOGY", "mode {mode}: {e:?}"),
        }
    }
}
