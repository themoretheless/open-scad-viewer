//! Bounded enumeration of different face pairs. Adjacent faces are not skipped:
//! sharing topology does not prove the absence of extra interior intersections.
#[path = "boundary_stitched_contact.rs"]
mod boundary_stitched_contact;
#[path = "source_volume_contact.rs"]
mod source_volume_contact;
#[path = "same_support_contact.rs"]
mod same_support_contact;
#[path = "projected_trim_contact.rs"]
mod projected_trim_contact;
#[path = "projected_trim_network.rs"]
mod projected_trim_network;
use crate::face_contact_groups::Certificate as DisjointGroup;
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::surface_contact_search::{self, Report as Search};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum individually classified pairs. Sweep range certificates use
    /// the shared geometry-cell budget instead and are reported separately.
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
    pub cells_per_pair: usize,
    pub domain_cells_per_pair: usize,
}
#[derive(Clone, Debug)]
pub enum SharedBoundary {
    SweepCap {
        cap: usize,
        wall: usize,
        edge: usize,
    },
    ExactHull(crate::boundary_hull_contact::Certificate),
    PlanarFace(crate::shared_boundary::Certificate),
    OppositeSides(crate::shared_boundary::OppositeSidesCertificate),
    BilinearGraph(crate::shared_boundary::bilinear_graph::Certificate),
    ClosedGraph(crate::shared_boundary::closed_graph::Certificate),
    JoinedSourceCharts(boundary_stitched_contact::Certificate),
    SourceVolumeBoundary(source_volume_contact::Certificate),
    OwnedVertex(crate::trimmed_vertex_contact::Certificate),
    SameSupportPartition(same_support_contact::Certificate),
    ProjectedTrimPartition(projected_trim_contact::Certificate),
    ProjectedTrimNetwork(projected_trim_network::Certificate),
}
#[derive(Clone, Debug)]
pub struct Pair {
    pub faces: [usize; 2],
    pub result: Option<Search>,
    pub boundary: Option<SharedBoundary>,
    /// Disjointness was proven exactly by control-hull separation, so no
    /// trimmed search report exists for this pair.
    pub hull_disjoint: bool,
    /// Exact control-hull separation work spent on this pair. Attributed to
    /// the pair so per-pair work adds up to the report totals.
    pub hull_cells: usize,
    /// Charged shared-boundary admission work, including exact vertex contacts.
    pub boundary_cells: usize,
    pub source_chart_cells: usize,
    pub volume_separation: Option<source_volume_contact::Certificate>,
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub pairs: Vec<Pair>,
    pub disjoint_groups: Vec<DisjointGroup>,
    pub grouped_pairs: usize,
    /// Includes preparation and unsuccessful group attempts.
    pub group_cells: usize,
    pub total_pairs: usize,
    /// First unvisited pair in lexicographic order; the entire suffix is pending.
    pub next_pair: Option<[usize; 2]>,
    pub cells: usize,
    pub domain_cells: usize,
    /// All distinct face pairs were proven disjoint. This says nothing about
    /// self-intersection inside one face or geometric validity of a volume.
    pub all_pairs_disjoint: bool,
    pub all_pairs_classified: bool,
}
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    inspect_with_hulls(model, tolerance_uv, limits, &[])
}
/// Scope is exactly the pairs incident to `face`; this cannot certify the
/// remaining body pairs or self-intersections within any individual face.
pub struct IncidentReport {
    pub face: usize,
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub next_pair: Option<[usize; 2]>,
    pub cells: usize,
    pub domain_cells: usize,
    pub all_incident_pairs_classified: bool,
    pub all_incident_pairs_disjoint: bool,
    /// Additional exact vertex work; legacy pair-search work is separate.
    pub vertex_exact_work: u64,
}
pub fn inspect_incident(
    model: &Model,
    face: usize,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<IncidentReport> {
    if face >= model.faces.len() {
        return Err(Error::new(
            "BREP_CONTACT_FACE",
            "Incident face index is out of range",
        ));
    }
    let mut inspector = Inspector::new(
        std::sync::Arc::new(model.clone()),
        tolerance_uv,
        limits,
        vec![],
        vec![],
        crate::face_contacts::ContactModes {
            sweep_hulls: false,
            joint_exact_domain: false,
            prepared: None,
        },
    )?;
    inspector.incident = Some(face);
    inspector.report.total_pairs = model.faces.len() - 1;
    while !inspector.finished() {
        inspector.advance(64)?;
    }
    let r = inspector.finish()?;
    Ok(IncidentReport {
        face,
        pairs: r.pairs,
        total_pairs: r.total_pairs,
        next_pair: r.next_pair,
        cells: r.cells,
        domain_cells: r.domain_cells,
        all_incident_pairs_classified: r.all_pairs_classified,
        all_incident_pairs_disjoint: r.all_pairs_disjoint,
        vertex_exact_work: 0,
    })
}
/// Extend the same scoped search with freshly proven exact owned-vertex
/// contacts. A metric locality bound never promotes the pair's exact status.
pub fn inspect_incident_with_vertices(
    model: &Model,
    face: usize,
    tolerance_uv: f64,
    limits: Limits,
    max_exact: u64,
) -> Result<IncidentReport> {
    if max_exact > 1000000 {
        return Err(Error::new(
            "BREP_CONTACT_BUDGET",
            "Exact vertex work exceeds its bounded allowance",
        ));
    }
    let mut report = inspect_incident(model, face, tolerance_uv, limits)?;
    for pair in &mut report.pairs {
        if pair.reason != "pair-unresolved" {
            continue;
        }
        if report.cells == limits.cells
            || report.domain_cells == limits.domain_cells
            || report.vertex_exact_work == max_exact
        {
            break;
        }
        let allowance = (limits.cells - report.cells)
            .min(limits.domain_cells - report.domain_cells)
            .min(100000);
        let attempt = crate::trimmed_vertex_contact::inspect(
            model,
            pair.faces,
            allowance,
            max_exact - report.vertex_exact_work,
        )?;
        report.cells += attempt.cells;
        report.vertex_exact_work += attempt.exact_work;
        // The vertex primitive has one combined geometry/trim ledger. Charge
        // all its work to the UV allowance too, rather than omit trim work.
        report.domain_cells += attempt.cells;
        pair.boundary_cells += attempt.cells;
        if let Some(c) = attempt.certificate.filter(|c| c.exact_vertex_contact) {
            pair.boundary = Some(SharedBoundary::OwnedVertex(c));
            pair.reason = "shared-vertex";
        }
    }
    report.all_incident_pairs_classified = report.next_pair.is_none()
        && report.pairs.len() == report.total_pairs
        && report.pairs.iter().all(|p| {
            p.boundary.is_some()
                || p.hull_disjoint
                || p.result.as_ref().is_some_and(|r| r.absence_proven)
        });
    // Exact vertex contact is a real contact, never a disjointness proof.
    Ok(report)
}
/// Hull certificates are admitted only by the exact boundary embedding audit.
pub(crate) fn inspect_with_hulls(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
) -> Result<Report> {
    inspect_with_certificates(model, tolerance_uv, limits, hulls, &[], false)
}
/// Cap tuples originate only from native recomputation in boundary embedding.
pub(crate) fn inspect_with_certificates(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    joint_exact_domain: bool,
) -> Result<Report> {
    inspect_impl(
        model,
        tolerance_uv,
        limits,
        hulls,
        caps,
        crate::face_contacts::ContactModes {
            sweep_hulls: !caps.is_empty(),
            joint_exact_domain,
            prepared: None,
        },
    )
}
/// Sweep-only exact hull separation, also for closed bodies without caps.
/// Every plane is proved on actual positive rational control coefficients.
#[allow(dead_code)]
pub(crate) fn inspect_with_sweep_certificates(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    joint_exact_domain: bool,
) -> Result<Report> {
    inspect_impl(
        model,
        tolerance_uv,
        limits,
        hulls,
        caps,
        crate::face_contacts::ContactModes {
            sweep_hulls: true,
            joint_exact_domain,
            prepared: None,
        },
    )
}
/// Prepared only by the same immutable-model boundary audit. The charged
/// count includes preparation and any earlier cap separation queries.
#[allow(dead_code)]
pub(crate) fn inspect_with_prepared_sweep_certificates(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    joint_exact_domain: bool,
    prepared: (Option<crate::face_contact_groups::Cover>, usize),
) -> Result<Report> {
    inspect_impl(
        model,
        tolerance_uv,
        limits,
        hulls,
        caps,
        crate::face_contacts::ContactModes {
            sweep_hulls: true,
            joint_exact_domain,
            prepared: Some(prepared),
        },
    )
}
/// Inspection modes: whether sweep hulls and the joint exact domain are used,
/// and an optional precomputed face-group cover with its work.
pub(crate) struct ContactModes {
    pub sweep_hulls: bool,
    pub joint_exact_domain: bool,
    pub prepared: Option<(Option<crate::face_contact_groups::Cover>, usize)>,
}
fn inspect_impl(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    hulls: &[crate::boundary_hull_contact::Certificate],
    caps: &[[usize; 3]],
    modes: ContactModes,
) -> Result<Report> {
    let ContactModes { sweep_hulls, joint_exact_domain, prepared } = modes;
    let mut inspector = Inspector::new(
        std::sync::Arc::new(model.clone()),
        tolerance_uv,
        limits,
        hulls.to_vec(),
        caps.to_vec(),
        crate::face_contacts::ContactModes {
            sweep_hulls,
            joint_exact_domain,
            prepared,
        },
    )?;
    while !inspector.finished() { inspector.advance(64)?; }
    inspector.finish()
}

/// Owns the exact model and cumulative work ledger throughout pair traversal.
/// Intermediate steps expose work counts, never a partial positive certificate.
pub(crate) struct Inspector {
    model: std::sync::Arc<Model>,
    limits: Limits,
    hulls: Vec<crate::boundary_hull_contact::Certificate>,
    caps: Vec<[usize; 3]>,
    sweep_hulls: bool,
    joint_exact_domain: bool,
    domains: Vec<FaceDomain>,
    cover: Option<crate::face_contact_groups::Cover>,
    report: Report,
    cursor: usize,
    pending: Vec<usize>,
    scalar_next: usize,
    done: bool,
    failed: bool,
    incident: Option<usize>,
}
impl Inspector {
    pub(crate) fn new(
        model: std::sync::Arc<Model>,
        tolerance_uv: f64,
        limits: Limits,
        hulls: Vec<crate::boundary_hull_contact::Certificate>,
        caps: Vec<[usize; 3]>,
        modes: ContactModes,
    ) -> Result<Self> {
        let ContactModes { sweep_hulls, joint_exact_domain, prepared } = modes;
        model.validate_boundary_diagnostic_inputs()?;
        if !(1..=100_000).contains(&limits.pairs)
            || !(1..=1_000_000).contains(&limits.cells)
            || !(1..=8_000_000).contains(&limits.domain_cells)
            || !(1..=100_000).contains(&limits.cells_per_pair)
            || !(1..=1_000_000).contains(&limits.domain_cells_per_pair)
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
        {
            return Err(Error::new(
                "BREP_CONTACT_BUDGET",
                "Use positive bounded pair, geometry and UV budgets and finite positive UV tolerance",
            ));
        }
        // Prepare each chart once, rather than validate/copy the model per pair.
        let domains = (0..model.faces.len())
            .map(|f| FaceDomain::new(&model, f, tolerance_uv))
            .collect::<Result<Vec<_>>>()?;
        let n = model.faces.len();
        let total_pairs = n
            .checked_mul(n.saturating_sub(1))
            .ok_or_else(|| Error::new("BREP_CONTACT_BUDGET", "Too many face pairs"))?
            / 2;
        let mut out = Report {
            pairs: Vec::new(),
            disjoint_groups: Vec::new(),
            grouped_pairs: 0,
            group_cells: 0,
            total_pairs,
            next_pair: None,
            cells: 0,
            domain_cells: 0,
            all_pairs_disjoint: true,
            all_pairs_classified: true,
        };
        let cover = if sweep_hulls && n > 1 {
            let (cover, cells) = prepared.unwrap_or_else(|| {
                crate::face_contact_groups::Cover::prepare(&model, limits.cells)
            });
            out.cells += cells;
            out.group_cells += cells;
            cover
        } else {
            None
        };
        let pending = cover.as_ref().map(|c| vec![c.root]).unwrap_or_default();
        Ok(Self {
            model,
            limits,
            hulls,
            caps,
            sweep_hulls,
            joint_exact_domain,
            domains,
            cover,
            report: out,
            cursor: 0,
            pending,
            scalar_next: 1,
            done: n < 2,
            failed: false,
            incident: None,
        })
    }
    pub(crate) fn finished(&self) -> bool {
        self.done
    }
    /// One unit visits a cover node or attempts the next scalar pair.
    pub(crate) fn advance(&mut self, max_units: usize) -> Result<usize> {
        if !(1..=100_000).contains(&max_units) {
            return Err(Error::new(
                "BREP_CONTACT_STEP",
                "Use a positive bounded traversal step",
            ));
        }
        if self.failed {
            return Err(Error::new(
                "BREP_CONTACT_FAILED",
                "Discard a failed traversal before retrying",
            ));
        }
        let result = self.advance_inner(max_units);
        self.failed = result.is_err();
        result
    }
    fn advance_inner(&mut self, max_units: usize) -> Result<usize> {
        if self.done {
            return Ok(0);
        }
        let model = std::sync::Arc::clone(&self.model);
        let source_volume = self
            .joint_exact_domain
            .then(|| source_volume_contact::Prepared::new(&model));
        let n = model.faces.len();
        let mut used = 0;
        while self.cursor < n && used < max_units {
            used += 1;
            let a = self.cursor;
            let b = match self.next_partner(a, n)? {
                Visit::Skip => continue,
                Visit::Stop(pair) => {
                    self.stop_at(pair);
                    return Ok(used);
                }
                Visit::Pair(b) => b,
            };
            let out = &self.report;
            if out.pairs.len() == self.limits.pairs
                || out.cells == self.limits.cells
                || out.domain_cells == self.limits.domain_cells
            {
                self.stop_at([a, b]);
                return Ok(used);
            }
            let context = PairContext {
                model: &model,
                limits: self.limits,
                hulls: &self.hulls,
                caps: &self.caps,
                sweep_hulls: self.sweep_hulls,
                joint_exact_domain: self.joint_exact_domain,
                domains: &self.domains,
                cover: self.cover.as_ref(),
                source_volume: source_volume.as_ref(),
            };
            let pair = context.classify([a, b], &mut self.report)?;
            self.report.pairs.push(pair);
        }
        if self.cursor == n {
            debug_assert_eq!(
                self.report.pairs.len() + self.report.grouped_pairs,
                self.report.total_pairs
            );
            self.done = true;
        }
        Ok(used)
    }
    /// Records the first pending pair; the whole lexicographic suffix stays unclassified.
    fn stop_at(&mut self, pair: [usize; 2]) {
        self.report.next_pair = Some(pair);
        self.report.all_pairs_disjoint = false;
        self.report.all_pairs_classified = false;
        self.done = true;
    }
    fn next_partner(&mut self, a: usize, n: usize) -> Result<Visit> {
        if self.cover.is_some() {
            self.next_cover_partner(a)
        } else {
            Ok(self.next_scalar_partner(a, n))
        }
    }
    /// Visits one cover node: a certified disjoint group, a split, or a leaf partner.
    fn next_cover_partner(&mut self, a: usize) -> Result<Visit> {
        let Some(cover) = self.cover.as_ref() else {
            return Err(cover_error());
        };
        let Some(index) = self.pending.pop() else {
            self.cursor += 1;
            self.scalar_next = self.cursor + 1;
            self.pending.push(cover.root);
            return Ok(Visit::Skip);
        };
        let node = cover.nodes.get(index).ok_or_else(cover_error)?;
        if node.range[1] <= a + 1 {
            return Ok(Visit::Skip);
        }
        if node.range[0] > a && node.children.is_some() {
            let out = &mut self.report;
            if out.cells == self.limits.cells {
                return Ok(Visit::Stop([a, node.range[0]]));
            }
            out.cells += 1;
            out.group_cells += 1;
            if let Some(certificate) = cover.certify(a, index) {
                out.grouped_pairs += certificate.range[1] - certificate.range[0];
                out.disjoint_groups.push(certificate);
                return Ok(Visit::Skip);
            }
        }
        if let Some([left, right]) = node.children {
            self.pending.push(right);
            self.pending.push(left);
            return Ok(Visit::Skip);
        }
        Ok(Visit::Pair(node.range[0]))
    }
    /// Lexicographic pair order, optionally restricted to pairs incident to one face.
    fn next_scalar_partner(&mut self, a: usize, n: usize) -> Visit {
        match self.incident {
            Some(face) if a < face => {
                self.cursor += 1;
                self.scalar_next = self.cursor + 1;
                Visit::Pair(face)
            }
            Some(face) if a > face => {
                self.cursor = n;
                Visit::Skip
            }
            Some(_) if self.scalar_next == n => {
                self.cursor = n;
                Visit::Skip
            }
            None if self.scalar_next == n => {
                self.cursor += 1;
                self.scalar_next = self.cursor + 1;
                Visit::Skip
            }
            Some(_) | None => {
                let b = self.scalar_next;
                self.scalar_next += 1;
                Visit::Pair(b)
            }
        }
    }
    pub(crate) fn finish(self) -> Result<Report> {
        if self.failed { return Err(Error::new("BREP_CONTACT_FAILED", "Discard a failed traversal")); }
        if !self.done { return Err(Error::new("BREP_CONTACT_PENDING", "Complete pair traversal before requesting its report")); }
        Ok(self.report)
    }
}

/// Bilinear graph boundary work admitted per pair by the joint embedding audit.
const BILINEAR_GRAPH_CELLS: u64 = 131_072;
/// Upper bound on one opposite-sides, projected-trim or same-support attempt.
const CHART_ATTEMPT_CELLS: usize = 100_000;
/// Upper bound on the restricted opposite-rows attempt.
const RESTRICTED_ROWS_CELLS: usize = 256;
/// Upper bound on exact control-hull separation work per pair.
const HULL_SEPARATION_CELLS: usize = 64;

/// One traversal unit: skip (no pair attempted), budget stop, or a pair partner.
enum Visit {
    Skip,
    Stop([usize; 2]),
    Pair(usize),
}

fn cover_error() -> Error {
    Error::new(
        "BREP_CONTACT_COVER",
        "Use a cover whose nodes belong to the traversal",
    )
}

/// Read-only inputs shared by every certification stage of one pair.
struct PairContext<'a> {
    model: &'a Model,
    limits: Limits,
    hulls: &'a [crate::boundary_hull_contact::Certificate],
    caps: &'a [[usize; 3]],
    sweep_hulls: bool,
    joint_exact_domain: bool,
    domains: &'a [FaceDomain],
    cover: Option<&'a crate::face_contact_groups::Cover>,
    source_volume: Option<&'a source_volume_contact::Prepared<'a>>,
}

