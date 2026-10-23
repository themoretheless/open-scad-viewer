use super::*;

impl<'a> Builder<'a> {
    fn vertex_list(&mut self, de: usize) -> Result<Vec<usize>> {
        let v = &entity(self.e, de, 502)?.values;
        let n = u(v, 1, "vertex count")?;
        if n > MAX_TOPOLOGY || v.len() != 2 + 3 * n {
            return Err(refuse("Malformed IGES vertex list"));
        }
        let mut out = Vec::new();
        for i in 0..n {
            let index = self.vertices.len();
            self.vertices.push(Vertex {
                point: [
                    f(v, 2 + 3 * i, "vertex X")? * self.scale,
                    f(v, 3 + 3 * i, "vertex Y")? * self.scale,
                    f(v, 4 + 3 * i, "vertex Z")? * self.scale,
                ],
            });
            self.vertex_de.push(de);
            out.push(index);
        }
        Ok(out)
    }
    fn build(mut self) -> Result<(Model, TopologyMapping)> {
        let mut vertex_lists = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty == 502 {
                vertex_lists.insert(de, self.vertex_list(de)?);
            }
        }
        let mut edge_lists = BTreeMap::<(usize, usize), usize>::new();
        for (&de, e) in self.e {
            if e.directory.ty != 504 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "edge count")?;
            if v.len() != 2 + 5 * n {
                return Err(refuse("Malformed IGES edge list"));
            }
            for i in 0..n {
                let at = 2 + 5 * i;
                let curve = curve126(self.e, u(v, at, "edge curve")?, 3, self.scale)?;
                let vl0 = u(v, at + 1, "start vertex list")?;
                let vi0 = u(v, at + 2, "start vertex index")?;
                let vl1 = u(v, at + 3, "end vertex list")?;
                let vi1 = u(v, at + 4, "end vertex index")?;
                let a = *vertex_lists
                    .get(&vl0)
                    .and_then(|x| x.get(vi0.checked_sub(1)?))
                    .ok_or_else(|| refuse("Invalid IGES start vertex index"))?;
                let b = *vertex_lists
                    .get(&vl1)
                    .and_then(|x| x.get(vi1.checked_sub(1)?))
                    .ok_or_else(|| refuse("Invalid IGES end vertex index"))?;
                let index = self.edges.len();
                self.edges.push(Edge {
                    degenerate: a == b,
                    vertices: [a, b],
                    curve,
                });
                self.edge_de.push(de);
                edge_lists.insert((de, i + 1), index);
            }
        }
        let mut loop_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 508 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "loop member count")?;
            let mut at = 2;
            let mut coedges = Vec::new();
            for _ in 0..n {
                if u(v, at, "loop member type")? != 0 {
                    return Err(refuse("Vertex-only IGES loop member is unsupported"));
                }
                let el = u(v, at + 1, "loop edge list")?;
                let ei = u(v, at + 2, "loop edge index")?;
                let reversed = u(v, at + 3, "loop orientation")? == 0;
                let pcount = u(v, at + 4, "loop pcurve count")?;
                if pcount != 1 {
                    return Err(refuse("Each IGES loop edge requires exactly one pcurve"));
                }
                let iso = u(v, at + 5, "loop pcurve isoparametric")?;
                if iso > 1 {
                    return Err(refuse("Invalid IGES isoparametric flag"));
                }
                let pcurve = curve126(self.e, u(v, at + 6, "loop pcurve")?, 2, 1.)?;
                let edge = *edge_lists
                    .get(&(el, ei))
                    .ok_or_else(|| refuse("Invalid IGES edge-list reference"))?;
                coedges.push(Coedge {
                    edge,
                    reversed,
                    pcurve,
                });
                at += 7;
            }
            if at != v.len() {
                return Err(refuse("Trailing IGES loop parameters"));
            }
            let index = self.loops.len();
            self.loops.push(Loop { coedges });
            self.loop_de.push(de);
            loop_map.insert(de, index);
        }
        let mut face_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 510 {
                continue;
            }
            let v = &e.values;
            let surface = surface128(self.e, u(v, 1, "face surface")?, self.scale)?;
            let n = u(v, 2, "face loop count")?;
            if n == 0 || v.len() != 4 + n {
                return Err(refuse("Malformed IGES face"));
            }
            let loops = (0..n)
                .map(|i| {
                    loop_map
                        .get(&u(v, 4 + i, "face loop")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES face loop"))
                })
                .collect::<Result<Vec<_>>>()?;
            for loop_index in &loops {
                for coedge in &self.loops[*loop_index].coedges {
                    verify_correspondence(
                        &self.edges[coedge.edge],
                        &coedge.pcurve,
                        &surface,
                        coedge.reversed,
                    )?;
                }
            }
            let index = self.faces.len();
            self.faces.push(Face {
                surface,
                outer: loops[0],
                holes: loops[1..].to_vec(),
            });
            self.face_de.push(de);
            face_map.insert(de, index);
        }
        let mut shell_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 514 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "shell face count")?;
            if n == 0 || v.len() != 2 + 2 * n {
                return Err(refuse("Malformed IGES shell"));
            }
            let faces = (0..n)
                .map(|i| {
                    Ok(FaceUse {
                        face: *face_map
                            .get(&u(v, 2 + 2 * i, "shell face")?)
                            .ok_or_else(|| refuse("Missing IGES shell face"))?,
                        reversed: u(v, 3 + 2 * i, "shell orientation")? == 0,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let index = self.shells.len();
            self.shells.push(Shell {
                faces,
                closed: true,
            });
            self.shell_de.push(de);
            shell_map.insert(de, index);
        }
        for (&de, e) in self.e {
            if e.directory.ty != 186 {
                continue;
            }
            let v = &e.values;
            let outer = *shell_map
                .get(&u(v, 1, "solid outer shell")?)
                .ok_or_else(|| refuse("Missing IGES outer shell"))?;
            let n = u(v, 3, "solid void count")?;
            if v.len() != 4 + 2 * n {
                return Err(refuse("Malformed IGES manifold solid B-rep"));
            }
            let inners = (0..n)
                .map(|i| {
                    shell_map
                        .get(&u(v, 4 + 2 * i, "void shell")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES void shell"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.bodies.push(Body {
                outer_shell: outer,
                inner_shells: inners,
            });
            self.body_de.push(de);
        }
        if self.bodies.is_empty() {
            return Err(refuse("IGES contains no manifold solid B-rep (186)"));
        }
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        if count > MAX_TOPOLOGY {
            return Err(refuse("IGES topology budget exceeds 32768 entities"));
        }
        let mut model = Model(
            brep_topology::Model {
                vertices: self.vertices,
                edges: self.edges,
                loops: self.loops,
                faces: self.faces,
                shells: self.shells,
                bodies: self.bodies,
                tolerance_mm: 1e-7,
            },
            TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model.validate()?;
        let mut map = Vec::new();
        for (kind, des) in [
            (TopoKind::Vertex, &self.vertex_de),
            (TopoKind::Edge, &self.edge_de),
            (TopoKind::Loop, &self.loop_de),
            (TopoKind::Face, &self.face_de),
            (TopoKind::Shell, &self.shell_de),
            (TopoKind::Body, &self.body_de),
        ] {
            map.extend(des.iter().enumerate().map(|(i, de)| (kind, i, *de)));
        }
        Ok((model, map))
    }
}

fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = BTreeMap::new();
    for e in entities
        .values()
        .filter(|e| e.directory.ty == 406 && e.directory.form == 15)
    {
        if e.values.len() != 4 {
            continue;
        }
        let Some(payload) = e.values[2].strip_prefix("OSCAD_TOPO/IGES2|") else {
            continue;
        };
        let p = payload.split('|').collect::<Vec<_>>();
        if p.len() != 4 {
            return Err(refuse("Malformed IGES identity property"));
        }
        let kind = TopoKind::parse(p[0]).ok_or_else(|| refuse("Unknown IGES identity kind"))?;
        let id = TopoId::parse(p[1]).map_err(refuse)?;
        let index = p[3]
            .parse::<usize>()
            .map_err(|_| refuse("Invalid IGES identity index"))?;
        let target = u(&e.values, 3, "identity target")?;
        let target_entity = entities
            .get(&target)
            .ok_or_else(|| refuse("Missing IGES identity target"))?;
        if id.kind() != kind || p[2] != topology_digest(target_entity) {
            return Err(refuse("Transplanted IGES identity metadata"));
        }
        if metadata.insert((kind, target, index), id).is_some() {
            return Err(refuse("Duplicate IGES identity property"));
        }
    }
    let count = map.len();
    if metadata.is_empty() {
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-iges",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial IGES identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    let mut ordinal = BTreeMap::<(TopoKind, usize), usize>::new();
    for &(kind, index, de) in map {
        let occurrence = ordinal.entry((kind, de)).or_insert(0);
        let id = *metadata
            .get(&(kind, de, *occurrence))
            .ok_or_else(|| refuse("IGES identity map is not topology-bijective"))?;
        *occurrence += 1;
        if !seen.insert(id) {
            return Err(refuse("Duplicate IGES TopoId"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => unreachable!(),
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => unreachable!(),
        }
    }
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    for relation in &mut model.1.lineage {
        for id in relation.parents.iter_mut().chain(&mut relation.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: count,
        created_count: 0,
        lost_count: 0,
    })
}

pub fn import_iges_v2(text: &str) -> Result<(Model, FeatureCertificate, IgesV2Report)> {
    let (entities, scale) = parse(text)?;
    const GEOMETRIC: &[usize] = &[124, 126, 128, 141, 142, 143, 186, 502, 504, 508, 510, 514];
    const METADATA: &[usize] = &[314, 402, 406];
    let mut ignored = BTreeSet::new();
    for e in entities.values() {
        if !GEOMETRIC.contains(&e.directory.ty) && !METADATA.contains(&e.directory.ty) {
            return Err(refuse(format!(
                "IGES entity {} is outside the finite direct subset",
                e.directory.ty
            )));
        }
        if METADATA.contains(&e.directory.ty) {
            ignored.insert(format!("entity-{}", e.directory.ty));
        }
        if e.directory.color != 0 {
            ignored.insert("directory-color".into());
        }
        if !e.directory.label.is_empty() {
            ignored.insert("directory-label".into());
        }
        if e.directory.transform != 0 {
            return Err(refuse(
                "Per-entity IGES transformations are typed-refused; bake one rigid placement into geometry",
            ));
        }
    }
    let builder = Builder {
        e: &entities,
        scale,
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        vertex_de: vec![],
        edge_de: vec![],
        loop_de: vec![],
        face_de: vec![],
        shell_de: vec![],
        body_de: vec![],
    };
    let (mut model, map) = builder.build()?;
    let identity = restore_identity(&entities, &map, &mut model)?;
    let topology_count = map.len();
    Ok((
        model,
        FeatureCertificate {
            capability: IGES_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "strict_80_column_sgdp_t",
                "direct_126_128_502_504_508_510_514_186",
                "shared_topology_and_pcurves",
                "multi_body_and_cavity",
                "no_aabb_constructor_or_mesh",
            ],
        },
        IgesV2Report {
            identity,
            ignored_metadata: ignored.into_iter().collect(),
            entity_count: entities.len(),
            topology_count,
        },
    ))
}

#[derive(Clone)]
pub(super) struct OutEntity {
    pub(super) ty: usize,
    pub(super) form: usize,
    pub(super) label: String,
    pub(super) color: i32,
    pub(super) values: Vec<String>,
}
pub(super) struct Writer {
    pub(super) entities: Vec<OutEntity>,
}
impl Writer {
    pub(super) fn new() -> Self {
        Self { entities: vec![] }
    }
    pub(super) fn emit(&mut self, ty: usize, values: Vec<String>) -> usize {
        self.entities.push(OutEntity {
            ty,
            form: 0,
            label: String::new(),
            color: 0,
            values,
        });
        2 * self.entities.len() - 1
    }
}
pub(super) fn number(x: f64) -> String {
    format!("{x:.17e}")
}
pub(super) fn curve_values(c: &Curve, dim: usize) -> Vec<String> {
    let n = c.control_points.len();
    let mut v = vec![
        "126".into(),
        (n - 1).to_string(),
        c.degree.to_string(),
        "0".into(),
        ((c.degree + 1 == n) as usize).to_string(),
        (c.periodic as usize).to_string(),
        "0".into(),
    ];
    v.extend(c.knots.iter().map(|x| number(*x)));
    v.extend(c.weights.iter().map(|x| number(*x)));
    for p in &c.control_points {
        v.push(number(p[0]));
        v.push(number(p[1]));
        v.push(number(if dim == 3 { p[2] } else { 0. }));
    }
    let d = c.domain();
    v.extend([
        number(d[0]),
        number(d[1]),
        "0".into(),
        "0".into(),
        "1".into(),
    ]);
    v
}
pub(super) fn surface_values(s: &Surface) -> Vec<String> {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let mut v = vec![
        "128".into(),
        (nu - 1).to_string(),
        (nv - 1).to_string(),
        s.degree_u.to_string(),
        s.degree_v.to_string(),
        "0".into(),
        "0".into(),
        (s.periodic_u as usize).to_string(),
        (s.periodic_v as usize).to_string(),
        "0".into(),
    ];
    v.extend(s.knots_u.iter().map(|x| number(*x)));
    v.extend(s.knots_v.iter().map(|x| number(*x)));
    v.extend(s.weights.iter().flatten().map(|x| number(*x)));
    for p in s.control_points.iter().flatten() {
        v.extend(p.iter().map(|x| number(*x)))
    }
    let u = [
        s.knots_u[s.degree_u],
        s.knots_u[s.knots_u.len() - s.degree_u - 1],
    ];
    let q = [
        s.knots_v[s.degree_v],
        s.knots_v[s.knots_v.len() - s.degree_v - 1],
    ];
    v.extend([number(u[0]), number(u[1]), number(q[0]), number(q[1])]);
    v
}
fn canonical(values: &[String]) -> String {
    values
        .iter()
        .map(|v| v.trim())
        .collect::<Vec<_>>()
        .join(",")
}
pub(super) fn digest_values(values: &[String]) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in canonical(values).bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub(super) fn pad_record(payload: &str, section: char, sequence: usize, width: usize) -> Result<String> {
    if payload.len() > width {
        return Err(refuse("IGES physical payload overflow"));
    }
    Ok(format!(
        "{payload:<width$}{section}{sequence:>7}",
        width = width
    ))
}
pub(super) fn hollerith(s: &str) -> String {
    format!("{}H{s}", s.len())
}
