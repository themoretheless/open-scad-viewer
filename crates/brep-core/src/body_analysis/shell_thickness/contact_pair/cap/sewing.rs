//! Automatic whole-edge sewing of sealed actual cap sheets. Endpoint identity
//! only proposes pairs; full affine-clock comparison and final actual lifts admit.
pub mod conform;
pub mod crossings;
use super::{Sheet, require};
use crate::{FaceUse, Model, Result, Shell};
use nurbs_core::curve_partition_agreement::mapped;
#[derive(Clone, Copy)]
pub struct Limits {
    pub pairs: usize,
    pub correspondence_cells: usize,
    pub boundary_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            pairs: 1000000,
            correspondence_cells: 100000,
            boundary_cells: 100000,
        }
    }
}
pub struct Join {
    pub edges: [usize; 2],
    pub reversed: bool,
    pub proof: mapped::Report,
}
pub struct Network {
    sheets: Vec<Sheet>,
    model: Model,
    limits: Limits,
    joins: Vec<Join>,
    pairs: usize,
    cells: usize,
    boundary: crate::boundary_agreement::Report,
}
impl Network {
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn joins(&self) -> &[Join] {
        &self.joins
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn correspondence_cells(&self) -> usize {
        self.cells
    }
    pub fn boundary_proof(&self) -> &crate::boundary_agreement::Report {
        &self.boundary
    }
    pub fn reverify(&self) -> Result<()> {
        for sheet in &self.sheets {
            sheet.reverify()?;
        }
        let (m, joins, _, _, _) = assemble(&self.sheets, self.limits)?;
        require(
            m == self.model
                && joins.len() == self.joins.len()
                && joins
                    .iter()
                    .zip(&self.joins)
                    .all(|(a, b)| a.edges == b.edges && a.reversed == b.reversed),
            "Sewn cap network differs from original reconstruction",
        )
    }
}
fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}
fn unite(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[a.max(b)] = a.min(b);
}
pub(super) type Built = (
    Model,
    Vec<Join>,
    usize,
    usize,
    crate::boundary_agreement::Report,
);
fn assemble(sheets: &[Sheet], l: Limits) -> Result<Built> {
    let models = sheets.iter().map(Sheet::model).collect::<Vec<_>>();
    assemble_models(&models, l)
}
/// Shared private kernel for sealed original sheets and qualified face sectors.
pub(super) fn assemble_models(models: &[&Model], l: Limits) -> Result<Built> {
    require(
        (1..=64).contains(&models.len())
            && (1..=1000000).contains(&l.pairs)
            && (1..=1000000).contains(&l.correspondence_cells)
            && (1..=1000000).contains(&l.boundary_cells),
        "Choose bounded sheet count and shared sewing allowances",
    )?;
    let tol = models[0].tolerance_mm;
    for model in models {
        model.validate()?;
        require(
            model.tolerance_mm == tol && model.bodies.is_empty(),
            "Cap sector sewing needs equal tolerances and separately qualified open/shell inputs without Bodies",
        )?;
    }
    require(
        models.iter().map(|m| m.edges.len()).sum::<usize>() <= 4096
            && models.iter().map(|m| m.faces.len()).sum::<usize>() <= crate::MAX_FACES,
        "Whole-edge cap sewing exceeds input entity allowances",
    )?;
    let mut m = Model::empty(tol)?;
    let mut owners = vec![];
    for (owner, old) in models.iter().enumerate() {
        let vo = m.vertices.len();
        let eo = m.edges.len();
        let lo = m.loops.len();
        m.vertices.extend(old.vertices.iter().cloned());
        for e in &old.edges {
            let mut e = e.clone();
            e.vertices = e.vertices.map(|v| v + vo);
            m.edges.push(e);
            owners.push(owner);
        }
        for wire in &old.loops {
            let mut w = wire.clone();
            for c in &mut w.coedges {
                c.edge += eo;
            }
            m.loops.push(w);
        }
        for f in &old.faces {
            let mut f = f.clone();
            f.outer += lo;
            for h in &mut f.holes {
                *h += lo;
            }
            m.faces.push(f);
        }
    }
    let mut original_uses = vec![0; m.edges.len()];
    for wire in &m.loops {
        for c in &wire.coedges {
            original_uses[c.edge] += 1;
        }
    }
    let mut paired = vec![None; m.edges.len()];
    let mut parent = (0..m.vertices.len()).collect::<Vec<_>>();
    let mut joins = vec![];
    let mut pairs = 0;
    let mut cells = 0;
    for i in 0..m.edges.len() {
        for j in i + 1..m.edges.len() {
            if owners[i] == owners[j] {
                continue;
            }
            require(
                pairs < l.pairs,
                "Shared complete-edge pair allowance exhausted",
            )?;
            pairs += 1;
            let a = &m.edges[i];
            let b = &m.edges[j];
            let ap = a.vertices.map(|v| m.vertices[v].point);
            let bp = b.vertices.map(|v| m.vertices[v].point);
            // Exact endpoint equality is a candidate index only, never curve equality.
            let reverse = if ap == bp {
                false
            } else if ap == [bp[1], bp[0]] {
                true
            } else {
                continue;
            };
            require(
                original_uses[i] == 1 && original_uses[j] == 1,
                "Coincident internal sector edges need material-role selection before sewing",
            )?;
            require(
                ap[0] != ap[1],
                "Cyclic boundaries require explicit whole-edge decomposition",
            )?;
            require(
                paired[i].is_none() && paired[j].is_none(),
                "Ambiguous edge has more than one candidate mate",
            )?;
            require(
                cells < l.correspondence_cells,
                "Shared complete-edge correspondence allowance exhausted",
            )?;
            let d = b.curve.domain();
            let proof = mapped::verify(
                &a.curve,
                &b.curve,
                a.curve.domain(),
                if reverse { [d[1], d[0]] } else { d },
                tol,
                (l.correspondence_cells - cells).min(100000),
            )?;
            cells += proof.cells;
            require(
                proof.accepted,
                "Endpoint candidate lacks complete original curve correspondence",
            )?;
            for end in 0..2 {
                unite(
                    &mut parent,
                    a.vertices[end],
                    b.vertices[if reverse { 1 - end } else { end }],
                );
            }
            paired[i] = Some((j, reverse));
            paired[j] = Some((i, reverse));
            joins.push(Join {
                edges: [i, j],
                reversed: reverse,
                proof,
            });
        }
    }
    let mut vmap = vec![0; m.vertices.len()];
    let mut vertices = vec![];
    let mut reps = std::collections::BTreeMap::new();
    for i in 0..m.vertices.len() {
        let r = root(&parent, i);
        let next = vertices.len();
        let id = *reps.entry(r).or_insert_with(|| {
            vertices.push(m.vertices[r].clone());
            next
        });
        vmap[i] = id;
    }
    let mut emap = vec![(0, false); m.edges.len()];
    let mut edges = vec![];
    for i in 0..m.edges.len() {
        if let Some((j, reverse)) = paired[i] {
            if j < i {
                emap[i] = (emap[j].0, reverse);
                continue;
            }
        }
        let mut edge = m.edges[i].clone();
        edge.vertices = edge.vertices.map(|v| vmap[v]);
        emap[i] = (edges.len(), false);
        edges.push(edge);
    }
    m.vertices = vertices;
    m.edges = edges;
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            let (edge, reverse) = emap[c.edge];
            c.edge = edge;
            c.reversed ^= reverse;
        }
    }
    // Orient each connected face component by its actual shared-edge incidence.
    let mut incidence = vec![vec![]; m.edges.len()];
    for (face, f) in m.faces.iter().enumerate() {
        for wire in std::iter::once(&f.outer).chain(&f.holes) {
            for c in &m.loops[*wire].coedges {
                incidence[c.edge].push((face, c.reversed));
            }
        }
    }
    let mut adjacency = vec![vec![]; m.faces.len()];
    for uses in &incidence {
        require(uses.len() <= 2, "Nonmanifold sewn edge incidence")?;
        if let [(a, ar), (b, br)] = uses.as_slice() {
            let flip = ar == br;
            adjacency[*a].push((*b, flip));
            adjacency[*b].push((*a, flip));
        }
    }
    let mut orientation = vec![None; m.faces.len()];
    for start in 0..m.faces.len() {
        if orientation[start].is_some() {
            continue;
        }
        orientation[start] = Some(false);
        let mut queue = vec![start];
        let mut faces = vec![];
        while let Some(face) = queue.pop() {
            let reversed = orientation[face].unwrap();
            faces.push(FaceUse { face, reversed });
            for &(other, flip) in &adjacency[face] {
                let desired = reversed ^ flip;
                match orientation[other] {
                    Some(v) => {
                        require(v == desired, "Sewn face orientation cycle is inconsistent")?
                    }
                    None => {
                        orientation[other] = Some(desired);
                        queue.push(other);
                    }
                }
            }
        }
        faces.sort_by_key(|f| f.face);
        // Closed incidence is recorded only as topology, never as material/volume.
        let closed = faces.iter().all(|u| {
            let f = &m.faces[u.face];
            std::iter::once(&f.outer).chain(&f.holes).all(|w| {
                m.loops[*w]
                    .coedges
                    .iter()
                    .all(|c| incidence[c.edge].len() == 2)
            })
        });
        m.shells.push(Shell { faces, closed });
    }
    m.rebuild_topology_ids();
    m.validate()?;
    let boundary = crate::boundary_agreement::verify(&m, l.boundary_cells)?;
    require(
        boundary.complete,
        "Canonical sewn edges disagree with an actual cap lift",
    )?;
    Ok((m, joins, pairs, cells, boundary))
}
/// Discover complete edges with literally equal endpoints across original sheets.
/// No near-endpoint snap, partial-edge split or interior contact is inferred.
/// Full correspondence, final lifts, incidence, vertex fans and shell orientation
/// are mandatory. Even closed incidence publishes NO Body or embedding claim.
pub fn build(sheets: Vec<Sheet>, limits: Limits) -> Result<Network> {
    let (model, joins, pairs, cells, boundary) = assemble(&sheets, limits)?;
    Ok(Network {
        sheets,
        model,
        limits,
        joins,
        pairs,
        cells,
        boundary,
    })
}
