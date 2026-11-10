//! Material-bounded six-sided central proposals. Source G2, quotient regularity,
//! surrounding contacts and sewn-corner volume admission remain separate.
use super::{TrihedralStrips, build_trihedral_rim, inspect_rim_supports};
use crate::source_box::QualifiedStockBox;
use nurbs_core::{
    Error, Result, curve::Curve, curve_surface_agreement as agreement, surface::Surface,
};
#[path = "central_loft/layers.rs"]
mod layers;
#[path = "central_loft/common_denominator.rs"]
mod common_denominator;
#[path = "central_loft/endpoint_jets.rs"]
mod endpoint_jets;
#[path = "central_loft/interior_fairing.rs"]
mod interior_fairing;
#[path = "central_loft/bivariate_fairing.rs"]
mod bivariate_fairing;
#[path = "central_loft/curvature_search.rs"]
mod curvature_search;
#[path = "central_loft/joint_seam_search.rs"]
mod joint_seam_search;
#[derive(Clone, Copy)]
pub struct CentralLoftOptions {
    /// Control-handle distance in world units; never an admission tolerance.
    pub handle_mm: f64,
    pub tolerances: [f64; 3],
    pub tolerance_uv: f64,
    pub operations: usize,
    pub cells: usize,
}
pub struct CentralLoftCandidate<'b, 's, 'p> {
    body: &'b QualifiedStockBox<'s>,
    strips: &'p TrihedralStrips,
    options: CentralLoftOptions,
    surface: Surface,
    rim_origin: usize,
    authored: layers::AuthoredRows,
    curves: Vec<Curve>,
    pcurves: Vec<Curve>,
    originals: Vec<(Surface, Curve)>,
    stock: [[f64; 2]; 3],
    controls: [[f64; 2]; 3],
    positions: Vec<agreement::Report>,
    operations: usize,
    cells: usize,
    correction: f64,
}
impl CentralLoftCandidate<'_, '_, '_> {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn curves(&self) -> &[Curve] {
        &self.curves
    }
    pub fn pcurves(&self) -> &[Curve] {
        &self.pcurves
    }
    /// Every positive-weight control lies within these original stock planes.
    pub fn stock_bounds(&self) -> [[f64; 2]; 3] {
        self.stock
    }
    pub fn control_bounds(&self) -> [[f64; 2]; 3] {
        self.controls
    }
    pub fn positions(&self) -> &[agreement::Report] {
        &self.positions
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    /// Maximum coordinate change of boundary proposal poles; full-curve proof is separate.
    pub fn boundary_correction_mm(&self) -> f64 {
        self.correction
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = propose_trihedral_center_at_rim(self.body, self.strips, self.options, self.rim_origin)?;
        require(
            self.surface == fresh.surface
                && self.rim_origin == fresh.rim_origin
                && self.authored == fresh.authored
                && self.curves == fresh.curves
                && self.pcurves == fresh.pcurves
                && self.stock == fresh.stock
                && self.controls == fresh.controls
                && self.operations == fresh.operations
                && self.cells == fresh.cells
                && self.correction == fresh.correction,
            "Central loft original geometry or material bounds changed",
        )
    }
    /// Fresh full original-support inspections. There is no inherited G2 flag;
    /// endpoint quotient geometry needs its own proof if ordinary jets vanish.
    pub fn inspect_original_g2(
        &self,
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<nurbs_core::surface_g2_pcurves::OwnedInspection>> {
        require(
            (6..=100000).contains(&max_cells) && max_exact <= 1000000,
            "Bound central original seam work",
        )?;
        let mut used = 0;
        let mut exact = 0;
        let mut reports = Vec::new();
        for i in 0..6 {
            let proof = nurbs_core::surface_g2_pcurves::inspect_owned(
                &self.curves[i],
                [&self.originals[i].1, &self.pcurves[i]],
                [&self.originals[i].0, &self.surface],
                [false, false],
                self.options.tolerances[0],
                self.options.tolerances[1],
                self.options.tolerances[2],
                (max_cells - used) / (6 - i),
                (max_exact - exact) / (6 - i) as u64,
            )?;
            used += proof.report.geometric.cells;
            exact += proof.report.positions.exact_work;
            require(
                used <= max_cells && exact <= max_exact,
                "Central seam query exceeded shared work",
            )?;
            reports.push(proof);
        }
        Ok(reports)
    }
}
fn refuse(s: &str) -> Error {
    Error::new("BREP_CENTRAL_LOFT_UNPROVEN", s)
}
fn require(ok: bool, s: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(refuse(s)) }
}
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn charge(&mut self, n: usize) -> Result<()> {
        require(
            n <= self.max - self.used,
            "Central construction work exhausted",
        )?;
        self.used += n;
        Ok(())
    }
}
pub fn propose_trihedral_center<'b, 's, 'p>(
    body: &'b QualifiedStockBox<'s>,
    strips: &'p TrihedralStrips,
    o: CentralLoftOptions,
) -> Result<CentralLoftCandidate<'b, 's, 'p>> {
    propose_trihedral_center_at_rim(body,strips,o,0)
}
/// A different chart of the SAME six owned boundary curves. `rim_origin` selects
/// one of three profile origins (cyclic offsets 0/2/4), never a source refit.
/// Position/support checks are fresh here; G2, curvature and body admission
/// remain separate mandatory gates. No certificate is inherited by reordering.
pub fn propose_trihedral_center_at_rim<'b, 's, 'p>(
    body: &'b QualifiedStockBox<'s>, strips: &'p TrihedralStrips,
    o: CentralLoftOptions, rim_origin: usize,
) -> Result<CentralLoftCandidate<'b, 's, 'p>> {
    require(rim_origin<3,"Central rim origin must select one of three owned profiles")?;
    require(
        o.handle_mm.is_finite()
            && o.handle_mm > 0.
            && o.tolerances.iter().all(|t| t.is_finite() && *t > 0.)
            && o.tolerances[0] <= body.source().tolerance_mm
            && o.tolerances[0] * 0.5 > 0.
            && o.tolerances[1] <= 2.
            && o.tolerance_uv.is_finite()
            && o.tolerance_uv > 0.
            && (1..=1000000).contains(&o.operations)
            && (12..=100000).contains(&o.cells),
        "Bound central handle, tolerances and work",
    )?;
    let mut work = Work {
        used: 0,
        max: o.operations,
    };
    work.charge(body.operations())?;
    body.reverify()?;
    let source = body.source();
    let stock = body.bounds();
    require(
        strips.strips.len() == 3
            && strips.plan.vertex < source.vertices.len()
            && source.1.vertices.get(strips.plan.vertex) == Some(&strips.plan.identity),
        "Central strips lost the actual source corner",
    )?;
    let origin = source.vertices[strips.plan.vertex].point;
    let mut axes = Vec::new();
    for s in &strips.strips {
        require(
            s.source_edge < source.edges.len()
                && source.1.edges.get(s.source_edge) == Some(&s.source_identity)
                && s.support_faces.iter().all(|&f| f < source.faces.len())
                && s.patch.model.faces.len() == 1,
            "Central strip lost source edge/support ownership",
        )?;
        let e = &source.edges[s.source_edge];
        require(
            e.vertices.contains(&strips.plan.vertex),
            "Strip edge is not incident to original corner",
        )?;
        let other = e
            .vertices
            .iter()
            .copied()
            .find(|&v| v != strips.plan.vertex)
            .ok_or_else(|| refuse("Degenerate corner edge"))?;
        let p = source.vertices[other].point;
        let nonzero: Vec<_> = (0..3).filter(|&k| p[k] != origin[k]).collect();
        require(
            nonzero.len() == 1,
            "Central loft needs original axis-aligned edge directions",
        )?;
        let mut d = [0.; 3];
        d[nonzero[0]] = (origin[nonzero[0]] - p[nonzero[0]]).signum();
        axes.push(d);
    }
    let mut rim = build_trihedral_rim(strips, o.tolerances)?;
    if rim_origin>0 {
        work.charge(6)?;
        rim.ordered.rotate_left(2*rim_origin);
    }
    let support = inspect_rim_supports(source, &rim, o.tolerance_uv, o.cells / 3)?;
    let mut directions = [[0.; 3]; 6];
    let mut originals = Vec::new();
    for (i, c) in rim.ordered.iter().enumerate() {
        let mut found = Vec::new();
        for (j, section) in rim.cross_sections.iter().enumerate() {
            let s = &strips.strips[j];
            let near = usize::from(s.end_order[0] != 0);
            for reversed in [false, true] {
                let actual = if reversed {
                    section.reverse()?
                } else {
                    section.clone()
                };
                if actual == *c {
                    directions[i] = axes[j];
                    let p = &s.patch.ends[near].skin_pcurve;
                    found.push((
                        s.patch.model.faces[0].surface.clone(),
                        if reversed { p.reverse()? } else { p.clone() },
                    ));
                }
            }
        }
        for (j, connector) in rim.connectors.iter().enumerate() {
            for reversed in [false, true] {
                let actual = if reversed {
                    connector.curve.reverse()?
                } else {
                    connector.curve.clone()
                };
                if actual == *c {
                    let p = &support.pcurves[j];
                    found.push((
                        source.faces[connector.source_face].surface.clone(),
                        if reversed { p.reverse()? } else { p.clone() },
                    ));
                }
            }
        }
        require(
            found.len() == 1,
            "Central rim support is not uniquely source-owned",
        )?;
        originals.push(found.remove(0));
    }
    require(
        (0..6).all(|i| {
            if i % 2 == 0 {
                directions[i].iter().filter(|&&x| x != 0.).count() == 1
            } else {
                directions[i] == [0.; 3]
            }
        }),
        "Central rim order must alternate cross sections and connectors",
    )?;
    let (surface, pcurves, correction, authored) = layers::build(
        &rim.ordered,
        directions,
        o.handle_mm,
        stock,
        o.tolerances[0] / 32.,
        &mut work,
    )?;
    let mut controls = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for p in surface.control_points.iter().flatten() {
        work.charge(3)?;
        for k in 0..3 {
            require(
                stock[k][0] <= p[k] && p[k] <= stock[k][1],
                "Central control hull leaves the actual original stock",
            )?;
            controls[k][0] = controls[k][0].min(p[k]);
            controls[k][1] = controls[k][1].max(p[k]);
        }
    }
    let mut cells = support.cells;
    let mut positions = Vec::new();
    for i in 0..6 {
        for (side,(s, p)) in [(&originals[i].0, &originals[i].1), (&surface, &pcurves[i])].into_iter().enumerate() {
            require(cells < o.cells, "Central positional work exhausted")?;
            let proof = agreement::verify(
                &rim.ordered[i],
                p,
                s,
                false,
                o.tolerances[0] * 0.5,
                o.cells - cells,
            )?;
            cells += proof.cells;
            if proof.status != agreement::Status::WithinTolerance {
                return Err(refuse(&format!(
                    "Central boundary {i} position unproven: {proof:?}; side={side}, world_degree={}, world_poles={}, pcurve_degree={}, pcurve_poles={}, support_degrees=[{},{}]", rim.ordered[i].degree,rim.ordered[i].control_points.len(),p.degree,p.control_points.len(),s.degree_u,s.degree_v
                )));
            }
            positions.push(proof);
        }
    }
    Ok(CentralLoftCandidate {
        body,
        strips,
        options: o,
        surface,
        rim_origin,
        authored,
        curves: rim.ordered,
        pcurves,
        originals,
        stock,
        controls,
        positions,
        operations: work.used,
        cells,
        correction,
    })
}
#[cfg(test)]
#[path = "central_loft/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "central_loft/span_diagnostics.rs"]
mod span_diagnostics;

#[path = "central_loft/span_coverage.rs"]
mod span_coverage;

#[path = "central_loft/poles.rs"]
mod poles;

#[path = "central_loft/exterior.rs"]
mod exterior;

#[path = "central_loft/sewing.rs"]
mod sewing;
pub use sewing::{SewnCornerNetworkCandidate,OpenCornerNetwork,NetworkSupportBinding,
    CornerInternalG2Audit, CornerInternalJoin, QualifiedCornerInternalG2,
    ContactReport, CornerAuditLimits, CornerBodyAudit, CornerCurvatureAudit, CornerCurvatureFace,
    CornerSewLimits, FluxLimits, FluxReport, MaterialReport, ProjectionDiskReport,
    QualifiedG2SewnCorner, QualifiedSewnCorner, SEWN_CORNER_CURVATURE_LIMIT, SewnCornerCandidate,
    StripFitReport,
};
