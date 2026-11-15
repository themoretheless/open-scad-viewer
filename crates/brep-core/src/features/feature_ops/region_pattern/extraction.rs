//! Compact original face subcomplex and close its supported outer/hole rims.
use super::*;
/// Largest original feature face selection accepted for one region.
const MAX_SELECTED_FACES: usize = 64;
/// History stage label for the closed feature region.
const CLOSE_STAGE: &str = "feature-region-close";
/// Role shared by the closing shell/body and by a lone cap face or rim loop.
const SUPPORT_RIM_ROLE: &str = "original-support-rim";
pub(super) struct Build {
    pub model: Model,
    pub supports: Vec<usize>,
    pub support_loop: Option<usize>,
    pub caps: Vec<usize>,
    pub map: BTreeMap<TopoId, TopoId>,
    pub work: ProofWork,
}
struct CapBinding {
    face: usize,
    loops: Vec<usize>,
    rim_keys: Vec<String>,
    key: String,
    parents: Vec<Vec<usize>>,
    supports: Vec<usize>,
}
type Incidence = BTreeMap<usize, Vec<(usize, usize)>>;
/// Rim edges, outside neighbour uses and selected-face adjacency.
struct RimScan {
    rim: BTreeSet<usize>,
    neighbors: BTreeSet<(usize, usize)>,
    adjacency: BTreeMap<usize, Vec<usize>>,
}
/// Source index to compacted tool index, per entity kind.
#[derive(Default)]
struct Compaction {
    vertices: BTreeMap<usize, usize>,
    edges: BTreeMap<usize, usize>,
    loops: BTreeMap<usize, usize>,
    faces: BTreeMap<usize, usize>,
}
/// Stable signature keys shared by every derived closing id.
struct ClosureKeys {
    support: String,
    signature: String,
}
/// Growing topology of the closed tool while cap faces are appended.
struct ToolParts {
    loops: Vec<crate::Loop>,
    faces: Vec<crate::Face>,
    shell_faces: Vec<crate::FaceUse>,
}
pub(super) fn build(
    source: &Model,
    faces: &[usize],
    role: FeatureRole,
    limits: Limits,
) -> Result<Build> {
    source.validate()?;
    let selected = validated_selection(source, faces)?;
    let (shell_id, body) = owning_shell(source, &selected)?;
    let shell = &source.shells[shell_id];
    let uses = shell
        .faces
        .iter()
        .map(|f| (f.face, f.reversed))
        .collect::<BTreeMap<_, _>>();
    if selected.iter().any(|f| !uses.contains_key(f)) {
        return Err(error("Feature has an unowned face"));
    }
    let incidence = shell_incidence(source, shell);
    let scan = scan_rim(&incidence, &selected)?;
    ensure_connected(&selected, &scan.adjacency)?;
    let closure = super::closure::prepare(
        source,
        &selected,
        &uses,
        &incidence,
        &scan.rim,
        &scan.neighbors,
        limits,
    )?;
    let compaction = compact(source, &selected);
    let invert = matches!(role, FeatureRole::Subtractive);
    let (mut tool, bindings) =
        assemble_tool(source, &selected, &uses, &compaction, closure.caps, invert);
    tool.rebuild_topology_ids();
    let mut map = carry_ids(&mut tool, source, &compaction);
    let keys = ClosureKeys {
        support: sorted_face_key(source, &closure.supports),
        signature: sorted_face_key(source, selected.iter()),
    };
    derive_closure_ids(&mut tool, &bindings, &keys);
    map.insert(source.1.shells[shell_id], tool.1.shells[0]);
    map.insert(source.1.bodies[body], tool.1.bodies[0]);
    carry_attributes(&mut tool, source, &map);
    tool.refresh_change_set(&[source]);
    record_history(&mut tool, source, &bindings, shell_id, body)?;
    tool.validate()?;
    Ok(Build {
        model: tool,
        supports: closure.supports,
        support_loop: closure.source_loop,
        caps: bindings.iter().map(|b| b.face).collect(),
        map,
        work: closure.work,
    })
}
fn validated_selection(source: &Model, faces: &[usize]) -> Result<BTreeSet<usize>> {
    let selected = faces.iter().copied().collect::<BTreeSet<_>>();
    if selected.is_empty()
        || selected.len() > MAX_SELECTED_FACES
        || selected.len() != faces.len()
        || selected.iter().any(|f| *f >= source.faces.len())
    {
        return Err(error("Choose 1..64 distinct original feature faces"));
    }
    Ok(selected)
}
/// The single closed, cavity-free shell owning the selection, and its body.
fn owning_shell(source: &Model, selected: &BTreeSet<usize>) -> Result<(usize, usize)> {
    let owners = source
        .shells
        .iter()
        .enumerate()
        .filter(|(_, s)| s.faces.iter().any(|f| selected.contains(&f.face)))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let [shell_id] = owners[..] else {
        return Err(error("Feature region needs one original shell"));
    };
    let body = source
        .bodies
        .iter()
        .position(|b| b.outer_shell == shell_id && b.inner_shells.is_empty())
        .ok_or_else(|| {
            error("Feature transfer requires one cavity-free original outer shell owner")
        })?;
    if !source.shells[shell_id].closed {
        return Err(error("Original feature shell is open"));
    }
    Ok((shell_id, body))
}
fn face_wires(face: &crate::Face) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(face.outer).chain(face.holes.iter().copied())
}
fn shell_incidence(source: &Model, shell: &crate::Shell) -> Incidence {
    let mut incidence = Incidence::new();
    for f in &shell.faces {
        for wire in face_wires(&source.faces[f.face]) {
            for c in &source.loops[wire].coedges {
                incidence.entry(c.edge).or_default().push((f.face, wire));
            }
        }
    }
    incidence
}
fn scan_rim(incidence: &Incidence, selected: &BTreeSet<usize>) -> Result<RimScan> {
    let mut scan = RimScan {
        rim: BTreeSet::new(),
        neighbors: BTreeSet::new(),
        adjacency: BTreeMap::new(),
    };
    for (&edge, u) in incidence {
        let &[first, second] = &u[..] else {
            return Err(error(
                "Original feature incidence is not closed two-manifold",
            ));
        };
        let a = selected.contains(&first.0);
        let b = selected.contains(&second.0);
        if a && b {
            scan.adjacency.entry(first.0).or_default().push(second.0);
            scan.adjacency.entry(second.0).or_default().push(first.0);
        }
        if a != b {
            scan.rim.insert(edge);
            scan.neighbors.insert(if a { second } else { first });
        }
    }
    Ok(scan)
}
fn ensure_connected(
    selected: &BTreeSet<usize>,
    adjacency: &BTreeMap<usize, Vec<usize>>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut todo = selected.first().copied().into_iter().collect::<Vec<_>>();
    while let Some(f) = todo.pop() {
        if seen.insert(f) {
            todo.extend(adjacency.get(&f).into_iter().flatten());
        }
    }
    if seen != *selected {
        return Err(error("Original feature faces are disconnected"));
    }
    Ok(())
}
fn compact(source: &Model, selected: &BTreeSet<usize>) -> Compaction {
    let mut maps = Compaction::default();
    for &f in selected {
        maps.faces.insert(f, 0);
        for w in face_wires(&source.faces[f]) {
            maps.loops.insert(w, 0);
            for c in &source.loops[w].coedges {
                maps.edges.insert(c.edge, 0);
                for v in source.edges[c.edge].vertices {
                    maps.vertices.insert(v, 0);
                }
            }
        }
    }
    for map in [
        &mut maps.vertices,
        &mut maps.edges,
        &mut maps.loops,
        &mut maps.faces,
    ] {
        for (i, v) in map.values_mut().enumerate() {
            *v = i;
        }
    }
    maps
}
fn joined(ids: &[TopoId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("/")
}
fn sorted_face_key<'a>(source: &Model, faces: impl IntoIterator<Item = &'a usize>) -> String {
    let mut ids = faces
        .into_iter()
        .map(|f| source.1.faces[*f])
        .collect::<Vec<_>>();
    ids.sort();
    joined(&ids)
}
/// Copies the compacted subcomplex and appends one closing face per cap.
fn assemble_tool(
    source: &Model,
    selected: &BTreeSet<usize>,
    uses: &BTreeMap<usize, bool>,
    maps: &Compaction,
    caps: Vec<super::closure::Cap>,
    invert: bool,
) -> (Model, Vec<CapBinding>) {
    let vertices = maps
        .vertices
        .keys()
        .map(|v| source.vertices[*v].clone())
        .collect();
    let edges = maps
        .edges
        .keys()
        .map(|e| {
            let mut e = source.edges[*e].clone();
            e.vertices = e.vertices.map(|v| maps.vertices[&v]);
            e
        })
        .collect();
    let mut parts = ToolParts {
        loops: maps
            .loops
            .keys()
            .map(|w| {
                let mut w = source.loops[*w].clone();
                for c in &mut w.coedges {
                    c.edge = maps.edges[&c.edge];
                }
                w
            })
            .collect(),
        faces: maps
            .faces
            .keys()
            .map(|f| {
                let mut f = source.faces[*f].clone();
                f.outer = maps.loops[&f.outer];
                f.holes = f.holes.iter().map(|w| maps.loops[w]).collect();
                f
            })
            .collect(),
        shell_faces: selected
            .iter()
            .map(|f| crate::FaceUse {
                face: maps.faces[f],
                reversed: uses[f] ^ invert,
            })
            .collect(),
    };
    let bindings = caps
        .into_iter()
        .map(|cap| append_cap(source, maps, cap, invert, &mut parts))
        .collect();
    let tool = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops: parts.loops,
            faces: parts.faces,
            shells: vec![crate::Shell {
                faces: parts.shell_faces,
                closed: true,
            }],
            bodies: vec![crate::Body {
                outer_shell: 0,
                inner_shells: Vec::new(),
            }],
            tolerance_mm: source.tolerance_mm,
        },
        crate::TopologyIds::default(),
    );
    (tool, bindings)
}
fn append_cap(
    source: &Model,
    maps: &Compaction,
    cap: super::closure::Cap,
    invert: bool,
    parts: &mut ToolParts,
) -> CapBinding {
    let mut cap_loops = Vec::new();
    let mut rim_keys = Vec::new();
    for mut ring in cap.rings {
        let mut ids = ring
            .iter()
            .map(|c| source.1.edges[c.edge])
            .collect::<Vec<_>>();
        ids.sort();
        rim_keys.push(joined(&ids));
        for c in &mut ring {
            c.edge = maps.edges[&c.edge];
        }
        cap_loops.push(parts.loops.len());
        parts.loops.push(crate::Loop { coedges: ring });
    }
    let mut sorted = rim_keys.clone();
    sorted.sort();
    let key = sorted.join("/");
    let face = parts.faces.len();
    parts.faces.push(crate::Face {
        surface: cap.surface,
        outer: cap_loops[0],
        holes: cap_loops[1..].to_vec(),
    });
    parts.shell_faces.push(crate::FaceUse {
        face,
        reversed: cap.reversed ^ invert,
    });
    CapBinding {
        face,
        loops: cap_loops,
        rim_keys,
        key,
        parents: cap.loop_parents,
        supports: cap.supports,
    }
}
/// Reuses source ids on copied entities and returns the source-to-tool map.
fn carry_ids(tool: &mut Model, source: &Model, maps: &Compaction) -> BTreeMap<TopoId, TopoId> {
    let mut map = BTreeMap::new();
    for (&old, &new) in &maps.vertices {
        tool.1.vertices[new] = source.1.vertices[old];
        map.insert(source.1.vertices[old], tool.1.vertices[new]);
    }
    for (&old, &new) in &maps.edges {
        tool.1.edges[new] = source.1.edges[old];
        map.insert(source.1.edges[old], tool.1.edges[new]);
    }
    for (&old, &new) in &maps.loops {
        tool.1.loops[new] = source.1.loops[old];
        map.insert(source.1.loops[old], tool.1.loops[new]);
    }
    for (&old, &new) in &maps.faces {
        tool.1.faces[new] = source.1.faces[old];
        map.insert(source.1.faces[old], tool.1.faces[new]);
    }
    map
}
fn derive_closure_ids(tool: &mut Model, bindings: &[CapBinding], keys: &ClosureKeys) {
    let derive =
        |kind, role: &[u8]| TopoId::derive(kind, CLOSE_STAGE, &keys.support, &keys.signature, role);
    tool.1.shells[0] = derive(TopoKind::Shell, SUPPORT_RIM_ROLE.as_bytes());
    tool.1.bodies[0] = derive(TopoKind::Body, SUPPORT_RIM_ROLE.as_bytes());
    let total_rims = bindings.iter().map(|b| b.loops.len()).sum::<usize>();
    for binding in bindings {
        let role = if bindings.len() == 1 {
            SUPPORT_RIM_ROLE
        } else {
            &binding.key
        };
        tool.1.faces[binding.face] = derive(TopoKind::Face, role.as_bytes());
        for (wire, rim_key) in binding.loops.iter().zip(&binding.rim_keys) {
            let role = if total_rims == 1 {
                SUPPORT_RIM_ROLE
            } else {
                rim_key
            };
            tool.1.loops[*wire] = derive(TopoKind::Loop, role.as_bytes());
        }
    }
}
fn carry_attributes(tool: &mut Model, source: &Model, map: &BTreeMap<TopoId, TopoId>) {
    tool.1.local_tolerances_mm = source
        .1
        .local_tolerances_mm
        .iter()
        .filter_map(|(id, t)| map.get(id).map(|new| (*new, *t)))
        .collect();
    tool.1.sewed_edges = source
        .1
        .sewed_edges
        .intersection(&map.keys().copied().collect())
        .copied()
        .collect();
}
fn record_history(
    tool: &mut Model,
    source: &Model,
    bindings: &[CapBinding],
    shell_id: usize,
    body: usize,
) -> Result<()> {
    for binding in bindings {
        for (wire, parents) in binding.loops.iter().zip(&binding.parents) {
            naming::derived(
                tool,
                parents.iter().map(|w| source.1.loops[*w]).collect(),
                *wire,
                true,
            )?;
        }
        naming::derived(
            tool,
            binding
                .supports
                .iter()
                .map(|f| source.1.faces[*f])
                .collect(),
            binding.face,
            false,
        )?;
    }
    let shell_child = tool.1.shells[0];
    let body_child = tool.1.bodies[0];
    naming::modified(
        tool,
        source.1.shells[shell_id],
        shell_child,
        CLOSE_STAGE,
        SUPPORT_RIM_ROLE,
    )?;
    naming::modified(
        tool,
        source.1.bodies[body],
        body_child,
        CLOSE_STAGE,
        SUPPORT_RIM_ROLE,
    )
}
