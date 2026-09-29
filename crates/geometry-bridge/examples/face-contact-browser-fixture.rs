use value_codec::json;
fn main() {
    let mut a = brep_core::cuboid([0.; 3], [1.; 3]).unwrap().0;
    let mut b = brep_core::cuboid([0.25, 0.25, 0.25], [1.25; 3]).unwrap().0;
    for e in &mut b.edges {
        for v in &mut e.vertices {
            *v += a.vertices.len();
        }
    }
    for l in &mut b.loops {
        for c in &mut l.coedges {
            c.edge += a.edges.len();
        }
    }
    for f in &mut b.faces {
        f.outer += a.loops.len();
        for h in &mut f.holes {
            *h += a.loops.len();
        }
    }
    for sh in &mut b.shells {
        for f in &mut sh.faces {
            f.face += a.faces.len();
        }
    }
    for body in &mut b.bodies {
        body.outer_shell += a.shells.len();
        for sh in &mut body.inner_shells {
            *sh += a.shells.len();
        }
    }
    a.vertices.extend(b.vertices);
    a.edges.extend(b.edges);
    a.loops.extend(b.loops);
    a.faces.extend(b.faces);
    a.shells.extend(b.shells);
    a.bodies.extend(b.bodies);
    let model: brep_core::Model =
        value_codec::from_value(value_codec::to_value(&a).unwrap()).unwrap();
    model.validate().unwrap();
    let limits = json!({"maxPairs":10000,"maxCells":10000,"maxDomainCells":1000000,"cellsPerPair":500,"domainCellsPerPair":50000,"maxBoxes":64});
    let mut request = limits.clone();
    request["op"] = json!("cad_face_contacts");
    request["model"] = json!(model);
    request["toleranceUv"] = json!(1e-8);
    let result = geometry_bridge::dispatch(request).unwrap();
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for (i, offset) in [0., 0.25].iter().enumerate() {
        for p in [
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ] {
            positions.extend(p.map(|x| x + offset));
        }
        indices.extend(
            [
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2,
                7, 6, 3, 0, 4, 3, 4, 7,
            ]
            .map(|x| x + i * 8),
        );
    }
    let document = json!({"version":1,"sketches":[],"bodies":[{"id":"contact-body","name":"Contact fixture","brep":model,"mesh":{"positions":positions,"indices":indices}}]});
    println!(
        "{}",
        json!({"document":document,"result":result,"limits":limits})
    );
}
