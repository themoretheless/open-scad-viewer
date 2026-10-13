use super::*;

fn render(mut w: Writer) -> Result<String> {
    let mut p_lines = Vec::new();
    let mut p_start = Vec::new();
    for (index, e) in w.entities.iter().enumerate() {
        let de = 2 * index + 1;
        let text = format!("{};", e.values.join(","));
        p_start.push(p_lines.len() + 1);
        for chunk in text.as_bytes().chunks(64) {
            let payload =
                std::str::from_utf8(chunk).map_err(|_| refuse("Non-ASCII IGES parameter"))?;
            p_lines.push(
                pad_record(payload, 'P', p_lines.len() + 1, 64)?[..64].to_string()
                    + &format!("{de:>8}P{:>7}", p_lines.len() + 1),
            );
        }
    }
    let mut lines = vec![pad_record(
        "OpenSCAD Viewer direct finite B-rep",
        'S',
        1,
        72,
    )?];
    let global = format!(
        "1H,,1H;,{} ,{} ,{} ,{} ,{} ,{},32,38,6,308,15,{},1.0,2,2HMM,1,0.001,{},{} ,11,0,{} ,{};",
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith("OpenSCAD Viewer"),
        hollerith("open-scad-viewer"),
        hollerith(""),
        hollerith(""),
        hollerith("IGES2"),
        number(1e-7),
        hollerith("20260917.000000"),
        number(1e-7),
        hollerith("20260917.000000")
    );
    for (i, chunk) in global.as_bytes().chunks(72).enumerate() {
        lines.push(pad_record(
            std::str::from_utf8(chunk).unwrap(),
            'G',
            i + 1,
            72,
        )?);
    }
    for (index, e) in w.entities.iter_mut().enumerate() {
        let de = 2 * index + 1;
        let a = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}",
            e.ty, p_start[index], 0, 0, 0, 0, 0, 0, "00000000"
        );
        let b = format!(
            "{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:<8}{:>8}",
            e.ty, 0, e.color, 0, e.form, 0, 0, e.label, 0
        );
        lines.push(pad_record(&a, 'D', de, 72)?);
        lines.push(pad_record(&b, 'D', de + 1, 72)?);
    }
    for p in &p_lines {
        lines.push(p.clone());
    }
    let terminate = format!(
        "S{:>7}G{:>7}D{:>7}P{:>7}",
        1,
        lines.iter().filter(|l| l.as_bytes()[72] == b'G').count(),
        2 * w.entities.len(),
        p_lines.len()
    );
    lines.push(pad_record(&terminate, 'T', 1, 72)?);
    let text = lines.join("\n") + "\n";
    if text.len() > MAX_BYTES {
        return Err(refuse("IGES output exceeds 16 MiB"));
    }
    Ok(text)
}

