//! Atomic trim/sew admission on the actual saved boundary, never on proposals.
use super::{ContactCut, ContactReplacement, refused, trim_and_replace_contacts};
use crate::{Model, face_curvature};
use nurbs_core::{Result, surface::Surface};

#[derive(Clone, Copy)]
pub struct ContactQualificationLimits {
    pub cells: usize,
    pub exact_work: u64,
    pub domain_cells: usize,
    pub shape_cells: usize,
}
impl Default for ContactQualificationLimits {
    fn default() -> Self {
        Self {
            cells: 100_000,
            exact_work: 1_000_000,
            domain_cells: 1_000_000,
            shape_cells: 1_000_000,
        }
    }
}
/// The model and evidence cannot be independently replaced after admission.
/// Sharp source/end edges are outside G2 scope. This receipt does not certify
/// material containment, thickness, radius laws or global self-intersections.
pub struct QualifiedContactBody {
    body: ContactReplacement,
    seams: Vec<(usize, nurbs_core::surface_g2_pcurves::Report)>,
    fields: Vec<face_curvature::Report>,
    cells: usize,
    exact_work: u64,
    domain_cells: usize,
    shape_cells: usize,
}
impl QualifiedContactBody {
    pub fn model(&self) -> &Model {
        &self.body.replacement.model
    }
    pub fn contacts(&self) -> &ContactReplacement {
        &self.body
    }
    pub fn seams(&self) -> &[(usize, nurbs_core::surface_g2_pcurves::Report)] {
        &self.seams
    }
    pub fn fields(&self) -> &[face_curvature::Report] {
        &self.fields
    }
    pub fn work(&self) -> (usize, u64, usize, usize) {
        (
            self.cells,
            self.exact_work,
            self.domain_cells,
            self.shape_cells,
        )
    }
}
/// Trim immutable source supports and sew the supplied contact loop; publish
/// only after whole saved seams pass G2 and every retained trimmed face passes
/// |principal curvature| <= 10/mm. Construction, seams and fields share bounded
/// work ledgers. `smooth` must explicitly select every intended smooth seam.
pub fn trim_and_replace_contacts_g2(
    source: &Model,
    cuts: &[ContactCut<'_>],
    surface: &Surface,
    reversed: bool,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
) -> Result<QualifiedContactBody> {
    if !(8..=100_000).contains(&limits.cells)
        || limits.exact_work > 1_000_000
        || !(1..=1_000_000).contains(&limits.domain_cells)
        || !(1..=1_000_000).contains(&limits.shape_cells)
        || source.faces.len() > 256
        || !cuts.iter().any(|c| c.smooth)
    {
        return Err(refused(
            "Need bounded qualification work and explicitly selected G2 contacts",
        ));
    }
    let body = trim_and_replace_contacts(
        source,
        cuts,
        surface,
        reversed,
        tolerance_uv,
        limits.cells,
        limits.exact_work,
    )?;
    let model = &body.replacement.model;
    let mut cells = body.cells;
    let mut exact_work = body.exact_work;
    let mut domain_cells = 0;
    let mut shape_cells = 0;
    let mut seams = Vec::new();
    for &edge in &body.smooth_edges {
        if cells >= limits.cells {
            return Err(refused("Saved-seam qualification work exhausted"));
        }
        let mut uses = Vec::new();
        for (face, f) in model.faces.iter().enumerate() {
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for co in &model.loops[wire].coedges {
                    if co.edge == edge {
                        uses.push((
                            face,
                            if co.reversed {
                                co.pcurve.reverse()?
                            } else {
                                co.pcurve.clone()
                            },
                        ));
                    }
                }
            }
        }
        if uses.len() != 2 || uses[0].0 == uses[1].0 {
            return Err(refused(
                "Saved G2 contact must have two distinct support uses",
            ));
        }
        let report = nurbs_core::surface_g2_pcurves::inspect(nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &model.edges[edge].curve, pcurves: [&uses[0].1, &uses[1].1], surfaces: [
                &model.faces[uses[0].0].surface,
                &model.faces[uses[1].0].surface,
            ], reversed: [false; 2] }, nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: source.tolerance_mm, normal: 1e-5, curvature: 1e-4 }, limits.cells - cells, limits.exact_work - exact_work)?;
        cells += report.geometric.cells;
        exact_work += report.positions.exact_work;
        if !report.positions.proven
            || report.geometric.decision != nurbs_core::surface_join::Decision::WithinTolerance
        {
            return Err(refused("Whole saved contact G2 remains unproven"));
        }
        seams.push((edge, report));
    }
    let mut fields = Vec::new();
    for face in 0..model.faces.len() {
        if cells >= limits.cells
            || domain_cells >= limits.domain_cells
            || shape_cells >= limits.shape_cells
        {
            return Err(refused("Whole saved-body curvature work exhausted"));
        }
        let report = face_curvature::inspect(
            model,
            face,
            10.,
            tolerance_uv,
            face_curvature::Limits {
                cells: (limits.cells - cells).min(50_000),
                domain_cells: limits.domain_cells - domain_cells,
                shape_cells: limits.shape_cells - shape_cells,
            },
            &[],
        )?;
        cells += report.cells;
        domain_cells += report.domain_cells;
        shape_cells += report.shape_cells;
        if report.decision != face_curvature::Decision::Within
            || report
                .absolute_curvature_upper
                .is_none_or(|x| !x.is_finite() || x > 10.)
        {
            return Err(refused(
                "Saved trimmed face curvature <=10/mm remains unproven",
            ));
        }
        fields.push(report);
    }
    Ok(QualifiedContactBody {
        body,
        seams,
        fields,
        cells,
        exact_work,
        domain_cells,
        shape_cells,
    })
}
