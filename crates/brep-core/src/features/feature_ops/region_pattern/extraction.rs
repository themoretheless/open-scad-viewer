//! Compact original face subcomplex and close its supported outer/hole rims.
use super::*;
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
pub(super) fn build(
    source: &Model,
    faces: &[usize],
    role: FeatureRole,
    limits: Limits,
) -> Result<Build> {
    source.validate()?;
    let selected = faces.iter().copied().collect::<BTreeSet<_>>();
    if selected.is_empty()
        || selected.len() > 64
        || selected.len() != faces.len()
        || selected.iter().any(|f| *f >= source.faces.len())
    {
        return Err(error("Choose 1..64 distinct original feature faces"));
    }
    let owners = source
        .shells
        .iter()
        .enumerate()
        .filter(|(_, s)| s.faces.iter().any(|f| selected.contains(&f.face)))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if owners.len() != 1 {
        return Err(error("Feature region needs one original shell"));
    }
    let shell_id = owners[0];
    let shell = &source.shells[shell_id];
    let body = source
        .bodies
        .iter()
        .position(|b| b.outer_shell == shell_id && b.inner_shells.is_empty())
        .ok_or_else(|| {
            error("Feature transfer requires one cavity-free original outer shell owner")
        })?;
    if !shell.closed {
        return Err(error("Original feature shell is open"));
    }
    let uses = shell
        .faces
        .iter()
        .map(|f| (f.face, f.reversed))
        .collect::<BTreeMap<_, _>>();
    if selected.iter().any(|f| !uses.contains_key(f)) {
        return Err(error("Feature has an unowned face"));
    }
    let mut incidence = BTreeMap::<usize, Vec<(usize, usize)>>::new();
    for f in &shell.faces {
        let face = &source.faces[f.face];
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for c in &source.loops[wire].coedges {
                incidence.entry(c.edge).or_default().push((f.face, wire));
            }
        }
    }
    let mut rim = BTreeSet::new();
    let mut neighbors = BTreeSet::new();
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for (&edge, u) in &incidence {
        if u.len() != 2 {
            return Err(error(
                "Original feature incidence is not closed two-manifold",
            ));
        }
        let a = selected.contains(&u[0].0);
        let b = selected.contains(&u[1].0);
        if a && b {
            adjacency.entry(u[0].0).or_default().push(u[1].0);
            adjacency.entry(u[1].0).or_default().push(u[0].0);
        }
        if a != b {
            rim.insert(edge);
            neighbors.insert(if a { u[1] } else { u[0] });
        }
    }
    let mut seen = BTreeSet::new();
    let mut todo = vec![*selected.first().unwrap()];
    while let Some(f) = todo.pop() {
        if seen.insert(f) {
            todo.extend(adjacency.get(&f).into_iter().flatten());
        }
    }
    if seen != selected {
        return Err(error("Original feature faces are disconnected"));
    }
    let closure = super::closure::prepare(
        source, &selected, &uses, &incidence, &rim, &neighbors, limits,
    )?;
    let mut vm = BTreeMap::new();
    let mut em = BTreeMap::new();
    let mut lm = BTreeMap::new();
    let mut fm = BTreeMap::new();
    for &f in &selected {
        fm.insert(f, 0);
        let face = &source.faces[f];
        for &w in std::iter::once(&face.outer).chain(&face.holes) {
            lm.insert(w, 0);
            for c in &source.loops[w].coedges {
                em.insert(c.edge, 0);
                for v in source.edges[c.edge].vertices {
                    vm.insert(v, 0);
                }
            }
        }
    }
    for (i, v) in vm.values_mut().enumerate() {
        *v = i;
    }
    for (i, v) in em.values_mut().enumerate() {
        *v = i;
    }
    for (i, v) in lm.values_mut().enumerate() {
        *v = i;
    }
    for (i, v) in fm.values_mut().enumerate() {
        *v = i;
    }
    let vertices = vm.keys().map(|v| source.vertices[*v].clone()).collect();
    let edges = em
        .keys()
        .map(|e| {
            let mut e = source.edges[*e].clone();
            e.vertices = e.vertices.map(|v| vm[&v]);
            e
        })
        .collect();
    let mut loops = lm
        .keys()
        .map(|w| {
            let mut w = source.loops[*w].clone();
            for c in &mut w.coedges {
                c.edge = em[&c.edge];
            }
            w
        })
        .collect::<Vec<_>>();
    let mut copied = fm
        .keys()
        .map(|f| {
            let mut f = source.faces[*f].clone();
            f.outer = lm[&f.outer];
            f.holes = f.holes.iter().map(|w| lm[w]).collect();
            f
        })
        .collect::<Vec<_>>();
    let invert = matches!(role, FeatureRole::Subtractive);
    let mut shell_faces = selected
        .iter()
        .map(|f| crate::FaceUse {
            face: fm[f],
            reversed: uses[f] ^ invert,
        })
        .collect::<Vec<_>>();
    let mut bindings = Vec::new();
    for cap in closure.caps {
        let mut cap_loops = Vec::new();
        let mut rim_keys = Vec::new();
        for mut ring in cap.rings {
            let mut ids = ring
                .iter()
                .map(|c| source.1.edges[c.edge])
                .collect::<Vec<_>>();
            ids.sort();
            rim_keys.push(
                ids.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("/"),
            );
            for c in &mut ring {
                c.edge = em[&c.edge];
            }
            cap_loops.push(loops.len());
            loops.push(crate::Loop { coedges: ring });
        }
        let mut sorted = rim_keys.clone();
        sorted.sort();
        let key = sorted.join("/");
        let face = copied.len();
        copied.push(crate::Face {
            surface: cap.surface,
            outer: cap_loops[0],
            holes: cap_loops[1..].to_vec(),
        });
        shell_faces.push(crate::FaceUse {
            face,
            reversed: cap.reversed ^ invert,
        });
        bindings.push(CapBinding {
            face,
            loops: cap_loops,
            rim_keys,
            key,
            parents: cap.loop_parents,
            supports: cap.supports,
        });
    }
    let mut tool = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops,
            faces: copied,
            shells: vec![crate::Shell {
                faces: shell_faces,
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
    tool.rebuild_topology_ids();
    let mut map = BTreeMap::new();
    for (&old, &new) in &vm {
        tool.1.vertices[new] = source.1.vertices[old];
        map.insert(source.1.vertices[old], tool.1.vertices[new]);
    }
    for (&old, &new) in &em {
        tool.1.edges[new] = source.1.edges[old];
        map.insert(source.1.edges[old], tool.1.edges[new]);
    }
    for (&old, &new) in &lm {
        tool.1.loops[new] = source.1.loops[old];
        map.insert(source.1.loops[old], tool.1.loops[new]);
    }
    for (&old, &new) in &fm {
        tool.1.faces[new] = source.1.faces[old];
        map.insert(source.1.faces[old], tool.1.faces[new]);
    }
    let mut selected_ids = selected
        .iter()
        .map(|f| source.1.faces[*f])
        .collect::<Vec<_>>();
    selected_ids.sort();
    let signature = selected_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("/");
    let mut support_ids = closure
        .supports
        .iter()
        .map(|f| source.1.faces[*f])
        .collect::<Vec<_>>();
    support_ids.sort();
    let support_key = support_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("/");
    for (kind, child) in [
        (TopoKind::Shell, &mut tool.1.shells[0]),
        (TopoKind::Body, &mut tool.1.bodies[0]),
    ] {
        *child = TopoId::derive(
            kind,
            "feature-region-close",
            &support_key,
            &signature,
            b"original-support-rim",
        );
    }
    let total_rims = bindings.iter().map(|b| b.loops.len()).sum::<usize>();
    for binding in &bindings {
        let role = if bindings.len() == 1 {
            "original-support-rim"
        } else {
            &binding.key
        };
        tool.1.faces[binding.face] = TopoId::derive(
            TopoKind::Face,
            "feature-region-close",
            &support_key,
            &signature,
            role.as_bytes(),
        );
        for (i, &wire) in binding.loops.iter().enumerate() {
            let role = if total_rims == 1 {
                "original-support-rim"
            } else {
                &binding.rim_keys[i]
            };
            tool.1.loops[wire] = TopoId::derive(
                TopoKind::Loop,
                "feature-region-close",
                &support_key,
                &signature,
                role.as_bytes(),
            );
        }
    }
    map.insert(source.1.shells[shell_id], tool.1.shells[0]);
    map.insert(source.1.bodies[body], tool.1.bodies[0]);
    tool.1.local_tolerances_mm = source
        .1
        .local_tolerances_mm
        .iter()
        .filter(|(id, _)| map.contains_key(id))
        .map(|(id, t)| (map[id], *t))
        .collect();
    tool.1.sewed_edges = source
        .1
        .sewed_edges
        .intersection(&map.keys().copied().collect())
        .copied()
        .collect();
    tool.refresh_change_set(&[source]);
    for binding in &bindings {
        for (wire, parents) in binding.loops.iter().zip(&binding.parents) {
            naming::derived(
                &mut tool,
                parents.iter().map(|w| source.1.loops[*w]).collect(),
                *wire,
                true,
            )?;
        }
        naming::derived(
            &mut tool,
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
        &mut tool,
        source.1.shells[shell_id],
        shell_child,
        "feature-region-close",
        "original-support-rim",
    )?;
    naming::modified(
        &mut tool,
        source.1.bodies[body],
        body_child,
        "feature-region-close",
        "original-support-rim",
    )?;
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
