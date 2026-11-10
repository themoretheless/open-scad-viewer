//! Independent guards for a constructive six-patch corner, not solid admission.
use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex, face_curvature as c, face_domain, region_replacement::propose_corner_patch_network};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
use std::collections::BTreeMap;
fn edge(s: &Surface, e: usize) -> Curve {
    match e {
        0 => s.iso(Axis::V, 0.),
        1 => s.iso(Axis::U, 1.),
        2 => s.iso(Axis::V, 1.).and_then(|c| c.reverse()),
        _ => s.iso(Axis::U, 0.).and_then(|c| c.reverse()),
    }
    .unwrap()
}
fn sheet(s: &Surface) -> Model {
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let curves = (0..4).map(|e| edge(s, e)).collect::<Vec<_>>();
    let mut m = Model(
        brep_topology::Model {
            vertices: curves
                .iter()
                .map(|c| Vertex {
                    point: c.control_points[0].clone().try_into().unwrap(),
                })
                .collect(),
            edges: curves
                .into_iter()
                .enumerate()
                .map(|(i, curve)| Edge {
                    vertices: [i, (i + 1) % 4],
                    curve,
                    degenerate: false,
                })
                .collect(),
            loops: vec![Loop {
                coedges: (0..4)
                    .map(|i| {
                        Coedge::new(
                            i,
                            false,
                            Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()])
                                .unwrap(),
                        )
                    })
                    .collect(),
            }],
            faces: vec![Face {
                surface: s.clone(),
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        Default::default(),
    );
    m.rebuild_topology_ids();
    m
}
fn main() {
    let a = std::env::args().collect::<Vec<_>>();
    assert!(a.len() == 2 || a.len() == 3, "output_dir [corner_radius]");
    let dir = std::path::Path::new(&a[1]);
    std::fs::create_dir_all(dir).unwrap();
    let radius = a.get(2).map(|s| s.parse().unwrap()).unwrap_or(0.75);
    let network = propose_corner_patch_network(radius, [0.13, 0.2, 0.35], 5., 0.214).unwrap();
    let surfaces = network
        .caps
        .iter()
        .chain(&network.strips)
        .collect::<Vec<_>>();
    let mut face_reports = Vec::new();
    for (i, s) in surfaces.iter().enumerate() {
        eprintln!("phase=face-{i}");
        let m = sheet(s);
        let material = face_domain::inspect_box_containment(
            &m,
            0,
            [[-1e-7, 10.0000001]; 3],
            1e-8,
            50000,
            1000000,
        )
        .unwrap();
        let curvature = c::inspect(
            &m,
            0,
            10.,
            1e-8,
            c::Limits {
                cells: 50000,
                domain_cells: 1000000,
                shape_cells: 1000000,
            },
            &[],
        )
        .unwrap();
        let r = value_codec::json!({"face":i,"box":format!("{:?}",material.decision),"curvature":format!("{:?}",curvature.decision),"upper":curvature.absolute_curvature_upper,"witness":curvature.witness_curvature,"cells":curvature.cells});
        eprintln!("{r:?}");
        face_reports.push(r);
        std::fs::write(dir.join("faces.json"),value_codec::to_string(&value_codec::json!({"faces":face_reports,"surfaces":surfaces,"radius":radius,"farRadii":[0.13,0.2,0.35],"accepted":false})).unwrap()).unwrap();
    }
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut seams = BTreeMap::<(usize, usize), (Curve, Vec<(usize, Curve)>)>::new();
    for f in 0..3 {
        for e in 0..4 {
            let mut nodes = [network.cap_nodes[f][e], network.cap_nodes[f][(e + 1) % 4]];
            let mut curve = edge(&network.caps[f], e);
            let mut pc =
                Curve::from_polyline(vec![uv[e].to_vec(), uv[(e + 1) % 4].to_vec()]).unwrap();
            if nodes[0] > nodes[1] {
                nodes.reverse();
                curve = curve.reverse().unwrap();
                pc = pc.reverse().unwrap();
            }
            let entry = seams
                .entry((nodes[0], nodes[1]))
                .or_insert_with(|| (curve, vec![]));
            entry.1.push((f, pc));
        }
    }
    let mut reports = Vec::new();
    for ((a, b), (curve, mut uses)) in seams {
        if uses.len() == 1 {
            let start = network.barycentric_nodes[a];
            let end = network.barycentric_nodes[b];
            let axis = (0..3).find(|&k| start[k] == 0. && end[k] == 0.).unwrap();
            let pc = Curve::from_polyline(vec![
                vec![start[(axis + 1) % 3], 0.],
                vec![end[(axis + 1) % 3], 0.],
            ])
            .unwrap();
            uses.push((3 + axis, pc));
        }
        assert_eq!(uses.len(), 2);
        eprintln!("phase=seam-{a}-{b}");
        let proof = nurbs_core::surface_g2_pcurves::inspect_owned(
            &curve,
            [&uses[0].1, &uses[1].1],
            [surfaces[uses[0].0], surfaces[uses[1].0]],
            [false; 2],
            1e-7,
            1e-5,
            1e-4,
            100000,
            1000000,
        )
        .unwrap();
        let r = value_codec::json!({"nodes":[a,b],"faces":[uses[0].0,uses[1].0],"positions":proof.report.positions.proven,"g2":format!("{:?}",proof.report.geometric.decision),"cells":proof.report.geometric.cells,"branches":proof.report.geometric.branches.iter().map(|b|value_codec::json!({"normalUpper":b.normal_upper,"tensorUpper":b.tensor_upper})).collect::<Vec<_>>()});
        eprintln!("{r:?}");
        reports.push(r);
        std::fs::write(dir.join("seams.json"),value_codec::to_string(&value_codec::json!({"seams":reports,"accepted":false,"scope":"Whole nine internal/strip interface seams; six surfaces only, no retained cube faces or closed solid"})).unwrap()).unwrap();
    }
}
