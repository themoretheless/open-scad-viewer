//! Oriented material-mask flux bounds. Unclassified trim retains its full
//! possible contribution; a rectangle is never silently dropped.
use super::*;
use nurbs_core::{interval_eval::Interval as I, trim_domain::Location};

pub struct MaterialFluxReport {
    pub bound: Option<[f64; 2]>,
    pub spans: usize,
    pub domain_cells: usize,
    pub reason: &'static str,
}
fn unknown_material_mask(flux: &nurbs_core::surface_flux::Bound, domain: [[f64; 2]; 2]) -> Result<I> {
    let mut area = I::point(1.);
    for range in domain {
        area = area.mul(I::point(range[1]).sub(I::point(range[0]))?)?;
    }
    I::new(flux.density[0].min(0.), flux.density[1].max(0.))?.mul(area)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn enclose_material_flux(
        &self, face: usize, domain: [[f64; 2]; 2], axis: usize, origin: f64,
        tolerance_uv: f64, controls: usize, spans: usize, domain_cells: usize,
    ) -> Result<MaterialFluxReport> {
        if !tolerance_uv.is_finite() || tolerance_uv <= 0. || domain_cells > 100000
            || controls > 1000000 || spans > 100000 || axis >= 3 || !origin.is_finite()
        {
            return Err(Error::new("BREP_PROFILE_FLUX", "Bound material classification tolerance and work"));
        }
        let natural = self.support_domain(face)?;
        if (0..2).any(|a| domain[a][0] < natural[a][0] || domain[a][1] > natural[a][1]) {
            return Err(Error::new("BREP_PROFILE_FLUX", "Keep flux rectangle in owned support domain"));
        }
        let mut out = MaterialFluxReport { bound: None, spans: 0, domain_cells: 0, reason: "expression-flux-enclosure-unproven" };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let source = root.restricted.proposal.body.geometry().shell();
        let unchanged = self.equivalent_original_face(face);
        let (flux, used) = if let Some(original) = unchanged {
            let surface = source.faces()[original][0].edges()[0].surface();
            let report = nurbs_core::surface_flux::bound(surface, domain, axis, origin, spans)?;
            (report.bound, report.spans)
        } else {
            self.enclose_support_flux(face, domain, axis, origin, controls, spans)?
        };
        out.spans = used;
        let Some(flux) = flux else { return Ok(out); };
        let mut integral = I::new(flux.integral[0], flux.integral[1])?;
        let winding = match self.faces[face].kind() {
            ShellFaceKind::Transition => if self.assembly.patch.rails.reverse_face { -1 } else { 1 },
            kind => {
                let original = match kind { ShellFaceKind::Original(i) => i, _ => root.face };
                let winding = source.regions().unwrap()[original].chart_winding();
                let whole_chart = unchanged == Some(original) && source.regions().unwrap()[original].whole_chart_material();
                if !whole_chart { integral = unknown_material_mask(&flux, domain)?; }
                if !whole_chart && domain_cells > 0 {
                    let loops = self.faces[face].loops.iter().map(|wire| wire.iter().filter_map(|u| match u {
                        ShellUse::Source(f) => Some(f.clone()), ShellUse::Transition(_) => None,
                    }).collect()).collect::<Vec<Vec<_>>>();
                    let classification = crate::source_contour_winding::classify(&loops, domain, tolerance_uv, domain_cells)?;
                    out.domain_cells = classification.cells;
                    if classification.location == Location::Outside { integral = I::point(0.); }
                    else if classification.location == Location::Inside && classification.winding == Some(winding) {
                        integral = I::new(flux.integral[0], flux.integral[1])?;
                    }
                }
                winding
            }
        };
        if winding != 1 && winding != -1 {
            return Err(Error::new("BREP_PROFILE_FLUX", "Require admitted chart orientation"));
        }
        if winding == -1 { integral = integral.mul(I::point(-1.))?; }
        out.bound = Some([integral.lo, integral.hi]);
        out.reason = "expression-oriented-material-flux-enclosed";
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mask_contains_full_flux_with_identical_outward_area_rounding() {
        let derivatives = [[[1.,1.],[0.,0.],[0.,0.]],[[0.,0.],[1.,1.],[0.,0.]]];
        for z in [[2.,3.],[-3.,-2.],[-2.,3.]] {
            for domain in [[[0.2,0.3],[0.2,0.3]],[[0.9,1.],[0.,1.]],[[0.,2.],[0.,3.]]] {
                let flux=nurbs_core::surface_flux::bound_expression([[0.,1.],[0.,1.],z],derivatives,domain,2,0.,false).unwrap();
                let mask=unknown_material_mask(&flux,domain).unwrap();
                assert!(mask.lo<=flux.integral[0] && flux.integral[1]<=mask.hi);
                assert!(mask.lo<=0. && mask.hi>=0.);
            }
        }
    }
}
