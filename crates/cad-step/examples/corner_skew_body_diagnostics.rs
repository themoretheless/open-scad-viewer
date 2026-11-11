//! Fresh oblique trihedral qualification; no inherited G2/curvature receipts.
use brep_core::{face_curvature as c, face_domain, region_replacement as r, watertight};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert!(args.len() == 2 || args.len() == 3, "output_dir [radius]");
    let dir = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(dir).unwrap();
    let radius = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(1.25);
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let binary = true;
    let net = if binary {
        r::propose_binary_corner_patch_network(radius, [0.25, 0.35, 0.5], 5., 0.214)
    } else {
        r::propose_corner_patch_network(radius, [0.25, 0.35, 0.5], 5., 0.214)
    }
    .unwrap();
    let requested_radius = radius;
    let radius = net.strips[0].control_points[0][0][0];
    let far_radii = std::array::from_fn::<_, 3, _>(|axis| {
        net.strips[axis].control_points[0][5][(axis + 1) % 3]
    });
    eprintln!("phase=assemble-closed-body");
    let mut body = match r::propose_corner_patch_body(&source, &net) {
        Ok(v) => v,
        Err(e) => {
            std::fs::write(
                dir.join("refused.json"),
                value_codec::to_string(
                    &value_codec::json!({"error":format!("{e}"),"accepted":false}),
                )
                .unwrap(),
            )
            .unwrap();
            return;
        }
    };
    assert_eq!(source, before);
    let original = body.model.clone();
    let placement = [
        [2., 1., 0., 0.],
        [0., 2., 0., 0.],
        [0., 0., 2., 0.],
        [0., 0., 0., 1.],
    ];
    body.model = brep_core::transform::affine(&body.model, placement).unwrap();
    let inverse = [
        [0.5, -0.25, 0., 0.],
        [0., 0.5, 0., 0.],
        [0., 0., 0.5, 0.],
        [0., 0., 0., 1.],
    ];
    let pulled = brep_core::transform::affine(&body.model, inverse).unwrap();
    assert_eq!(
        pulled.0, original.0,
        "Material pullback must restore every authored coefficient exactly"
    );
    std::fs::write(dir.join("body.json"),value_codec::to_string(&value_codec::json!({"model":body.model,"capFaces":body.cap_faces,"stripFaces":body.strip_faces,"stopFaces":body.stop_faces,"smoothEdges":body.smooth_edges,"radius":radius,"farRadii":far_radii,"requestedRadius":requested_radius,"binaryBoundaries":binary,"placement":placement,"exactMaterialPullback":true,"accepted":false})).unwrap()).unwrap();
    let closure = watertight::inspect_with_exact(&body.model, 1000000, 1000000).unwrap();
    let trims = face_domain::audit_trim_regions(&body.model, 1e-8, 10000, 100000, 1000000).unwrap();
    let mut all = closure.closure.status == watertight::Closure::WithinTolerance && trims.all_valid;
    let mut fields = Vec::new();
    for f in 0..body.model.faces.len() {
        eprintln!("phase=body-face-{f}");
        let k = c::inspect(
            &body.model,
            f,
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
        let m = face_domain::inspect_box_containment(
            &pulled,
            f,
            [[-1e-7, 10.0000001]; 3],
            1e-8,
            50000,
            1000000,
        )
        .unwrap();
        all &= k.decision == c::Decision::Within && m.decision == face_domain::BoxDecision::Within;
        fields.push(value_codec::json!({"face":f,"curvature":format!("{:?}",k.decision),"upper":k.absolute_curvature_upper,"box":format!("{:?}",m.decision)}));
        std::fs::write(
            dir.join("fields.json"),
            value_codec::to_string(&value_codec::json!({"faces":fields,"accepted":false})).unwrap(),
        )
        .unwrap();
    }
    let mut seams = Vec::new();
    for &e in &body.smooth_edges {
        eprintln!("phase=body-seam-{e}");
        let mut uses = Vec::new();
        for (f, face) in body.model.faces.iter().enumerate() {
            for co in &body.model.loops[face.outer].coedges {
                if co.edge == e {
                    uses.push((
                        f,
                        if co.reversed {
                            co.pcurve.reverse().unwrap()
                        } else {
                            co.pcurve.clone()
                        },
                    ));
                }
            }
        }
        assert_eq!(uses.len(), 2);
        let p = nurbs_core::surface_g2_pcurves::inspect_owned(nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &body.model.edges[e].curve, pcurves: [&uses[0].1, &uses[1].1], surfaces: [
                &body.model.faces[uses[0].0].surface,
                &body.model.faces[uses[1].0].surface,
            ], reversed: [false; 2] }, nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 }, 100000, 1000000)
        .unwrap();
        all &= p.report.positions.proven
            && p.report.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance;
        seams.push(value_codec::json!({"edge":e,"faces":[uses[0].0,uses[1].0],"positions":p.report.positions.proven,"g2":format!("{:?}",p.report.geometric.decision),"cells":p.report.geometric.cells,"branches":p.report.geometric.branches.iter().map(|b|value_codec::json!({"normalUpper":b.normal_upper,"tensorUpper":b.tensor_upper})).collect::<Vec<_>>()}));
        std::fs::write(
            dir.join("seams.json"),
            value_codec::to_string(&value_codec::json!({"seams":seams,"accepted":false})).unwrap(),
        )
        .unwrap();
    }
    let (step, _, _) = cad_step::export_step_v5(&body.model).unwrap();
    std::fs::write(dir.join("candidate.step"), step).unwrap();
    let report = value_codec::json!({"qualifiedForDeclaredGates":all,"closure":format!("{:?}",closure.closure.status),"allTrimsValid":trims.all_valid,"faces":body.model.faces.len(),"edges":body.model.edges.len(),"vertices":body.model.vertices.len(),"euler":brep_topology::explorer::euler_characteristic(&body.model.0),"fields":fields,"seams":seams,"radius":radius,"farRadii":far_radii,"requestedRadius":requested_radius,"binaryBoundaries":binary,"placement":placement,"exactMaterialPullback":true,"farStops":"planar sharp; not G2","globalSelfIntersectionQualified":false,"scope":"Oblique trihedral from an affinely placed cube: whole world-space curvature and G2, source bounds via exact inverse material pullback. Stops remain sharp; not arbitrary corner valence or a global self-intersection receipt."});
    std::fs::write(
        dir.join("report.json"),
        value_codec::to_string(&report).unwrap(),
    )
    .unwrap();
    eprintln!("qualifiedForDeclaredGates={all}");
}
