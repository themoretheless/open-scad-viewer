//! Automatically propose vertex-on-edge subdivisions, then sew qualified pieces.
//! This is not all-contact discovery: refused proposals stay explicit, and
//! intersections wholly inside two edges/faces are not classified by this pass.
use super::{Network, Sheet, require};
use crate::Result;
use crate::shell_thickness::contact_pair::cap::partition;
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_splits: usize,
    pub candidates: usize,
    pub pairs: usize,
    pub evaluations: usize,
    pub per_candidate_evaluations: usize,
    pub partition: partition::Limits,
    pub sewing: super::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_splits: 64,
            candidates: 256,
            pairs: 100000,
            evaluations: 32768,
            per_candidate_evaluations: 128,
            partition: Default::default(),
            sewing: Default::default(),
        }
    }
}
pub struct Insertion {
    pub sheet: usize,
    pub record: usize,
    pub target_sheet: usize,
    pub target_vertex: usize,
}
pub enum Stage {
    ParameterProposal,
    CompleteSubdivision,
}
pub struct Refusal {
    pub sheet: usize,
    pub edge: usize,
    pub target_sheet: usize,
    pub target_vertex: usize,
    pub stage: Stage,
    pub reason: String,
}
pub struct Conformed {
    network: Network,
    insertions: Vec<Insertion>,
    refusals: Vec<Refusal>,
    pairs: usize,
    evaluations_upper: usize,
    hull_exclusions: usize,
    exact_endpoints: usize,
}
impl Conformed {
    pub fn network(&self) -> &Network {
        &self.network
    }
    pub fn insertions(&self) -> &[Insertion] {
        &self.insertions
    }
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    /// Exact used count for successful proposals; failed proposals charge their
    /// entire allocation because they do not return a verified visit count.
    pub fn evaluations_upper(&self) -> usize {
        self.evaluations_upper
    }
    pub fn hull_exclusions(&self) -> usize {
        self.hull_exclusions
    }
    pub fn exact_endpoints(&self) -> usize {
        self.exact_endpoints
    }
    /// Replays all admitted original cuts and positive network proofs. This does
    /// not upgrade skipped/failed numerical proposals into negative contact proof.
    pub fn reverify(&self) -> Result<()> {
        self.network.reverify()?;
        for i in &self.insertions {
            let sheet = &self.network.sheets()[i.sheet];
            let record = &sheet.partitions()[i.record];
            let target =
                self.network.sheets()[i.target_sheet].model().vertices[i.target_vertex].point;
            require(
                record.vertex == Some(target) && record.search.is_some(),
                "Automatic insertion lost its original neighboring vertex ownership",
            )?;
        }
        Ok(())
    }
}
fn outside_hull(curve: &nurbs_core::curve::Curve, p: [f64; 3], tolerance: f64) -> bool {
    if curve.weights.iter().any(|w| *w <= 0.) {
        return false;
    }
    (0..3).any(|k| {
        let lo = curve
            .control_points
            .iter()
            .map(|q| q[k])
            .fold(f64::INFINITY, f64::min);
        let hi = curve
            .control_points
            .iter()
            .map(|q| q[k])
            .fold(f64::NEG_INFINITY, f64::max);
        // Positive rational convex hull; outward rounding is mandatory for exclusion.
        p[k] < (lo - tolerance).next_down() || p[k] > (hi + tolerance).next_up()
    })
}
/// All original neighboring vertices are tried against current edge pieces in
/// bounded order. Literal endpoints are already present; convex-hull separation
/// is a rigorous broad-phase exclusion. Every other numerical result is a
/// proposal, and complete original/UV/support/nesting proofs alone admit a cut.
/// A returned network qualifies its actual geometry, never complete discovery.
pub fn build(mut sheets: Vec<Sheet>, l: Limits) -> Result<Conformed> {
    require(
        (1..=64).contains(&sheets.len())
            && (1..=64).contains(&l.max_splits)
            && (1..=4096).contains(&l.candidates)
            && (1..=1000000).contains(&l.pairs)
            && (17..=1000000).contains(&l.evaluations)
            && (17..=4096).contains(&l.per_candidate_evaluations)
            && (1..=1000000).contains(&l.sewing.pairs)
            && (1..=1000000).contains(&l.sewing.correspondence_cells)
            && (1..=1000000).contains(&l.sewing.boundary_cells),
        "Choose bounded automatic cap-conform work allowances",
    )?;
    partition::validate_limits(l.partition)?;
    let tolerance = sheets[0].model().tolerance_mm;
    require(
        sheets.iter().all(|s| s.model().tolerance_mm == tolerance),
        "Conforming sheets must share the same boundary tolerance",
    )?;
    let count = sheets.iter().map(|s| s.model().edges.len()).sum::<usize>();
    let vertices = sheets
        .iter()
        .map(|s| s.model().vertices.len())
        .sum::<usize>();
    require(
        count <= 4096 && vertices <= 4096,
        "Automatic cap conform input exceeds edge/vertex allowance",
    )?;
    let targets = sheets
        .iter()
        .enumerate()
        .flat_map(|(s, sheet)| {
            sheet
                .model()
                .vertices
                .iter()
                .enumerate()
                .map(move |(v, p)| (s, v, p.point))
        })
        .collect::<Vec<_>>();
    let mut insertions = vec![];
    let mut refusals = vec![];
    let mut pairs = 0;
    let mut evaluations = 0;
    let mut candidates = 0;
    let mut hull_exclusions = 0;
    let mut exact_endpoints = 0;
    for owner in 0..sheets.len() {
        let mut edge = 0;
        while edge < sheets[owner].model().edges.len() {
            for &(target_sheet, target_vertex, point) in &targets {
                if target_sheet == owner {
                    continue;
                }
                require(
                    pairs < l.pairs,
                    "Shared vertex/edge pair allowance exhausted",
                )?;
                pairs += 1;
                let sheet = &mut sheets[owner];
                let e = &sheet.model.edges[edge];
                if e.vertices
                    .iter()
                    .any(|&v| sheet.model.vertices[v].point == point)
                {
                    exact_endpoints += 1;
                    continue;
                }
                if outside_hull(&e.curve, point, tolerance) {
                    hull_exclusions += 1;
                    continue;
                }
                require(
                    candidates < l.candidates,
                    "Shared automatic parameter candidate allowance exhausted",
                )?;
                candidates += 1;
                require(
                    l.evaluations - evaluations >= 17,
                    "Shared automatic parameter evaluation allowance exhausted",
                )?;
                let quota = l.per_candidate_evaluations.min(l.evaluations - evaluations);
                let proposal = partition::propose_parameter(&e.curve, point, quota);
                let (parameter, used) = match proposal {
                    Ok(p) => p,
                    Err(error) => {
                        evaluations += quota;
                        refusals.push(Refusal {
                            sheet: owner,
                            edge,
                            target_sheet,
                            target_vertex,
                            stage: Stage::ParameterProposal,
                            reason: error.to_string(),
                        });
                        continue;
                    }
                };
                evaluations += used;
                let mut roots = sheet.edge_roots.clone();
                match partition::apply(
                    &sheet.cap,
                    &sheet.model,
                    &mut roots,
                    edge,
                    parameter,
                    Some(point),
                    l.partition,
                ) {
                    Ok((model, boundary, mut record)) => {
                        require(
                            insertions.len() < l.max_splits && sheet.partitions.len() < 64,
                            "Automatic cap subdivision count exhausted",
                        )?;
                        record.search = Some(partition::Search {
                            budget: quota,
                            evaluations: used,
                        });
                        let index = sheet.partitions.len();
                        sheet.model = model;
                        sheet.boundary = boundary;
                        sheet.edge_roots = roots;
                        sheet.partitions.push(record);
                        insertions.push(Insertion {
                            sheet: owner,
                            record: index,
                            target_sheet,
                            target_vertex,
                        });
                    }
                    Err(error) => refusals.push(Refusal {
                        sheet: owner,
                        edge,
                        target_sheet,
                        target_vertex,
                        stage: Stage::CompleteSubdivision,
                        reason: error.to_string(),
                    }),
                }
            }
            edge += 1;
        }
    }
    let network = super::build(sheets, l.sewing)?;
    Ok(Conformed {
        network,
        insertions,
        refusals,
        pairs,
        evaluations_upper: evaluations,
        hull_exclusions,
        exact_endpoints,
    })
}
