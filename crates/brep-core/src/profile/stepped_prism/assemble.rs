use super::*;

fn assemble(parts: Vec<Model>, tolerance: f64) -> Result<Model> {
    let mut source = Model::empty(tolerance)?;
    let mut records = Vec::new();
    let mut owners = 0;
    for part in parts {
        append(&mut source, &part, &mut records, &mut owners)?;
    }
    if records.is_empty() {
        return Ok(source);
    }
    let anchor = source.vertices[0].point;
    let scale = source
        .vertices
        .iter()
        .flat_map(|v| (0..3).map(move |i| (v.point[i] - anchor[i]).abs()))
        .fold(1., f64::max);
    let epsilon = scale * 4096. * f64::EPSILON;
    if epsilon > tolerance {
        return Err(ambiguous("Cell sewing exceeds coordinate precision"));
    }
    let mut vertices = Dsu::new(source.vertices.len());
    let mut edges = Dsu::new(source.edges.len());
    let mut bodies = Dsu::new(owners);
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (i, r) in records.iter().enumerate() {
        groups.entry(r.key.clone()).or_default().push(i);
    }
    let mut cancelled = vec![false; records.len()];
    for group in groups.values().filter(|v| v.len() > 1) {
        if group.len() != 2 || records[group[0]].direction == records[group[1]].direction {
            return Err(unsupported(
                "Cell decomposition has duplicate or nonmanifold coincident faces",
            ));
        }
        let a = &records[group[0]];
        let b = &records[group[1]];
        let ae = face_edges(&source, a.usage.face);
        let be = face_edges(&source, b.usage.face);
        if ae.len() != be.len() {
            return Err(unsupported(
                "Shared cell faces have incompatible edge partitions",
            ));
        }
        let mut used = BTreeSet::new();
        for x in ae {
            let mut candidates = Vec::new();
            for &y in &be {
                if used.contains(&y) {
                    continue;
                }
                let ca = &source.edges[x].curve;
                let cb = &source.edges[y].curve;
                if same_curve(ca, cb, epsilon) {
                    candidates.push((y, false));
                } else if same_curve(ca, &cb.reverse()?, epsilon) {
                    candidates.push((y, true));
                }
            }
            if candidates.len() != 1 {
                return Err(ambiguous("Shared cell edge parameterization is unresolved"));
            }
            let (y, reverse) = candidates[0];
            used.insert(y);
            for i in 0..2 {
                vertices.join(
                    source.edges[x].vertices[i],
                    source.edges[y].vertices[if reverse { 1 - i } else { i }],
                );
            }
            edges.join(x, y);
        }
        bodies.join(a.owner, b.owner);
        cancelled[group[0]] = true;
        cancelled[group[1]] = true;
    }
    let mut model = Model::empty(tolerance)?;
    let mut vertex_ids = BTreeMap::new();
    let mut edge_ids = BTreeMap::new();
    let mut uses = Vec::new();
    let mut face_owners = Vec::new();
    for (i, r) in records.iter().enumerate().filter(|(i, _)| !cancelled[*i]) {
        let _ = i;
        let source_face = &source.faces[r.usage.face];
        let mut wires = Vec::new();
        for &wire in std::iter::once(&source_face.outer).chain(&source_face.holes) {
            let mut coedges = Vec::new();
            for c in &source.loops[wire].coedges {
                let root = edges.root(c.edge);
                let id = if let Some(&id) = edge_ids.get(&root) {
                    id
                } else {
                    let edge = &source.edges[root];
                    let mut ids = [0; 2];
                    for k in 0..2 {
                        let vertex = vertices.root(edge.vertices[k]);
                        ids[k] = *vertex_ids.entry(vertex).or_insert_with(|| {
                            let id = model.vertices.len();
                            model.vertices.push(source.vertices[vertex].clone());
                            id
                        });
                    }
                    let id = model.edges.len();
                    model.edges.push(Edge {
                        vertices: ids,
                        curve: edge.curve.clone(),
                        degenerate: false,
                    });
                    edge_ids.insert(root, id);
                    id
                };
                let forward = vertices.root(source.edges[c.edge].vertices[0])
                    == vertices.root(source.edges[root].vertices[0]);
                coedges.push(Coedge::new(
                    id,
                    c.reversed ^ !forward,
                    c.pcurve.clone(),
                ));
            }
            wires.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        let face = model.faces.len();
        model.faces.push(Face {
            surface: source_face.surface.clone(),
            outer: wires[0],
            holes: wires[1..].to_vec(),
        });
        uses.push(FaceUse {
            face,
            reversed: r.usage.reversed,
        });
        face_owners.push(bodies.root(r.owner));
    }
    let mut face_sets = Dsu::new(uses.len());
    let mut incidence = BTreeMap::<usize, Vec<usize>>::new();
    for use_ in &uses {
        for edge in face_edges(&model, use_.face) {
            incidence.entry(edge).or_default().push(use_.face);
        }
    }
    for faces in incidence.values() {
        if faces.len() != 2 {
            return Err(unsupported(
                "Stepped boundary has an unmatched or nonmanifold edge",
            ));
        }
        if face_owners[faces[0]] != face_owners[faces[1]] {
            return Err(unsupported(
                "Point or edge contact cannot merge cell bodies",
            ));
        }
        face_sets.join(faces[0], faces[1]);
    }
    let mut components = BTreeMap::<usize, Vec<FaceUse>>::new();
    for use_ in uses {
        components
            .entry(face_sets.root(use_.face))
            .or_default()
            .push(use_);
    }
    let mut shells = BTreeMap::<usize, (Option<usize>, Vec<usize>)>::new();
    for faces in components.into_values() {
        let owner = face_owners[faces[0].face];
        let (volume, error) = cap_volume(&model, &faces, anchor[2])?;
        if volume.abs() <= error || !volume.is_finite() {
            return Err(ambiguous(
                "Shell orientation has unresolved signed cap volume",
            ));
        }
        let shell = model.shells.len();
        model.shells.push(Shell {
            faces,
            closed: true,
        });
        let entry = shells.entry(owner).or_default();
        if volume > 0. {
            if entry.0.replace(shell).is_some() {
                return Err(unsupported(
                    "Connected cells produced multiple outer shells",
                ));
            }
        } else {
            entry.1.push(shell);
        }
    }
    for (outer, inner_shells) in shells.into_values() {
        model.bodies.push(Body {
            outer_shell: outer.ok_or_else(|| unsupported("Cell body has no outward shell"))?,
            inner_shells,
        });
    }
    model.rebuild_topology_ids();
    model.validate()?;
    // Also prove complete geometric boundary coverage after cancellation. This
    // rejects a pair of individually closed cells with an uncancelled interior.
    if recognize(&model)?.is_none() {
        return Err(unsupported(
            "Assembled boundary left the layered prism envelope",
        ));
    }
    Ok(model)
}

pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    let Some(la) = recognize(a)? else {
        return Ok(None);
    };
    let Some(lb) = recognize(b)? else {
        return Ok(None);
    };
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let profiles = la
        .iter()
        .chain(&lb)
        .map(|l| l.profile.clone())
        .collect::<Vec<_>>();
    let cells = partition(&profiles, tolerance)?;
    let mut levels = la
        .iter()
        .chain(&lb)
        .flat_map(|l| [l.low, l.high])
        .collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    levels.dedup();
    if levels.len() > MAX_LAYERS + 1 {
        return Err(limit());
    }
    let mut parts = Vec::new();
    let mut face_count = 0;
    for bounds in levels.windows(2) {
        for cell in &cells {
            let inside = |layers: &[Layer], offset: usize| {
                layers.iter().enumerate().any(|(i, l)| {
                    l.low <= bounds[0]
                        && l.high >= bounds[1]
                        && cell.membership & (1 << (offset + i)) != 0
                })
            };
            let ia = inside(&la, 0);
            let ib = inside(&lb, la.len());
            let selected = match operation {
                "union" => ia || ib,
                "difference" => ia && !ib,
                "intersection" => ia && ib,
                "xor" => ia != ib,
                _ => {
                    return Err(Error::new(
                        "BREP_INVALID_OPERATION",
                        "Unsupported stepped Boolean operation",
                    ));
                }
            };
            if selected {
                if bounds[1] - bounds[0] < 1e-5 {
                    return Err(unsupported(
                        "Stepped prism requires each occupied Z slab to be at least 0.00001 mm high",
                    ));
                }
                let part = prism::extrude(&cell.profile, bounds[0], bounds[1])?;
                face_count += part.faces.len();
                if face_count > 256 {
                    return Err(limit());
                }
                parts.push(part);
            }
        }
    }
    let mut result = assemble(parts, tolerance)?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(Some(result))
}
