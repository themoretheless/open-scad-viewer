//! Indexed source ancestry for exact output topology. Parent selection follows
//! actual constructor slots and certified source bindings, never a geometric
//! fingerprint search. Geometry digests only namespace newly generated IDs.
use super::*;
use crate::profile_certificate::ExactRatio;
use crate::{ChangeKind, ChangeProvenance, ChangeSet, TopoId, TopoKind, TopologyChange};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identities {
    pub vertices: Vec<TopoId>,
    pub edges: Vec<TopoId>,
    pub loops: Vec<TopoId>,
    pub faces: Vec<TopoId>,
    pub shells: Vec<TopoId>,
    pub bodies: Vec<TopoId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaceOrigin {
    RetainedSide {
        face: TopoId,
        rim: TopoId,
        source_loop: TopoId,
        coedge: [usize; 2],
        source_piece: [usize; 2],
        oriented_range: [ExactRatio; 2],
        stored_range: [ExactRatio; 2],
        reversed: bool,
        full_piece: bool,
    },
    Blend {
        selected_faces: [TopoId; 3],
        bridge: usize,
        span: usize,
    },
    Cap {
        face: TopoId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceRecord {
    pub output: usize,
    pub id: TopoId,
    pub origin: FaceOrigin,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lineage {
    pub identities: Identities,
    pub faces: Vec<FaceRecord>,
    pub change_set: ChangeSet,
}
#[derive(Clone, Copy)]
enum Piece {
    Source(usize),
    Bridge(usize),
}
fn negate(r: &ExactRatio) -> Result<ExactRatio> {
    let n = r[0]
        .parse::<num_bigint::BigInt>()
        .map_err(|_| error("Invalid source-owned lineage parameter"))?;
    Ok([(-n).to_string(), r[1].clone()])
}
fn namespace(body: &AdmittedFullRound) -> String {
    let mut hash = Sha256::new();
    let mut feed = |s: &str| {
        hash.update((s.len() as u64).to_be_bytes());
        hash.update(s.as_bytes());
    };
    feed("full-round-exact-topology/1");
    feed(&body.source().1.bodies[0].to_string());
    let source = &body.source().1;
    for (kind, ids) in [
        (TopoKind::Vertex, &source.vertices),
        (TopoKind::Edge, &source.edges),
        (TopoKind::Loop, &source.loops),
        (TopoKind::Face, &source.faces),
        (TopoKind::Shell, &source.shells),
        (TopoKind::Body, &source.bodies),
    ] {
        feed(kind.as_str());
        for id in ids {
            feed(&id.to_string());
        }
    }
    for face in body.selected_faces {
        feed(&body.source().1.faces[face].to_string());
    }
    for h in body.material.result().shell_geometry().height() {
        for q in h {
            feed(&q);
        }
    }
    for (wi, w) in body
        .material
        .result()
        .shell_geometry()
        .source_profiles()
        .iter()
        .enumerate()
    {
        feed(&format!("wire/{wi}/{}", w.pieces().len()));
        for (pi, piece) in w.pieces().iter().enumerate() {
            feed(&format!("piece/{pi}/{}", piece.len()));
            for span in piece {
                for h in span.homogeneous_controls() {
                    for q in h {
                        for s in q {
                            feed(&s);
                        }
                    }
                }
            }
        }
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
fn add_change(
    set: &mut ChangeSet,
    kind: ChangeKind,
    topo_kind: TopoKind,
    parents: Vec<TopoId>,
    children: Vec<TopoId>,
    role: String,
    scope: &str,
) {
    for id in parents.iter().chain(&children) {
        set.nodes.insert(*id, topo_kind);
    }
    set.changes.push(TopologyChange {
        kind,
        topo_kind,
        parents,
        children,
        provenance: ChangeProvenance {
            operation: "full-round-exact/1".into(),
            operand: None,
            occurrence: scope.into(),
        },
        role,
        anchor: None,
    });
}
pub fn derive(body: &AdmittedFullRound) -> Result<Lineage> {
    let source = body.source();
    source.validate()?;
    let recognized = crate::prism::recognize(source)?
        .ok_or_else(|| error("Exact lineage needs original prism ownership"))?;
    let raw = raw_profiles::bind(source, recognized.z_max, &recognized.loops)?;
    let mut middle = Vec::new();
    for (i, c) in raw[0].iter().enumerate() {
        if c.belongs_to_face(source, body.selected_faces[1])? {
            middle.push(i);
        }
    }
    if middle.len() != 1 {
        return Err(error("Exact lineage middle source census is ambiguous"));
    }
    let middle = middle[0];
    let mut plans = Vec::new();
    for (wi, w) in raw.iter().enumerate() {
        let mut plan = Vec::new();
        for ci in 0..w.len() {
            if wi == 0 && ci == middle {
                plan.extend([Piece::Bridge(0), Piece::Bridge(1)]);
            } else {
                plan.push(Piece::Source(ci));
            }
        }
        plans.push(plan);
    }
    let shell = body.material.result().shell_geometry();
    let output = body.model();
    if plans.len() != shell.source_profiles().len()
        || output.faces.len() != shell.wall_sources().len() + 2
    {
        return Err(error("Exact lineage output constructor census changed"));
    }
    for (plan, w) in plans.iter().zip(shell.source_profiles()) {
        if plan.len() != w.sources().len() {
            return Err(error("Exact lineage source piece census changed"));
        }
    }
    let scope = namespace(body);
    let ids = |kind: TopoKind, n: usize| {
        (0..n)
            .map(|i| {
                TopoId::derive(
                    kind,
                    "full-round-exact/1",
                    &scope,
                    &format!("{}/{i}", kind.as_str()),
                    scope.as_bytes(),
                )
            })
            .collect::<Vec<_>>()
    };
    let mut identities = Identities {
        vertices: ids(TopoKind::Vertex, output.vertices.len()),
        edges: ids(TopoKind::Edge, output.edges.len()),
        loops: ids(TopoKind::Loop, output.loops.len()),
        faces: ids(TopoKind::Face, output.faces.len()),
        shells: ids(TopoKind::Shell, output.shells.len()),
        bodies: ids(TopoKind::Body, output.bodies.len()),
    };
    let source_ids = source
        .1
        .vertices
        .iter()
        .chain(&source.1.edges)
        .chain(&source.1.loops)
        .chain(&source.1.faces)
        .chain(&source.1.shells)
        .chain(&source.1.bodies)
        .copied()
        .collect::<BTreeSet<_>>();
    let generated = identities
        .vertices
        .iter()
        .chain(&identities.edges)
        .chain(&identities.loops)
        .chain(&identities.faces)
        .chain(&identities.shells)
        .chain(&identities.bodies)
        .copied()
        .collect::<Vec<_>>();
    if generated.iter().any(|id| source_ids.contains(id))
        || generated.iter().copied().collect::<BTreeSet<_>>().len() != generated.len()
    {
        return Err(error(
            "Exact generated topology IDs collide with source or output identities",
        ));
    }
    let supports = body.material.source().boundary().profiles().supports();
    let mut faces = Vec::new();
    let mut groups = BTreeMap::<TopoId, Vec<usize>>::new();
    let selected = body.selected_faces.map(|f| source.1.faces[f]);
    for (face, [wi, pi, si]) in shell.wall_sources().iter().copied().enumerate() {
        let w = &shell.source_profiles()[wi];
        let restriction = &w.sources()[pi];
        let (parent, origin) = match plans[wi][pi] {
            Piece::Source(ci) => {
                let original = &raw[wi][ci];
                if w.editable()[pi]
                    || restriction.source() != original.curve()
                    || w.pieces()[pi][si] != restriction.spans()[si]
                {
                    return Err(error(
                        "Exact retained lineage does not own unchanged indexed source geometry",
                    ));
                }
                let bindings = supports
                    .bindings()
                    .iter()
                    .filter(|b| b.source_index() == [wi, ci])
                    .collect::<Vec<_>>();
                if bindings.len() != 1 || bindings[0].edge() != original.edge() {
                    return Err(error("Exact lineage original side binding is ambiguous"));
                }
                let parent = source.1.faces[bindings[0].face()];
                let range = restriction.spans()[si].parameter_range();
                let stored = if original.reversed() {
                    [negate(&range[0])?, negate(&range[1])?]
                } else {
                    range.clone()
                };
                let full = restriction.parameter_range() == original.full()?.parameter_range()
                    && supports
                        .bindings()
                        .iter()
                        .filter(|b| b.face() == bindings[0].face())
                        .count()
                        == 1;
                (
                    parent,
                    FaceOrigin::RetainedSide {
                        face: parent,
                        rim: source.1.edges[original.edge()],
                        source_loop: source.1.loops[original.coedge()[0]],
                        coedge: original.coedge(),
                        source_piece: [wi, ci],
                        oriented_range: range,
                        stored_range: stored,
                        reversed: original.reversed(),
                        full_piece: full,
                    },
                )
            }
            Piece::Bridge(bridge) => {
                if !w.editable()[pi] || restriction.source() != &body.recipe.bridges[bridge] {
                    return Err(error(
                        "Exact generated lineage is not the frozen bridge proposal",
                    ));
                }
                (
                    selected[1],
                    FaceOrigin::Blend {
                        selected_faces: selected,
                        bridge,
                        span: si,
                    },
                )
            }
        };
        groups.entry(parent).or_default().push(face);
        faces.push(FaceRecord {
            output: face,
            id: identities.faces[face],
            origin,
        });
    }
    let height = shell.height();
    for end in 0..2 {
        let parents = body
            .material
            .source()
            .boundary()
            .caps()
            .caps()
            .iter()
            .filter(|cap| {
                let z = source.faces[cap.face()].surface.control_points[0][0][2];
                num_rational::BigRational::from_float(z)
                    .is_some_and(|q| [q.numer().to_string(), q.denom().to_string()] == height[end])
            })
            .collect::<Vec<_>>();
        if parents.len() != 1 {
            return Err(error(
                "Exact cap lineage has no unique original height owner",
            ));
        }
        let parent = source.1.faces[parents[0].face()];
        let face = faces.len();
        groups.entry(parent).or_default().push(face);
        faces.push(FaceRecord {
            output: face,
            id: identities.faces[face],
            origin: FaceOrigin::Cap { face: parent },
        });
    }
    if groups.keys().copied().collect::<BTreeSet<_>>() != source.1.faces.iter().copied().collect() {
        return Err(error("Exact face ancestry omits an original source face"));
    }
    let mut change_set = source.1.change_set.clone();
    for (parent, children) in groups {
        let unchanged = children.len() == 1
            && matches!(
                &faces[children[0]].origin,
                FaceOrigin::RetainedSide {
                    full_piece: true,
                    ..
                }
            );
        let kind = if unchanged {
            ChangeKind::Persisted
        } else if children.len() == 1 {
            ChangeKind::Modified
        } else {
            ChangeKind::Split
        };
        if unchanged {
            identities.faces[children[0]] = parent;
            faces[children[0]].id = parent;
        }
        add_change(
            &mut change_set,
            kind,
            TopoKind::Face,
            vec![parent],
            children.iter().map(|i| identities.faces[*i]).collect(),
            "face ancestry".into(),
            &scope,
        );
    }
    for (kind, ids) in [
        (TopoKind::Vertex, &identities.vertices),
        (TopoKind::Edge, &identities.edges),
        (TopoKind::Loop, &identities.loops),
    ] {
        for (i, id) in ids.iter().enumerate() {
            add_change(
                &mut change_set,
                ChangeKind::Generated,
                kind,
                Vec::new(),
                vec![*id],
                format!("{}/{i}", kind.as_str()),
                &scope,
            );
        }
    }
    for (kind, parents, children) in [
        (TopoKind::Shell, &source.1.shells, &identities.shells),
        (TopoKind::Body, &source.1.bodies, &identities.bodies),
    ] {
        if parents.len() != 1 || children.len() != 1 {
            return Err(error("Exact lineage currently requires one shell/body"));
        }
        add_change(
            &mut change_set,
            ChangeKind::Modified,
            kind,
            vec![parents[0]],
            vec![children[0]],
            kind.as_str().into(),
            &scope,
        );
    }
    let all = identities
        .vertices
        .iter()
        .chain(&identities.edges)
        .chain(&identities.loops)
        .chain(&identities.faces)
        .chain(&identities.shells)
        .chain(&identities.bodies)
        .copied()
        .collect::<Vec<_>>();
    if all.iter().copied().collect::<BTreeSet<_>>().len() != all.len() {
        return Err(error("Exact output topology IDs are not unique"));
    }
    change_set.validate()?;
    Ok(Lineage {
        identities,
        faces,
        change_set,
    })
}
impl Lineage {
    pub fn reverify(&self, body: &AdmittedFullRound) -> Result<()> {
        if &derive(body)? != self {
            return Err(error("Exact source topology lineage replay changed"));
        }
        Ok(())
    }
}
