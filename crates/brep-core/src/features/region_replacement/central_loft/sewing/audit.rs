//! Independent fresh material, embedding and flux results; no inherited flags.
use super::*;
#[derive(Clone, Copy)]
pub struct CornerAuditLimits {
    pub spans: usize,
    pub linear_cells: usize,
    pub contacts: crate::face_contacts::Limits,
    pub flux: FluxLimits,
}
pub struct CornerBodyAudit {
    pub material: MaterialReport,
    pub contacts: ContactReport,
    pub central_embedding: ProjectionDiskReport,
    pub absence_proven: bool,
    pub intersections: crate::self_intersection::Report,
    pub flux: FluxReport,
    pub exact_boundary: bool,
    pub volume_bounds_mm3: Option<[f64; 2]>,
    pub material_preservation_proven: bool,
    pub removed_volume_bounds_mm3: Option<[f64; 2]>,
    pub stock_volume_mm3: [f64; 2],
}
impl SewnCornerCandidate {
    pub fn audit(&self, tolerance_uv: f64, l: CornerAuditLimits) -> Result<CornerBodyAudit> {
        self.model.validate()?;
        require(
            tolerance_uv.is_finite() && tolerance_uv > 0.,
            "Bound sewn corner UV tolerance",
        )?;
        let material = material::inspect(&self.model, self.stock_bounds, self.trims.all_valid)?;
        let faces =
            crate::face_injectivity::inspect_with_linear(&self.model, l.spans, l.linear_cells)?;
        let pairs = crate::face_contacts::inspect(&self.model, tolerance_uv, l.contacts)?;
        let direction = std::array::from_fn(|k| {
            if self.corner_point[k] == self.stock_bounds[k][0] {
                1
            } else {
                -1
            }
        });
        let central = projection::inspect(
            &self.model.faces[self.central_face].surface,
            direction,
            10000,
            1000000,
        )?;
        let all_faces = faces.faces.iter().all(|f| {
            f.absence_proven() || (f.face == self.central_face && central.certificate.is_some())
        });
        let absence_proven = faces.all_faces_injective && pairs.all_pairs_classified;
        let exact_boundary = self.closure.exact.all_equal && self.closure.exact.all_joins_exact;
        let contacts = contacts::inspect(
            &self.model,
            &pairs,
            exact_boundary,
            material.boundary_inside_stock,
            100000,
        );
        let joint_absence = all_faces && contacts.all_pairs_classified;
        let intersections = crate::self_intersection::Report {
            absence_proven,
            faces,
            pairs,
        };
        let flux = flux::inspect(&self.model, tolerance_uv, l.flux)?;
        let volume_bounds_mm3 = flux.signed_flux_mm3.filter(|v| {
            self.closed_boundary_proven()
                && exact_boundary
                && joint_absence
                && flux.converged
                && v[0] > 0.
        });
        let material_preservation_proven = material.boundary_inside_stock
            && volume_bounds_mm3.is_some_and(|v| v[1] <= self.stock_volume[1]);
        let removed_volume_bounds_mm3 = if material_preservation_proven {
            let v = volume_bounds_mm3.unwrap();
            let d = nurbs_core::interval_eval::Interval::new(
                self.stock_volume[0],
                self.stock_volume[1],
            )?
            .sub(nurbs_core::interval_eval::Interval::new(v[0], v[1])?)?;
            Some([d.lo, d.hi])
        } else {
            None
        };
        Ok(CornerBodyAudit {
            material,
            contacts,
            central_embedding: central.report,
            absence_proven: joint_absence,
            intersections,
            flux,
            exact_boundary,
            volume_bounds_mm3,
            material_preservation_proven,
            removed_volume_bounds_mm3,
            stock_volume_mm3: self.stock_volume,
        })
    }
}
