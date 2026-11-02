//! Interior curve/curve imprint proposals on sealed cap sheets. Exact frozen-input
//! root existence is distinct from tolerance-qualified authored common vertices.
//! Point imprints neither merge face sectors nor certify an embedded solid.
use super::{Sheet, require};
use crate::shell_thickness::contact_pair::cap::partition;
use crate::{Model, Result};
use nurbs_core::{
    curve::Curve,
    intersection::{
        self as cc, CertifiedCurveIntersection, CurveCurveComponentKind, CurveCurveIntersection,
    },
};
#[derive(Clone, Copy)]
pub struct Limits {
    pub pairs: usize,
    pub potential_pairs: usize,
    pub candidates: usize,
    pub exact_cells: usize,
    pub per_pair_cells: usize,
    pub proposal_boxes: usize,
    pub per_pair_boxes: usize,
    pub max_splits: usize,
    pub proposal_fallback: bool,
    pub partition: partition::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            pairs: 100000,
            potential_pairs: 256,
            candidates: 256,
            exact_cells: 100000,
            per_pair_cells: 4096,
            proposal_boxes: 32768,
            per_pair_boxes: 512,
            max_splits: 64,
            proposal_fallback: true,
            partition: Default::default(),
        }
    }
}
/// Frozen ACTUAL input edge, including any prior qualified partitions. Exact
/// roots concern this definition, never the ideal normal-offset manifold.
pub struct Source {
    pub sheet: usize,
    pub edge: usize,
    pub curve: Curve,
}
pub struct Pair {
    pub sources: [usize; 2],
    pub exact: Option<CertifiedCurveIntersection>,
    pub proposal: Option<CurveCurveIntersection>,
    pub errors: Vec<String>,
    exact_quota: usize,
    proposal_quota: usize,
}
#[derive(Clone, Copy)]
pub enum Origin {
    Exact(usize),
    Proposal(usize),
}
#[derive(Clone, Copy)]
pub struct Mark {
    pub sheet: usize,
    pub vertex: usize,
    pub record: Option<usize>,
}
pub struct Event {
    pub pair: usize,
    pub origin: Origin,
    pub parameters: [f64; 2],
    pub point: [f64; 3],
    pub marks: [Mark; 2],
}
pub struct Refusal {
    pub pair: usize,
    pub parameters: [f64; 2],
    pub reason: String,
}
pub struct Imprinted {
    sheets: Vec<Sheet>,
    sources: Vec<Source>,
    pairs: Vec<Pair>,
    events: Vec<Event>,
    refusals: Vec<Refusal>,
    visited_pairs: usize,
    hull_exclusions: usize,
    exact_cells_upper: usize,
    proposal_boxes_upper: usize,
    tolerance: f64,
}
impl Imprinted {
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
    pub fn pairs(&self) -> &[Pair] {
        &self.pairs
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }
    pub fn visited_pairs(&self) -> usize {
        self.visited_pairs
    }
    pub fn hull_exclusions(&self) -> usize {
        self.hull_exclusions
    }
    pub fn exact_cells_upper(&self) -> usize {
        self.exact_cells_upper
    }
    pub fn proposal_boxes_upper(&self) -> usize {
        self.proposal_boxes_upper
    }
    pub fn into_sheets(self) -> Vec<Sheet> {
        self.sheets
    }
    pub fn reverify(&self) -> Result<()> {
        for s in &self.sheets {
            s.reverify()?;
        }
        // Re-run every retained exact report once, including roots whose
        // construction was refused. General results remain numerical proposals.
        for pair in &self.pairs {
            let a = &self.sources[pair.sources[0]].curve;
            let b = &self.sources[pair.sources[1]].curve;
            if let Some(old) = &pair.exact {
                let fresh = cc::intersect_curve_curve_certified_report(
                    a,
                    b,
                    self.tolerance * 0.25,
                    pair.exact_quota,
                )?;
                require(
                    fresh.complete == old.complete
                        && fresh.identical_parameter_overlap == old.identical_parameter_overlap
                        && fresh.cells == old.cells
                        && fresh.unresolved == old.unresolved
                        && fresh.crossings.len() == old.crossings.len()
                        && fresh.crossings.iter().zip(&old.crossings).all(|(a, b)| {
                            a.parameters == b.parameters
                                && a.position_bounds == b.position_bounds
                                && a.contact == b.contact
                        }),
                    "Frozen-input exact crossing report differs on replay",
                )?;
            }
            if let Some(old) = &pair.proposal {
                let fresh = cc::intersect_curve_curve_bounded_report(
                    a,
                    b,
                    Some(context(self.tolerance)?),
                    pair.proposal_quota,
                )?;
                let (a, b) = (&fresh.report, &old.report);
                require(
                    a.boxes_visited == b.boxes_visited
                        && a.components.len() == b.components.len()
                        && a.unresolved.len() == b.unresolved.len()
                        && a.components.iter().zip(&b.components).all(|(a, b)| {
                            a.kind == b.kind
                                && a.first == b.first
                                && a.second == b.second
                                && a.point == b.point
                                && a.contact == b.contact
                                && a.enclosure == b.enclosure
                        })
                        && a.unresolved.iter().zip(&b.unresolved).all(|(a, b)| {
                            a.parameter_box == b.parameter_box
                                && a.classification == b.classification
                        }),
                    "Frozen-input spatial proposal report differs on replay",
                )?;
            }
        }
        for e in &self.events {
            let pair = &self.pairs[e.pair];
            let parameters = match e.origin {
                Origin::Exact(i) => pair.exact.as_ref().unwrap().crossings[i]
                    .parameters
                    .map(|r| r[0] + (r[1] - r[0]) * 0.5),
                Origin::Proposal(i) => {
                    let c = &pair.proposal.as_ref().unwrap().report.components[i];
                    [c.first, c.second]
                }
            };
            require(
                parameters == e.parameters,
                "Interior event lost its frozen-input parameter proposal",
            )?;
            for (k, mark) in e.marks.into_iter().enumerate() {
                if let Some(i) = mark.record {
                    let record = &self.sheets[mark.sheet].partitions()[i];
                    require(
                        record.parameter == e.parameters[k] && record.vertex == Some(e.point),
                        "Interior event lost its original subdivision clock/vertex",
                    )?;
                }
                require(
                    self.sheets[mark.sheet].model().vertices[mark.vertex].point == e.point,
                    "Interior imprint lost its literal common vertex",
                )?;
            }
        }
        Ok(())
    }
}
fn context(t: f64) -> Result<cad_predicates::ToleranceContext> {
    cad_predicates::ToleranceContext::from_brep_tolerance_mm(t)
        .map_err(|e| crate::Error::new("BREP_OFFSET_CAP_REFUSED", format!("{e:?}")))
}
fn separated(a: &Curve, b: &Curve, t: f64) -> bool {
    if a.weights.iter().chain(&b.weights).any(|w| *w <= 0.) {
        return false;
    }
    (0..3).any(|k| {
        let bounds = |c: &Curve| {
            [
                c.control_points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::INFINITY, f64::min),
                c.control_points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        };
        let a = bounds(a);
        let b = bounds(b);
        (a[1] + t).next_up() < b[0] || (b[1] + t).next_up() < a[0]
    })
}
fn interior(c: &Curve, t: f64) -> bool {
    let d = c.domain();
    t.is_finite() && d[0] < t && t < d[1]
}
#[derive(Clone, Copy)]
enum Location {
    Interior(usize),
    Vertex(usize),
}
fn locate(s: &Sheet, roots: &[usize], source: usize, t: f64) -> Result<Location> {
    for (i, e) in s.model().edges.iter().enumerate() {
        if roots[i] != source {
            continue;
        }
        let d = e.curve.domain();
        if d[0] < t && t < d[1] {
            return Ok(Location::Interior(i));
        }
        if t == d[0] {
            return Ok(Location::Vertex(e.vertices[0]));
        }
        if t == d[1] {
            return Ok(Location::Vertex(e.vertices[1]));
        }
    }
    Err(crate::Error::new(
        "BREP_OFFSET_CAP_REFUSED",
        "Original crossing parameter has no current owned piece",
    ))
}
type Prepared = (
    Model,
    crate::boundary_agreement::Report,
    partition::Record,
    Vec<usize>,
);
fn prepare(s: &Sheet, edge: usize, t: f64, p: [f64; 3], l: partition::Limits) -> Result<Prepared> {
    let mut roots = s.edge_roots.clone();
    let (m, b, r) = partition::apply(&s.cap, &s.model, &mut roots, edge, t, Some(p), l)?;
    Ok((m, b, r, roots))
}
fn imprint(
    sheets: &mut [Sheet],
    piece_roots: &mut [Vec<usize>],
    sources: &[Source],
    pair: &Pair,
    pair_index: usize,
    origin: Origin,
    parameters: [f64; 2],
    proposal: [f64; 3],
    l: Limits,
    splits: &mut usize,
) -> Result<Event> {
    let owners = pair.sources.map(|s| sources[s].sheet);
    let places = [
        locate(
            &sheets[owners[0]],
            &piece_roots[owners[0]],
            pair.sources[0],
            parameters[0],
        )?,
        locate(
            &sheets[owners[1]],
            &piece_roots[owners[1]],
            pair.sources[1],
            parameters[1],
        )?,
    ];
    let mut point = proposal;
    for k in 0..2 {
        if let Location::Vertex(v) = places[k] {
            point = sheets[owners[k]].model().vertices[v].point;
        }
    }
    let mut pending: Vec<Option<Prepared>> = vec![];
    let needed = places
        .iter()
        .filter(|p| matches!(p, Location::Interior(_)))
        .count();
    require(
        *splits + needed <= l.max_splits,
        "Interior imprint subdivision allowance exhausted",
    )?;
    for k in 0..2 {
        pending.push(match places[k] {
            Location::Vertex(v) => {
                require(
                    sheets[owners[k]].model().vertices[v].point == point,
                    "Existing interior imprint vertices need separate endpoint healing",
                )?;
                None
            }
            Location::Interior(edge) => {
                require(
                    sheets[owners[k]].partitions.len() < 64,
                    "Interior imprint sheet subdivision count exhausted",
                )?;
                Some(
                    prepare(&sheets[owners[k]], edge, parameters[k], point, l.partition).map_err(
                        |e| {
                            crate::Error::new(
                                "BREP_OFFSET_CAP_REFUSED",
                                format!(
                                    "Complete imprint subdivision on sheet {} refused: {}",
                                    owners[k], e
                                ),
                            )
                        },
                    )?,
                )
            }
        });
    }
    // Both full subdivisions succeeded before either original Sheet is changed.
    let mut marks = vec![];
    for (k, prepared) in pending.into_iter().enumerate() {
        let owner = owners[k];
        let mark = if let Some((model, boundary, record, roots)) = prepared {
            let vertex = sheets[owner].model().vertices.len();
            let index = sheets[owner].partitions.len();
            sheets[owner].model = model;
            sheets[owner].boundary = boundary;
            sheets[owner].edge_roots = roots;
            sheets[owner].partitions.push(record);
            piece_roots[owner].push(pair.sources[k]);
            Mark {
                sheet: owner,
                vertex,
                record: Some(index),
            }
        } else {
            let Location::Vertex(vertex) = places[k] else {
                unreachable!()
            };
            Mark {
                sheet: owner,
                vertex,
                record: None,
            }
        };
        marks.push(mark);
    }
    *splits += needed;
    Ok(Event {
        pair: pair_index,
        origin,
        parameters,
        point,
        marks: [marks[0], marks[1]],
    })
}
/// Detect exact original planar/affine crossings where certified; unresolved
/// spatial pairs may propose roots through the general solver. Both resulting
/// complete subdivisions must independently qualify. No numerical component
/// is upgraded to exact existence, absence, face imprint or material authority.
pub fn build(mut sheets: Vec<Sheet>, l: Limits) -> Result<Imprinted> {
    require(
        (1..=64).contains(&sheets.len())
            && (1..=1000000).contains(&l.pairs)
            && (1..=4096).contains(&l.potential_pairs)
            && (1..=4096).contains(&l.candidates)
            && (1..=1000000).contains(&l.exact_cells)
            && (1..=100000).contains(&l.per_pair_cells)
            && (1..=1000000).contains(&l.proposal_boxes)
            && (1..=8192).contains(&l.per_pair_boxes)
            && (1..=128).contains(&l.max_splits),
        "Choose bounded interior crossing stage allowances",
    )?;
    partition::validate_limits(l.partition)?;
    let tolerance = sheets[0].model().tolerance_mm;
    require(
        sheets.iter().all(|s| s.model().tolerance_mm == tolerance),
        "Interior crossing sheets must share boundary tolerance",
    )?;
    require(
        sheets.iter().map(|s| s.model().edges.len()).sum::<usize>() <= 512,
        "Interior crossing input exceeds 512 original edges",
    )?;
    let mut sources = vec![];
    let mut piece_roots = vec![];
    for (owner, sheet) in sheets.iter().enumerate() {
        let mut roots = vec![];
        for (edge, e) in sheet.model().edges.iter().enumerate() {
            require(
                e.curve.control_points.len() <= 256 && e.curve.degree <= 25 && !e.curve.periodic,
                "Interior curve input needs bounded nonperiodic original definitions",
            )?;
            roots.push(sources.len());
            sources.push(Source {
                sheet: owner,
                edge,
                curve: e.curve.clone(),
            });
        }
        piece_roots.push(roots);
    }
    let mut pairs = vec![];
    let mut events = vec![];
    let mut refusals = vec![];
    let mut visited = 0;
    let mut excluded = 0;
    let mut cells = 0;
    let mut boxes = 0;
    let mut splits = 0;
    let mut attempted = 0;
    for i in 0..sources.len() {
        for j in i + 1..sources.len() {
            if sources[i].sheet == sources[j].sheet {
                continue;
            }
            require(
                visited < l.pairs,
                "Shared interior curve pair allowance exhausted",
            )?;
            visited += 1;
            let a = &sources[i].curve;
            let b = &sources[j].curve;
            if separated(a, b, tolerance) {
                excluded += 1;
                continue;
            }
            require(
                pairs.len() < l.potential_pairs && cells < l.exact_cells,
                "Shared original crossing proof allowance exhausted",
            )?;
            let quota = l.per_pair_cells.min(l.exact_cells - cells);
            let mut errors = vec![];
            let exact =
                match cc::intersect_curve_curve_certified_report(a, b, tolerance * 0.25, quota) {
                    Ok(r) => {
                        cells += r.cells;
                        Some(r)
                    }
                    Err(e) => {
                        cells += quota;
                        errors.push(e.to_string());
                        None
                    }
                };
            let needs_proposal = l.proposal_fallback
                && exact
                    .as_ref()
                    .is_none_or(|r| !r.complete && r.crossings.is_empty());
            let mut proposal_quota = 0;
            let proposal = if needs_proposal {
                require(
                    boxes < l.proposal_boxes,
                    "Shared spatial crossing proposal allowance exhausted",
                )?;
                proposal_quota = l.per_pair_boxes.min(l.proposal_boxes - boxes);
                match cc::intersect_curve_curve_bounded_report(
                    a,
                    b,
                    Some(context(tolerance)?),
                    proposal_quota,
                ) {
                    Ok(r) => {
                        boxes += r.report.boxes_visited;
                        Some(r)
                    }
                    Err(e) => {
                        boxes += proposal_quota;
                        errors.push(e.to_string());
                        None
                    }
                }
            } else {
                None
            };
            let pair = Pair {
                sources: [i, j],
                exact,
                proposal,
                errors,
                exact_quota: quota,
                proposal_quota,
            };
            let mut candidates = vec![];
            if let Some(r) = &pair.exact {
                for (k, c) in r.crossings.iter().enumerate() {
                    let t = c.parameters.map(|d| d[0] + (d[1] - d[0]) * 0.5);
                    if interior(a, t[0]) && interior(b, t[1]) {
                        candidates.push((
                            Origin::Exact(k),
                            t,
                            c.position_bounds.map(|d| d[0] + (d[1] - d[0]) * 0.5),
                        ));
                    }
                }
            }
            if let Some(r) = &pair.proposal {
                for (k, c) in r.report.components.iter().enumerate() {
                    if c.kind == CurveCurveComponentKind::Point
                        && interior(a, c.first)
                        && interior(b, c.second)
                    {
                        candidates.push((Origin::Proposal(k), [c.first, c.second], c.point));
                    }
                }
            }
            for (origin, t, p) in candidates {
                require(
                    attempted < l.candidates,
                    "Interior crossing candidate allowance exhausted",
                )?;
                attempted += 1;
                match imprint(
                    &mut sheets,
                    &mut piece_roots,
                    &sources,
                    &pair,
                    pairs.len(),
                    origin,
                    t,
                    p,
                    l,
                    &mut splits,
                ) {
                    Ok(e) => events.push(e),
                    Err(e) => refusals.push(Refusal {
                        pair: pairs.len(),
                        parameters: t,
                        reason: e.to_string(),
                    }),
                }
            }
            pairs.push(pair);
        }
    }
    Ok(Imprinted {
        sheets,
        sources,
        pairs,
        events,
        refusals,
        visited_pairs: visited,
        hull_exclusions: excluded,
        exact_cells_upper: cells,
        proposal_boxes_upper: boxes,
        tolerance,
    })
}