/// Mutable per-pair ledger built up by the certification stages.
struct PairWork {
    faces: [usize; 2],
    boundary: Option<SharedBoundary>,
    boundary_cells: usize,
    source_chart_cells: usize,
    volume_separation: Option<source_volume_contact::Certificate>,
    hull_disjoint: bool,
    hull_cells: usize,
    periodic: bool,
    pair_allowance: usize,
}

impl PairWork {
    fn unresolved(&self) -> bool {
        self.boundary.is_none() && !self.hull_disjoint && self.volume_separation.is_none()
    }
    fn disjoint(&self) -> bool {
        self.hull_disjoint || self.volume_separation.is_some()
    }
    fn spent(&self) -> usize {
        self.boundary_cells + self.source_chart_cells + self.hull_cells
    }
}

impl PairContext<'_> {
    fn classify(&self, faces: [usize; 2], out: &mut Report) -> Result<Pair> {
        let [a, b] = faces;
        let sa = self.surface(a)?;
        let sb = self.surface(b)?;
        let mut work = PairWork {
            faces,
            boundary: self.listed_boundary(faces)?,
            boundary_cells: 0,
            source_chart_cells: 0,
            volume_separation: None,
            hull_disjoint: false,
            hull_cells: 0,
            periodic: sa.periodic_u || sa.periodic_v || sb.periodic_u || sb.periodic_v,
            pair_allowance: 0,
        };
        self.budgeted_opposite_sides(&mut work, out);
        work.pair_allowance = self.limits.cells_per_pair.saturating_sub(work.boundary_cells);
        // Quintic polygon corner charts need a source-volume proof: their
        // piecewise profile weights need not have one global chart map.
        // Try the exact four-wall cell before spending the pair budget on
        // a stitched surface. Other chart families keep their ordering.
        let volume_first =
            sa.degree_u == 1 && sa.degree_v == 5 && sb.degree_u == 1 && sb.degree_v == 5;
        if volume_first && !work.periodic && self.chart_open(&work, out) {
            self.source_volume_chart(&mut work, out)?;
        }
        self.control_hull_separation(&mut work, sa, sb, out);
        if work.periodic && self.chart_open(&work, out) {
            work.hull_disjoint = crate::boundary_hull_contact::periodic_cap_disjoint(self.model, faces);
            work.hull_cells = 1;
            work.pair_allowance -= 1;
            out.cells += 1;
        }
        self.chart_partitions(&mut work, out)?;
        self.jordan_partition(&mut work, out)?;
        if !volume_first && !work.periodic && self.chart_open(&work, out) {
            self.source_volume_chart(&mut work, out)?;
        }
        self.closed_graph(&mut work, out);
        self.trimmed_search(work, sa, sb, out)
    }
    fn surface(&self, face: usize) -> Result<&crate::Surface> {
        self.model
            .faces
            .get(face)
            .map(|f| &f.surface)
            .ok_or_else(face_error)
    }
    /// Boundary certificates that need no shared work ledger.
    fn listed_boundary(&self, faces: [usize; 2]) -> Result<Option<SharedBoundary>> {
        let [a, b] = faces;
        let model = self.model;
        let joint = self.joint_exact_domain;
        if let Some(c) = self
            .caps
            .iter()
            .find(|c| [c[0].min(c[1]), c[0].max(c[1])] == [a, b])
        {
            return Ok(Some(SharedBoundary::SweepCap {
                cap: c[0],
                wall: c[1],
                edge: c[2],
            }));
        }
        if let Some(c) = self.hulls.iter().find(|c| c.faces == [a, b]) {
            return Ok(Some(SharedBoundary::ExactHull(c.clone())));
        }
        if let Some(c) = joint
            .then(|| crate::boundary_hull_contact::certify(model, [a, b]))
            .flatten()
        {
            // Only the joint embedding audit authorizes this recomputation.
            // The same individual-pair limit bounds all such attempts,
            // including pairs beyond the old lexicographic prefix.
            return Ok(Some(SharedBoundary::ExactHull(c)));
        }
        if let Some(c) = crate::shared_boundary::certify(model, [a, b]) {
            return Ok(Some(SharedBoundary::PlanarFace(c)));
        }
        if let Some(c) = joint
            .then(|| {
                crate::shared_boundary::bilinear_graph::certify(model, [a, b], BILINEAR_GRAPH_CELLS)
            })
            .flatten()
        {
            return Ok(Some(SharedBoundary::BilinearGraph(c)));
        }
        Ok(crate::shared_boundary::certify_opposite(model, [a, b])?
            .map(SharedBoundary::OppositeSides))
    }
    fn budgeted_opposite_sides(&self, work: &mut PairWork, out: &mut Report) {
        if work.boundary.is_some() || !self.sweep_hulls || !self.joint_exact_domain {
            return;
        }
        let allowance = (self.limits.cells - out.cells)
            .min(self.limits.cells_per_pair.saturating_sub(work.boundary_cells))
            .min(CHART_ATTEMPT_CELLS);
        let (certificate, used) =
            crate::shared_boundary::certify_opposite_with_budget(self.model, work.faces, allowance);
        work.boundary_cells += used;
        out.cells += used;
        if let Some(c) = certificate {
            work.boundary = Some(SharedBoundary::OppositeSides(c));
        }
    }
    /// A source-chart stage may run only while the pair and shared budgets remain open.
    fn chart_open(&self, work: &PairWork, out: &Report) -> bool {
        work.unresolved()
            && self.joint_exact_domain
            && work.source_chart_cells < work.pair_allowance
            && out.cells < self.limits.cells
    }
    fn chart_budget(&self, work: &PairWork, out: &Report, cap: usize) -> usize {
        (self.limits.cells - out.cells)
            .min(work.pair_allowance - work.source_chart_cells)
            .min(cap)
    }
    fn source_volume_chart(&self, work: &mut PairWork, out: &mut Report) -> Result<()> {
        let volume = self.source_volume.ok_or_else(|| {
            Error::new(
                "BREP_CONTACT_SOURCE_VOLUME",
                "Prepare source volumes before joint exact-domain certification",
            )
        })?;
        let separated = volume.certify(work.faces, self.chart_budget(work, out, usize::MAX))?;
        work.source_chart_cells += separated.cells;
        out.cells += separated.cells;
        if let Some(c) = separated.certificate {
            if c.shared_edge.is_some() {
                work.boundary = Some(SharedBoundary::SourceVolumeBoundary(c));
            } else {
                work.volume_separation = Some(c);
            }
        }
        Ok(())
    }
    /// Exact control-hull separation is the fast path only for sweep
    /// certification, including closed no-cap bodies: the plain face-contact
    /// API keeps the frozen trim-search work accounting byte-for-byte.
    fn control_hull_separation(
        &self,
        work: &mut PairWork,
        sa: &crate::Surface,
        sb: &crate::Surface,
        out: &mut Report,
    ) {
        if work.boundary.is_none()
            && work.volume_separation.is_none()
            && !work.periodic
            && (self.sweep_hulls || self.joint_exact_domain)
        {
            let [a, b] = work.faces;
            let allowance = (self.limits.cells - out.cells)
                .min(
                    self.limits
                        .cells_per_pair
                        .saturating_sub(work.boundary_cells + work.source_chart_cells)
                        / 2,
                )
                .min(HULL_SEPARATION_CELLS);
            let cached = if allowance > 0 {
                self.cover.and_then(|c| c.pair_separated(a, b))
            } else {
                None
            };
            (work.hull_disjoint, work.hull_cells) = match cached {
                Some(true) => (true, 1),
                Some(false) => {
                    let (separated, used) =
                        crate::control_hull_separation::inspect(sa, sb, allowance - 1);
                    (separated, used + 1)
                }
                None => crate::control_hull_separation::inspect(sa, sb, allowance),
            };
        }
        out.cells += work.hull_cells;
        work.pair_allowance = work.pair_allowance.saturating_sub(work.hull_cells);
    }
    /// Runs one source-chart attempt and charges its work to the pair and the report.
    fn chart_attempt(
        &self,
        work: &mut PairWork,
        out: &mut Report,
        cap: usize,
        attempt: impl FnOnce(&Model, [usize; 2], usize) -> Result<(usize, Option<SharedBoundary>)>,
    ) -> Result<()> {
        let budget = self.chart_budget(work, out, cap);
        let (cells, boundary) = attempt(self.model, work.faces, budget)?;
        work.source_chart_cells += cells;
        out.cells += cells;
        work.boundary = boundary;
        Ok(())
    }
    fn chart_partitions(&self, work: &mut PairWork, out: &mut Report) -> Result<()> {
        use crate::shared_boundary::opposite_rows;
        let shared_edges = |work: &PairWork| {
            projected_trim_network::multiple_shared_edges(self.model, work.faces)
        };
        if self.chart_open(work, out) && shared_edges(work) {
            self.chart_attempt(work, out, usize::MAX, |model, faces, budget| {
                let attempt = opposite_rows::certify(model, faces, budget);
                Ok((attempt.cells, attempt.certificate.map(SharedBoundary::ExactHull)))
            })?;
        }
        if self.chart_open(work, out) {
            self.chart_attempt(work, out, RESTRICTED_ROWS_CELLS, |model, faces, budget| {
                let attempt = opposite_rows::certify_restricted(model, faces, budget);
                Ok((attempt.cells, attempt.certificate.map(SharedBoundary::ExactHull)))
            })?;
        }
        if self.chart_open(work, out) && shared_edges(work) {
            self.chart_attempt(work, out, CHART_ATTEMPT_CELLS, |model, faces, budget| {
                let attempt = projected_trim_network::certify(model, faces, budget)?;
                Ok((
                    attempt.cells,
                    attempt.certificate.map(SharedBoundary::ProjectedTrimNetwork),
                ))
            })?;
        }
        if self.chart_open(work, out) {
            self.chart_attempt(work, out, CHART_ATTEMPT_CELLS, |model, faces, budget| {
                let attempt = same_support_contact::certify(model, faces, budget)?;
                Ok((
                    attempt.cells,
                    attempt.certificate.map(SharedBoundary::SameSupportPartition),
                ))
            })?;
        }
        if self.chart_open(work, out) {
            self.chart_attempt(work, out, CHART_ATTEMPT_CELLS, |model, faces, budget| {
                let attempt = projected_trim_contact::certify(model, faces, budget)?;
                Ok((
                    attempt.cells,
                    attempt.certificate.map(SharedBoundary::ProjectedTrimPartition),
                ))
            })?;
        }
        if !work.periodic && self.chart_open(work, out) {
            self.chart_attempt(work, out, usize::MAX, |model, faces, budget| {
                let joined = boundary_stitched_contact::certify(model, faces, budget)?;
                Ok((
                    joined.cells,
                    joined.certificate.map(SharedBoundary::JoinedSourceCharts),
                ))
            })?;
        }
        Ok(())
    }
    fn jordan_partition(&self, work: &mut PairWork, out: &mut Report) -> Result<()> {
        if self.chart_open(work, out) {
            self.chart_attempt(work, out, usize::MAX, |model, faces, budget| {
                let (certificate, cells) = crate::shared_boundary::jordan_partition::certify(model, faces, budget);
                Ok((cells, certificate.map(SharedBoundary::ExactHull)))
            })?;
        }
        Ok(())
    }
    fn closed_graph(&self, work: &mut PairWork, out: &mut Report) {
        if !work.unresolved() || !self.joint_exact_domain {
            return;
        }
        let allowance = (self.limits.cells - out.cells)
            .min(self.limits.cells_per_pair.saturating_sub(work.spent()));
        let (certificate, used) =
            crate::shared_boundary::closed_graph::certify(self.model, work.faces, allowance);
        work.boundary_cells += used;
        out.cells += used;
        work.boundary = certificate.map(SharedBoundary::ClosedGraph);
    }
    /// Falls back to the trimmed search and records the pair's final classification.
    fn trimmed_search(
        &self,
        work: PairWork,
        sa: &crate::Surface,
        sb: &crate::Surface,
        out: &mut Report,
    ) -> Result<Pair> {
        let [a, b] = work.faces;
        let limits = self.limits;
        let result = if work.boundary.is_some()
            || work.disjoint()
            || work.periodic
            || out.cells == limits.cells
            || work.spent() == limits.cells_per_pair
        {
            None
        } else {
            let search = if self.sweep_hulls {
                surface_contact_search::search_trimmed_with_control_hulls
            } else {
                surface_contact_search::search_trimmed
            };
            let domain_a = self.domains.get(a).ok_or_else(face_error)?;
            let domain_b = self.domains.get(b).ok_or_else(face_error)?;
            Some(search(
                sa,
                sb,
                [&domain_a.region, &domain_b.region],
                (limits.cells - out.cells).min(limits.cells_per_pair - work.spent()),
                (limits.domain_cells - out.domain_cells).min(limits.domain_cells_per_pair),
            )?)
        };
        let reason = match &result {
            None if work.disjoint() => "pair-disjoint",
            None if work.boundary.is_some() => "shared-boundary",
            None if work.periodic => "periodic-trim-not-supported",
            None => "pair-unresolved",
            Some(r) if r.absence_proven => "pair-disjoint",
            Some(r) if r.contact.is_some() => "interior-contact",
            Some(_) => "pair-unresolved",
        };
        let proven_absent = result.as_ref().is_some_and(|r| r.absence_proven);
        out.all_pairs_classified &= work.boundary.is_some() || work.disjoint() || proven_absent;
        out.all_pairs_disjoint &= work.disjoint() || proven_absent;
        if let Some(r) = &result {
            out.cells += r.cells;
            out.domain_cells += r.domain_cells;
        }
        Ok(Pair {
            faces: work.faces,
            result,
            boundary: work.boundary,
            hull_disjoint: work.hull_disjoint,
            hull_cells: work.hull_cells,
            boundary_cells: work.boundary_cells,
            source_chart_cells: work.source_chart_cells,
            volume_separation: work.volume_separation,
            reason,
        })
    }
}

fn face_error() -> Error {
    Error::new("BREP_CONTACT_FACE", "Use face pairs inside the model")
}


#[cfg(test)]
#[path = "face_contacts/tests/tests_integration.rs"]
mod tests;

#[path = "face_contacts/frozen_contacts.rs"]
mod frozen_contacts;
#[cfg(test)]
use frozen_contacts::frozen_inspect_impl;
