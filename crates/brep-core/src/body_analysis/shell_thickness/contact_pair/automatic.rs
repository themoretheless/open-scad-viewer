//! Unseeded original-root discovery feeding actual, freshly qualified BRep caps.
use super::{Band, Limits, Thickness, refused, thicken};
use crate::Result;
use nurbs_core::{
    offset_contact_path,
    surface::Surface,
    surface_offset::{PairDomain, boundary_contact, coverage},
};

#[derive(Clone, Copy)]
pub struct DiscoveryLimits {
    pub search: coverage::DiscoveryLimits,
    /// Independent bounded search of all eight literal natural boundaries.
    pub boundary_search: boundary_contact::DiscoveryLimits,
    /// Independent per-attempt construction budgets, not one shared allowance.
    pub construction: Limits,
    /// Caps the number of complete construction attempts, including failures.
    pub max_constructions: usize,
}
impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            search: coverage::DiscoveryLimits {
                candidate_boxes: 4096,
                parameter_tolerance: 0.25,
                predictor_queries: 64,
                max_paths: 8,
                path: offset_contact_path::Limits {
                    cells: 64,
                    spans: 32,
                    iterations: 16,
                    numerical_tolerance_mm: 1e-9,
                    padding_fraction: 0.1,
                },
                qualification: coverage::Limits {
                    boxes: 20000,
                    spans_per_bound: 32,
                    path_cells: 8192,
                },
            },
            construction: Limits::default(),
            boundary_search: boundary_contact::DiscoveryLimits::default(),
            max_constructions: 8,
        }
    }
}
pub struct Attempt {
    request: usize,
    band: Band,
    skin: Option<Thickness>,
    reason: String,
    covers_all_original_roots: bool,
}
impl Attempt {
    pub fn request(&self) -> usize {
        self.request
    }
    pub fn band(&self) -> Band {
        self.band
    }
    /// A locally qualified closed skin. Omitted original contact remains in
    /// discovery.pending; this is not an automatic global material/solid union.
    pub fn local_skin(&self) -> Option<&Thickness> {
        self.skin.as_ref()
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
    /// Only root-set coverage, not support extension or global solid embedding.
    pub fn covers_all_original_roots(&self) -> bool {
        self.covers_all_original_roots
    }
}
pub struct AutomaticReport {
    sources: [Surface; 2],
    reversed: [bool; 2],
    distance: f64,
    uv_tolerance: f64,
    normal_tolerance: f64,
    sewing_tolerance: f64,
    limits: DiscoveryLimits,
    discovery: coverage::DiscoveryReport,
    boundary_contacts: boundary_contact::DiscoveryReport,
    attempts: Vec<Attempt>,
}
impl AutomaticReport {
    pub fn discovery(&self) -> &coverage::DiscoveryReport {
        &self.discovery
    }
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
    /// Certified boundary endpoints do not erase pending interior root regions
    /// or turn independently found endpoints into a connected trimmed branch.
    pub fn boundary_contacts(&self) -> &boundary_contact::DiscoveryReport {
        &self.boundary_contacts
    }
    /// Build fresh certified bridges from one automatically discovered endpoint
    /// to this report's automatically discovered interior paths. Caller supplies
    /// only work limits, never UV anchors, bridge curves or positive receipts.
    pub fn connect_endpoint(
        &self,
        index: usize,
        limits: boundary_contact::attachment::bridge::Limits,
    ) -> Result<boundary_contact::attachment::bridge::Report> {
        self.boundary_contacts
            .endpoints
            .get(index)
            .ok_or_else(|| refused("Endpoint index outside original discovery"))?
            .bridge_paths(&self.discovery.requests, limits)
    }
    /// Fit one discovered complete path as an actual C0 paired-UV/world chain.
    /// Original cell tubes/clocks propose pieces; all curve and root-transition
    /// authority is regenerated. This does not close a trim region or select
    /// material, and omitted natural-domain root regions remain pending.
    pub fn fit_path(
        &self,
        index: usize,
        tolerance_uv: f64,
        tolerance_mm: f64,
        limits: nurbs_core::surface_offset::contact_curve::chain::Limits,
    ) -> Result<nurbs_core::surface_offset::contact_curve::chain::Chain> {
        use nurbs_core::surface_offset::contact_curve::chain::{self, Span};
        let request = self
            .discovery
            .requests
            .get(index)
            .ok_or_else(|| refused("Path index outside original discovery"))?;
        let path = self
            .discovery
            .coverage
            .paths
            .get(index)
            .filter(|p| p.continuous_path_proven)
            .ok_or_else(|| refused("Discovered path lacks complete original continuity"))?;
        let spans: Vec<_> = path
            .cells
            .iter()
            .map(|c| Span {
                owner: 0,
                axis: request.axis,
                drive: c.drive,
                free: c.first_other,
                other: c.second,
                reversed: false,
            })
            .collect();
        let distances = self
            .reversed
            .map(|r| if r { -self.distance } else { self.distance });
        Ok(chain::build(
            self.sources.each_ref(),
            distances,
            &spans,
            tolerance_uv,
            tolerance_mm,
            limits,
        )?)
    }
    /// Close an automatically discovered contact path on a selected original
    /// natural boundary. This returns a qualified UV Jordan region with world
    /// lifts, not a sewn solid or inferred material sector. Initial chain fit
    /// and closure's fresh chain rebuild each have their own stage allowance.
    #[allow(clippy::too_many_arguments)]
    pub fn close_path(
        &self,
        index: usize,
        side: usize,
        far_side: usize,
        tolerance_uv: f64,
        tolerance_mm: f64,
        chain_limits: nurbs_core::surface_offset::contact_curve::chain::Limits,
        region_limits: nurbs_core::surface_offset::contact_curve::chain::region::Limits,
    ) -> Result<nurbs_core::surface_offset::contact_curve::chain::region::Region> {
        if side >= 2 || far_side >= 4 {
            return Err(refused("Choose an original source and natural boundary"));
        }
        let chain = self.fit_path(index, tolerance_uv, tolerance_mm, chain_limits)?;
        Ok(chain.close_to_boundary(side, far_side, region_limits)?)
    }
    pub fn complete_contact_coverage(&self) -> bool {
        self.attempts.iter().any(|a| a.covers_all_original_roots)
    }
    /// Fresh discovery and full native reconstruction, including failed attempts
    /// and pending domains. A saved local skin cannot supply global coverage.
    pub fn reverify(&self) -> Result<()> {
        let fresh = discover(
            &self.sources,
            self.reversed,
            self.distance,
            self.uv_tolerance,
            self.normal_tolerance,
            self.sewing_tolerance,
            self.limits,
        )?;
        if fresh.complete_contact_coverage() != self.complete_contact_coverage()
            || fresh.discovery.coverage.pending != self.discovery.coverage.pending
            || fresh.discovery.coverage.root_coverage_proven
                != self.discovery.coverage.root_coverage_proven
            || fresh.discovery.coverage.absence_proven != self.discovery.coverage.absence_proven
            || fresh.attempts.len() != self.attempts.len()
            || fresh.boundary_contacts.faces.len() != self.boundary_contacts.faces.len()
            || fresh
                .boundary_contacts
                .faces
                .iter()
                .zip(&self.boundary_contacts.faces)
                .any(|(a, b)| {
                    a.boundary != b.boundary
                        || a.candidates.pending != b.candidates.pending
                        || a.candidates.boxes != b.candidates.boxes
                        || a.coverage != b.coverage
                })
            || fresh.boundary_contacts.endpoints.len() != self.boundary_contacts.endpoints.len()
            || fresh
                .boundary_contacts
                .endpoints
                .iter()
                .zip(&self.boundary_contacts.endpoints)
                .any(|(a, b)| {
                    a.boundary() != b.boundary()
                        || a.original_tube() != b.original_tube()
                        || a.regular_inward_branch_proven() != b.regular_inward_branch_proven()
                        || a.witness()
                            .map(|w| (w.first_uv, w.second_uv, w.point, w.contraction_upper))
                            != b.witness()
                                .map(|w| (w.first_uv, w.second_uv, w.point, w.contraction_upper))
                })
            || fresh.attempts.iter().zip(&self.attempts).any(|(a, b)| {
                a.request != b.request
                    || a.band != b.band
                    || a.reason != b.reason
                    || a.covers_all_original_roots != b.covers_all_original_roots
                    || a.skin.as_ref().map(|s| s.model()) != b.skin.as_ref().map(|s| s.model())
            })
        {
            return Err(refused("Automatic discovery differs from original replay"));
        }
        Ok(())
    }
}
fn domain(b: Band) -> PairDomain {
    let mut first = [[0.; 2]; 2];
    first[b.axis] = b.drive;
    first[1 - b.axis] = b.first_other;
    [first, b.second]
}
fn contains(a: PairDomain, b: PairDomain) -> bool {
    (0..2).all(|s| (0..2).all(|k| a[s][k][0] <= b[s][k][0] && a[s][k][1] >= b[s][k][1]))
}
fn band(request: &coverage::PathRequest, path: &offset_contact_path::Report) -> Option<Band> {
    if !path.continuous_path_proven || path.cells.is_empty() {
        return None;
    }
    let mut b = Band {
        axis: request.axis,
        drive: request.drive,
        first_other: [f64::INFINITY, f64::NEG_INFINITY],
        second: [[f64::INFINITY, f64::NEG_INFINITY]; 2],
    };
    for c in &path.cells {
        b.first_other[0] = b.first_other[0].min(c.first_other[0]);
        b.first_other[1] = b.first_other[1].max(c.first_other[1]);
        for k in 0..2 {
            b.second[k][0] = b.second[k][0].min(c.second[k][0]);
            b.second[k][1] = b.second[k][1].max(c.second[k][1]);
        }
    }
    Some(b)
}
/// No seeds, drive interval, UV maps, caps or positive receipts are supplied.
/// Independently replayed original-root paths propose enclosing contact bands;
/// every attempted band repeats uniform UV/world and full actual BRep gates.
/// Failed constructions retain diagnostics. Incomplete discovery retains every
/// pending natural-domain region even when a useful LOCAL skin was built.
/// Each attempt has its own explicit work allowance, with a bounded attempt
/// count; failed work is never silently refunded into a shared budget.
#[allow(clippy::too_many_arguments)]
pub fn discover(
    sources: &[Surface; 2],
    reversed: [bool; 2],
    distance: f64,
    uv_tolerance: f64,
    normal_tolerance: f64,
    sewing_tolerance: f64,
    limits: DiscoveryLimits,
) -> Result<AutomaticReport> {
    super::validate(limits.construction)?;
    if !(1..=32).contains(&limits.max_constructions)
        || !uv_tolerance.is_finite()
        || uv_tolerance <= 0.
    {
        return Err(refused(
            "Choose bounded constructions and positive UV tolerance",
        ));
    }
    super::super::freeform_network::validate(
        sources,
        sources,
        &reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness: distance,
            normal_tolerance,
            normal_cells: limits.construction.caps.proof_cells,
            sewing_tolerance,
        },
        limits.construction.sewing,
    )?;
    let distances = reversed.map(|r| if r { -distance } else { distance });
    let discovery = coverage::discover(sources.each_ref(), distances, limits.search)?;
    let boundary_contacts =
        boundary_contact::discover(sources.each_ref(), distances, limits.boundary_search)?;
    let mut attempts = vec![];
    for (index, (request, path)) in discovery
        .requests
        .iter()
        .zip(&discovery.coverage.paths)
        .enumerate()
    {
        if attempts.len() == limits.max_constructions {
            break;
        }
        let Some(b) = band(request, path) else {
            continue;
        };
        // Enclosing several successful tubes is a proposal, never an inherited
        // uniform branch proof. thicken must prove the entire enclosing band.
        let built = thicken(
            sources,
            reversed,
            distance,
            b,
            uv_tolerance,
            normal_tolerance,
            sewing_tolerance,
            limits.construction,
        );
        let (skin, reason) = match built {
            Ok(s) => (Some(s), "qualified-local-skin".into()),
            Err(e) => (None, e.to_string()),
        };
        let all = skin.is_some()
            && discovery.coverage.root_coverage_proven
            && !discovery.coverage.absence_proven
            && !discovery.coverage.covered.is_empty()
            && discovery
                .coverage
                .covered
                .iter()
                .all(|c| contains(domain(b), c.domain));
        attempts.push(Attempt {
            request: index,
            band: b,
            skin,
            reason,
            covers_all_original_roots: all,
        });
    }
    Ok(AutomaticReport {
        sources: sources.clone(),
        reversed,
        distance,
        uv_tolerance,
        normal_tolerance,
        sewing_tolerance,
        limits,
        discovery,
        boundary_contacts,
        attempts,
    })
}
