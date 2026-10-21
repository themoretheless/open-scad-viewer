//! Sufficient embedding proof for the entire straight-line UV homotopy.
use super::{
    Admission, AdmissionStatus, Connectivity, LinkStatus, Report, inspect_connectivity,
    verify_partition,
};
use crate::{Result, check, curve::Curve, curve_jets, distance_bounds::Interval as I};
use std::collections::VecDeque;

pub const ISOTOPY_DIRECTIONS: [[f64; 2]; 16] = [
    [1., 0.],
    [2., 1.],
    [1., 1.],
    [1., 2.],
    [0., 1.],
    [-1., 2.],
    [-1., 1.],
    [-2., 1.],
    [-1., 0.],
    [-2., -1.],
    [-1., -1.],
    [-1., -2.],
    [0., -1.],
    [1., -2.],
    [1., -1.],
    [2., -1.],
];
#[derive(Clone, Copy, Debug)]
pub struct IsotopyLimits {
    pub tolerance_uv: f64,
    pub max_deviation_cells: usize,
    /// Computed source/candidate jet-cell pairs, including replaced parents.
    pub max_jet_cells: usize,
    /// Generated pair requests, including requests made obsolete by refinement.
    pub max_pairs: usize,
}
#[derive(Clone, Debug)]
pub struct IsotopyLoopProof {
    pub source_closed: Option<bool>,
    pub connectivity: Option<Connectivity>,
    pub admission: Option<Admission>,
}
#[derive(Clone, Debug)]
pub struct IsotopyCellProof {
    pub loop_index: usize,
    pub piece_index: usize,
    pub domain: [f64; 2],
    /// Encloses every H(t,s), t in domain, s in [0,1], in lifted UV.
    pub hull: [[f64; 2]; 2],
    /// Bits refer to ISOTOPY_DIRECTIONS; each is positive for both derivatives.
    pub direction_mask: u16,
}
#[derive(Clone, Debug)]
pub struct IsotopyReport {
    pub lifted_isotopy_preserved: bool,
    pub periodic_isotopy_preserved: bool,
    pub loops: Vec<IsotopyLoopProof>,
    pub deviation_cells: usize,
    pub jet_cells: usize,
    pub leaf_cells: usize,
    pub pair_requests: usize,
    pub pair_checks: usize,
    /// Complete proof partition only after the lifted proof succeeds.
    pub cells: Vec<IsotopyCellProof>,
    /// Positive lower gap between the entire family enclosure and its images.
    pub periodic_clearance: Option<[f64; 2]>,
    pub reason: Option<&'static str>,
    pub periodic_reason: Option<&'static str>,
}
#[derive(Clone, Copy)]
struct Cell {
    loop_index: usize,
    piece: usize,
    spans: [usize; 2],
    domain: [f64; 2],
    depth: usize,
    hull: [I; 2],
    mask: u16,
}
struct MaskTree {
    size: usize,
    tree: Vec<u16>,
}
impl MaskTree {
    fn new(masks: &[u16]) -> Self {
        let size = masks.len().next_power_of_two();
        let mut tree = vec![u16::MAX; 2 * size];
        tree[size..size + masks.len()].copy_from_slice(masks);
        for i in (1..size).rev() {
            tree[i] = tree[2 * i] & tree[2 * i + 1];
        }
        Self { size, tree }
    }
    fn range(&self, mut a: usize, mut b: usize) -> u16 {
        a += self.size;
        b += self.size;
        let mut mask = u16::MAX;
        while a < b {
            if a % 2 != 0 {
                mask &= self.tree[a];
                a += 1;
            }
            if b % 2 != 0 {
                b -= 1;
                mask &= self.tree[b];
            }
            a /= 2;
            b /= 2;
        }
        mask
    }
}
struct Mesh {
    cells: Vec<Option<Cell>>,
    loops: Vec<Vec<usize>>,
    positions: Vec<usize>,
    trees: Vec<MaskTree>,
}
fn make_cell(
    sources: &[Curve],
    partitions: &[Report],
    li: usize,
    piece: usize,
    spans: [usize; 2],
    domain: [f64; 2],
    depth: usize,
) -> Result<Cell> {
    let source = curve_jets::enclose(&sources[li], spans[0], domain)?;
    let candidate = &partitions[li].pieces[piece];
    let retained = curve_jets::enclose(&candidate.curve, spans[1], domain)?;
    let mut hull = [I::point(0.); 2];
    for axis in 0..2 {
        let shifted = retained[0][axis].add(I::point(candidate.translation[axis]))?;
        hull[axis] = I::new(
            source[0][axis].lo.min(shifted.lo),
            source[0][axis].hi.max(shifted.hi),
        )?;
    }
    let mut mask = 0u16;
    for (bit, n) in ISOTOPY_DIRECTIONS.iter().enumerate() {
        let project =
            |d: &[I]| -> Result<I> { d[0].mul(I::point(n[0]))?.add(d[1].mul(I::point(n[1]))?) };
        if project(&source[1])?.lo > 0. && project(&retained[1])?.lo > 0. {
            mask |= 1u16 << bit;
        }
    }
    // Both jets use the same local x parameter for this domain. Its positive
    // width does not affect monotonicity; no derivative magnitude is inferred.
    Ok(Cell {
        loop_index: li,
        piece,
        spans,
        domain,
        depth,
        hull,
        mask,
    })
}
impl Mesh {
    fn refine(
        &mut self,
        id: usize,
        sources: &[Curve],
        partitions: &[Report],
        max_cells: usize,
    ) -> Result<Option<&'static str>> {
        if self.cells.len() + 2 > max_cells {
            return Ok(Some("jet-budget"));
        }
        let parent = self.cells[id].unwrap();
        let middle = parent.domain[0] * 0.5 + parent.domain[1] * 0.5;
        if middle <= parent.domain[0] || middle >= parent.domain[1] {
            return Ok(Some("jet-precision-unproven"));
        }
        let first = make_cell(
            sources,
            partitions,
            parent.loop_index,
            parent.piece,
            parent.spans,
            [parent.domain[0], middle],
            parent.depth + 1,
        )?;
        let second = make_cell(
            sources,
            partitions,
            parent.loop_index,
            parent.piece,
            parent.spans,
            [middle, parent.domain[1]],
            parent.depth + 1,
        )?;
        let start = self.cells.len();
        self.cells.push(Some(first));
        self.cells.push(Some(second));
        self.cells[id] = None;
        let pos = self.loops[parent.loop_index]
            .iter()
            .position(|x| *x == id)
            .unwrap();
        self.loops[parent.loop_index].splice(pos..pos + 1, [start, start + 1]);
        Ok(None)
    }
    fn regularize(
        &mut self,
        sources: &[Curve],
        partitions: &[Report],
        max_cells: usize,
    ) -> Result<Option<&'static str>> {
        loop {
            let invalid = self
                .loops
                .iter()
                .flatten()
                .copied()
                .find(|&i| self.cells[i].unwrap().mask == 0);
            let Some(id) = invalid else {
                return Ok(None);
            };
            if let Some(reason) = self.refine(id, sources, partitions, max_cells)? {
                return Ok(Some(reason));
            }
        }
    }
    fn rebuild(&mut self) {
        self.positions.resize(self.cells.len(), 0);
        self.trees.clear();
        for ids in &self.loops {
            let mut masks = Vec::with_capacity(ids.len());
            for (position, &id) in ids.iter().enumerate() {
                self.positions[id] = position;
                masks.push(self.cells[id].unwrap().mask);
            }
            self.trees.push(MaskTree::new(&masks));
        }
    }
    fn active(&self) -> Vec<usize> {
        self.loops.iter().flatten().copied().collect()
    }
    fn pair_proven(&self, a: usize, b: usize) -> bool {
        let x = self.cells[a].unwrap();
        let y = self.cells[b].unwrap();
        if (0..2).any(|axis| x.hull[axis].hi < y.hull[axis].lo || y.hull[axis].hi < x.hull[axis].lo)
        {
            return true;
        }
        if x.loop_index != y.loop_index {
            return false;
        }
        let i = self.positions[a].min(self.positions[b]);
        let j = self.positions[a].max(self.positions[b]);
        let n = self.loops[x.loop_index].len();
        let tree = &self.trees[x.loop_index];
        tree.range(i, j + 1) != 0 || (tree.range(j, n) & tree.range(0, i + 1)) != 0
    }
}
fn fail(mut out: IsotopyReport, mesh: &Mesh, reason: &'static str) -> IsotopyReport {
    out.jet_cells = mesh.cells.len();
    out.leaf_cells = mesh.loops.iter().map(Vec::len).sum();
    out.reason = Some(reason);
    out
}
/// Sufficient isotopy proof for H(t,s)=(1-s)source(t)+s*(piece(t)+translation).
/// Every loop stays embedded and all loops stay mutually disjoint for every s.
/// Per-cell monotonicity and full pair coverage are continuous interval proofs,
/// not endpoint/sampling tests. Success preserves lifted nesting/orientation;
/// it does not claim equality of the moved region's winding at fixed points.
/// Periodic success additionally requires the complete family enclosure to be
/// strictly narrower than both chart periods, excluding every translated copy.
pub fn verify_isotopy(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    limits: IsotopyLimits,
) -> Result<IsotopyReport> {
    check(
        !sources.is_empty()
            && sources.len() <= 64
            && sources.len() == partitions.len()
            && partitions.iter().map(|p| p.pieces.len()).sum::<usize>() <= 256
            && limits.tolerance_uv.is_finite()
            && limits.tolerance_uv > 0.
            && (1..=100000).contains(&limits.max_deviation_cells)
            && (1..=4096).contains(&limits.max_jet_cells)
            && (1..=100000).contains(&limits.max_pairs)
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1]),
        "Invalid isotopy inputs or budgets",
    )?;
    let mut out = IsotopyReport {
        lifted_isotopy_preserved: false,
        periodic_isotopy_preserved: false,
        loops: Vec::new(),
        deviation_cells: 0,
        jet_cells: 0,
        leaf_cells: 0,
        pair_requests: 0,
        pair_checks: 0,
        cells: Vec::new(),
        periodic_clearance: None,
        reason: None,
        periodic_reason: Some("lifted-isotopy-unproven"),
    };
    for (source, partition) in sources.iter().zip(partitions) {
        source.validate()?;
        check(
            !source.periodic && source.control_points[0].len() == 2,
            "Isotopy needs lifted nonperiodic UV loops",
        )?;
        let closed = crate::trim_domain::exact_loop_joins(std::slice::from_ref(source))?;
        let mut proof = IsotopyLoopProof {
            source_closed: closed,
            connectivity: None,
            admission: None,
        };
        if closed != Some(true) {
            out.loops.push(proof);
            out.reason = Some("source-closure-unproven");
            return Ok(out);
        }
        let connectivity = inspect_connectivity(partition, chart, Some([0, 0]), 256)?;
        let connected = connectivity.status == LinkStatus::Exact;
        proof.connectivity = Some(connectivity);
        if !connected {
            out.loops.push(proof);
            out.reason = Some("candidate-closure-unproven");
            return Ok(out);
        }
        if out.deviation_cells == limits.max_deviation_cells {
            out.loops.push(proof);
            out.reason = Some("deviation-budget");
            return Ok(out);
        }
        let admission = verify_partition(
            source,
            partition,
            limits.tolerance_uv,
            limits.max_deviation_cells - out.deviation_cells,
        )?;
        out.deviation_cells += admission.work;
        let admitted = admission.status == AdmissionStatus::WithinTolerance;
        proof.admission = Some(admission);
        out.loops.push(proof);
        if !admitted {
            out.reason = Some("deviation-unproven");
            return Ok(out);
        }
    }
    let mut mesh = Mesh {
        cells: Vec::new(),
        loops: vec![Vec::new(); sources.len()],
        positions: Vec::new(),
        trees: Vec::new(),
    };
    for (li, (source, partition)) in sources.iter().zip(partitions).enumerate() {
        for (piece, candidate) in partition.pieces.iter().enumerate() {
            let [a, b] = candidate.source_domain;
            let mut breaks: Vec<_> = source
                .knots
                .iter()
                .chain(&candidate.curve.knots)
                .copied()
                .filter(|t| a <= *t && *t <= b)
                .collect();
            breaks.extend([a, b]);
            breaks.sort_by(f64::total_cmp);
            breaks.dedup();
            for d in breaks.windows(2) {
                if mesh.cells.len() == limits.max_jet_cells {
                    return Ok(fail(out, &mesh, "jet-budget"));
                }
                let span = |c: &Curve| {
                    (c.degree..c.control_points.len())
                        .find(|&i| c.knots[i] <= d[0] && d[0] < c.knots[i + 1])
                        .unwrap()
                };
                let cell = make_cell(
                    sources,
                    partitions,
                    li,
                    piece,
                    [span(source), span(&candidate.curve)],
                    [d[0], d[1]],
                    0,
                )?;
                let id = mesh.cells.len();
                mesh.cells.push(Some(cell));
                mesh.loops[li].push(id);
            }
        }
    }
    if let Some(reason) = mesh.regularize(sources, partitions, limits.max_jet_cells)? {
        return Ok(fail(out, &mesh, reason));
    }
    mesh.rebuild();
    let active = mesh.active();
    let mut pending = VecDeque::new();
    for i in 0..active.len() {
        for j in i + 1..active.len() {
            if out.pair_requests == limits.max_pairs {
                return Ok(fail(out, &mesh, "pair-budget"));
            }
            pending.push_back((active[i], active[j]));
            out.pair_requests += 1;
        }
    }
    while let Some((a, b)) = pending.pop_front() {
        let (Some(x), Some(y)) = (mesh.cells[a], mesh.cells[b]) else {
            continue;
        };
        out.pair_checks += 1;
        if mesh.pair_proven(a, b) {
            continue;
        }
        let extent = |c: Cell| (c.hull[0].hi - c.hull[0].lo).max(c.hull[1].hi - c.hull[1].lo);
        let (ex, ey) = (extent(x), extent(y));
        let id = if ex > 2. * ey {
            a
        } else if ey > 2. * ex {
            b
        } else if x.depth <= y.depth {
            a
        } else {
            b
        };
        let previous = mesh.cells.len();
        if let Some(reason) = mesh.refine(id, sources, partitions, limits.max_jet_cells)? {
            return Ok(fail(out, &mesh, reason));
        }
        if let Some(reason) = mesh.regularize(sources, partitions, limits.max_jet_cells)? {
            return Ok(fail(out, &mesh, reason));
        }
        mesh.rebuild();
        let active = mesh.active();
        for &fresh in active.iter().filter(|&&id| id >= previous) {
            for &other in active.iter().filter(|&&id| id < fresh) {
                if out.pair_requests == limits.max_pairs {
                    return Ok(fail(out, &mesh, "pair-budget"));
                }
                pending.push_back((other, fresh));
                out.pair_requests += 1;
            }
        }
    }
    out.lifted_isotopy_preserved = true;
    out.jet_cells = mesh.cells.len();
    out.leaf_cells = mesh.loops.iter().map(Vec::len).sum();
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for id in mesh.active() {
        let c = mesh.cells[id].unwrap();
        for a in 0..2 {
            hull[a][0] = hull[a][0].min(c.hull[a].lo);
            hull[a][1] = hull[a][1].max(c.hull[a].hi);
        }
        out.cells.push(IsotopyCellProof {
            loop_index: c.loop_index,
            piece_index: c.piece,
            domain: c.domain,
            hull: c.hull.map(|i| [i.lo, i.hi]),
            direction_mask: c.mask,
        });
    }
    let mut clearance = [0.; 2];
    for a in 0..2 {
        let period = I::point(chart[a][1]).sub(I::point(chart[a][0]))?;
        let width = I::point(hull[a][1]).sub(I::point(hull[a][0]))?;
        clearance[a] = I::point(period.lo).sub(I::point(width.hi))?.lo;
    }
    if clearance.iter().all(|x| *x > 0.) {
        out.periodic_isotopy_preserved = true;
        out.periodic_clearance = Some(clearance);
        out.periodic_reason = None;
    } else {
        out.periodic_reason = Some("periodic-image-clearance-unproven");
    }
    Ok(out)
}
