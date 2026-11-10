//! Whole authored UV-face guards, independent of sewing/solid admission.
use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex, face_curvature as c, face_domain::{self, BoxDecision}};
use nurbs_core::{curve::Curve, surface::Surface};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "candidate.json output.json");
    let data: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let surface: Surface = value_codec::from_value(data["surface"].clone()).unwrap();
    let entries = data["boundaries"].as_array().unwrap();
    assert_eq!(entries.len(), 6);
    let curves = entries
        .iter()
        .map(|b| value_codec::from_value::<Curve>(b["curve"].clone()).unwrap())
        .collect::<Vec<_>>();
    let pcs = entries
        .iter()
        .map(|b| value_codec::from_value::<Curve>(b["candidatePcurve"].clone()).unwrap())
        .collect::<Vec<_>>();
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    model.vertices = curves
        .iter()
        .map(|c| Vertex {
            point: c.control_points[0].clone().try_into().unwrap(),
        })
        .collect();
    model.edges = curves
        .iter()
        .enumerate()
        .map(|(i, c)| {
            assert_eq!(
                c.control_points.last().unwrap(),
                &curves[(i + 1) % 6].control_points[0]
            );
            Edge {
                vertices: [i, (i + 1) % 6],
                curve: c.clone(),
                degenerate: false,
            }
        })
        .collect();
    model.loops = vec![Loop {
        coedges: pcs
            .into_iter()
            .enumerate()
            .map(|(i, p)| Coedge::new(i, false, p))
            .collect(),
    }];
    model.faces = vec![Face {
        surface,
        outer: 0,
        holes: vec![],
    }];
    model.shells = vec![Shell {
        faces: vec![FaceUse {
            face: 0,
            reversed: false,
        }],
        closed: false,
    }];
    model.bodies.clear();
    model.rebuild_topology_ids();
    let material = face_domain::inspect_box_containment(
        &model,
        0,
        [[-1e-7, 10.0000001]; 3],
        1e-8,
        50000,
        1000000,
    )
    .unwrap();
    let mut report = value_codec::json!({"material":format!("{:?}",material.decision),"materialCells":material.cells,"materialWitnessUV":material.witness_uv,"materialWitnessBounds":material.witness_bounds,"accepted":false,"scope":"One unsewn authored UV-face. No whole-seam, sewing, solid or volume admission."});
    std::fs::write(&args[2], value_codec::to_string(&report).unwrap()).unwrap();
    if material.decision != BoxDecision::Within {
        return;
    }
    let curvature = c::inspect(
        &model,
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
    report["curvature"] = value_codec::json!({"decision":format!("{:?}",curvature.decision),"upper":curvature.absolute_curvature_upper,"witness":curvature.witness_curvature,"witnessUV":curvature.witness_uv,"cells":curvature.cells});
    std::fs::write(&args[2], value_codec::to_string(&report).unwrap()).unwrap();
}