pub fn export_iges_v2(model: &Model) -> Result<(String, FeatureCertificate, IgesV2Report)> {
    model.validate()?;
    if model.bodies.is_empty() || model.shells.iter().any(|s| !s.closed) {
        return Err(refuse("IGES /2 exports closed manifold bodies only"));
    }
    let mut w = Writer::new();
    let surfaces = model
        .faces
        .iter()
        .map(|face| w.emit(128, surface_values(&face.surface)))
        .collect::<Vec<_>>();
    let curves = model
        .edges
        .iter()
        .map(|edge| w.emit(126, curve_values(&edge.curve, 3)))
        .collect::<Vec<_>>();
    let mut vertex_values = vec!["502".into(), model.vertices.len().to_string()];
    for vertex in &model.vertices {
        vertex_values.extend(vertex.point.iter().map(|x| number(*x)))
    }
    let vertex_list = w.emit(502, vertex_values);
    let mut edge_values = vec!["504".into(), model.edges.len().to_string()];
    for (i, e) in model.edges.iter().enumerate() {
        edge_values.extend([
            curves[i].to_string(),
            vertex_list.to_string(),
            (e.vertices[0] + 1).to_string(),
            vertex_list.to_string(),
            (e.vertices[1] + 1).to_string(),
        ])
    }
    let edge_list = w.emit(504, edge_values);
    let mut loop_des = Vec::new();
    for l in &model.loops {
        let mut values = vec!["508".into(), l.coedges.len().to_string()];
        for c in &l.coedges {
            let pc = w.emit(126, curve_values(&c.pcurve, 2));
            values.extend([
                "0".into(),
                edge_list.to_string(),
                (c.edge + 1).to_string(),
                ((!c.reversed) as usize).to_string(),
                "1".into(),
                "0".into(),
                pc.to_string(),
            ]);
        }
        loop_des.push(w.emit(508, values));
    }
    let mut face_des = Vec::new();
    for (i, face) in model.faces.iter().enumerate() {
        let loops = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .collect::<Vec<_>>();
        let mut values = vec![
            "510".into(),
            surfaces[i].to_string(),
            loops.len().to_string(),
            "1".into(),
        ];
        values.extend(loops.iter().map(|i| loop_des[*i].to_string()));
        face_des.push(w.emit(510, values));
    }
    let mut shell_des = Vec::new();
    for shell in &model.shells {
        let mut values = vec!["514".into(), shell.faces.len().to_string()];
        for f in &shell.faces {
            values.extend([
                face_des[f.face].to_string(),
                ((!f.reversed) as usize).to_string(),
            ])
        }
        shell_des.push(w.emit(514, values));
    }
    let mut body_des = Vec::new();
    for body in &model.bodies {
        let mut values = vec![
            "186".into(),
            shell_des[body.outer_shell].to_string(),
            "1".into(),
            body.inner_shells.len().to_string(),
        ];
        for s in &body.inner_shells {
            values.extend([shell_des[*s].to_string(), "0".into()])
        }
        body_des.push(w.emit(186, values));
    }
    let groups = [
        (
            TopoKind::Vertex,
            model.1.vertices.as_slice(),
            vec![vertex_list; model.vertices.len()],
        ),
        (
            TopoKind::Edge,
            model.1.edges.as_slice(),
            vec![edge_list; model.edges.len()],
        ),
        (TopoKind::Loop, model.1.loops.as_slice(), loop_des.clone()),
        (TopoKind::Face, model.1.faces.as_slice(), face_des.clone()),
        (
            TopoKind::Shell,
            model.1.shells.as_slice(),
            shell_des.clone(),
        ),
        (TopoKind::Body, model.1.bodies.as_slice(), body_des.clone()),
    ];
    for (kind, ids, targets) in groups {
        let mut ordinal = BTreeMap::<usize, usize>::new();
        for (&id, target) in ids.iter().zip(targets) {
            let occurrence = ordinal.entry(target).or_insert(0);
            let values = &w.entities[(target - 1) / 2].values;
            let payload = format!(
                "OSCAD_TOPO/IGES2|{}|{}|{}|{}",
                kind.as_str(),
                id,
                digest_values(values),
                *occurrence
            );
            *occurrence += 1;
            let de = w.emit(
                406,
                vec![
                    "406".into(),
                    "15".into(),
                    hollerith(&payload),
                    target.to_string(),
                ],
            );
            w.entities[(de - 1) / 2].form = 15;
        }
    }
    let text = render(w)?;
    let entity_count = parse(&text)?.0.len();
    let topology_count = model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len();
    Ok((
        text,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "direct_model_export",
                "rational_126_128",
                "manifold_186_topology",
                "graph_bound_topoid_properties",
                "no_fallback",
            ],
        },
        IgesV2Report {
            identity: crate::step_interchange::StepIdentityReport {
                preserved: true,
                source: "internal-metadata",
                preserved_count: topology_count,
                created_count: 0,
                lost_count: 0,
            },
            ignored_metadata: vec![],
            entity_count,
            topology_count,
        },
    ))
}
