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
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn enclose_material_flux(
        &self, face: usize, domain: [[f64; 2]; 2], axis: usize, origin: f64,
        tolerance_uv: f64, controls: usize, spans: usize, domain_cells: usize,
    ) -> Result<MaterialFluxReport> {
        if !tolerance_uv.is_finite() || tolerance_uv <= 0. || domain_cells > 100000 {
            return Err(Error::new("BREP_PROFILE_FLUX", "Bound material classification tolerance and work"));
        }
        let natural = self.support_domain(face)?;
        if (0..2).any(|a| domain[a][0] < natural[a][0] || domain[a][1] > natural[a][1]) {
            return Err(Error::new("BREP_PROFILE_FLUX", "Keep flux rectangle in owned support domain"));
        }
        let mut out = MaterialFluxReport { bound: None, spans: 0, domain_cells: 0, reason: "expression-flux-enclosure-unproven" };
        let (flux, used) = self.enclose_support_flux(face, domain, axis, origin, controls, spans)?;
        out.spans = used;
        let Some(flux) = flux else { return Ok(out); };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let source = root.restricted.proposal.body.geometry().shell();
        let mut integral = I::new(flux.integral[0], flux.integral[1])?;
        let winding = match self.faces[face].kind() {
            ShellFaceKind::Transition => if self.assembly.patch.rails.reverse_face { -1 } else { 1 },
            kind => {
                let original = match kind { ShellFaceKind::Original(i) => i, _ => root.face };
                let winding = source.regions().unwrap()[original].chart_winding();
                let area = I::point(domain[0][1]).sub(I::point(domain[0][0]))?
                    .mul(I::point(domain[1][1]).sub(I::point(domain[1][0]))?)?;
                integral = I::new(flux.density[0].min(0.), flux.density[1].max(0.))?.mul(area)?;
                if domain_cells > 0 {
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
