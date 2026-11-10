//! Exact oriented volume flux over the actual sewn surface definitions.
//! A positive value is not a global embedding or solid-admission certificate.
use super::*;
use nurbs_core::{
    original_graph_transition::{polynomial_surface_flux, sum_fluxes},
    surface::Surface,
};
pub struct SignedFlux {
    faces: Vec<Q>,
    total: Q,
}
impl SignedFlux {
    pub fn face_contributions(&self) -> &[Q] {
        &self.faces
    }
    pub fn total(&self) -> &Q {
        &self.total
    }
}
fn normalize(s: &Surface, curves: Vec<Vec<[Q; 2]>>) -> Result<Vec<Vec<[Q; 2]>>> {
    let domains = [
        (s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]),
        (s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]),
    ];
    let low = domains.map(|(a, _)| Q::from_float(a).unwrap());
    let span = std::array::from_fn::<_, 2, _>(|k| Q::from_float(domains[k].1).unwrap() - &low[k]);
    require(
        span.iter().all(|x| x > &Q::zero()),
        "Invalid original flux chart",
    )?;
    Ok(curves
        .into_iter()
        .map(|c| {
            c.into_iter()
                .map(|p| std::array::from_fn(|k| (&p[k] - &low[k]) / &span[k]))
                .collect()
        })
        .collect())
}
impl ClosedShell {
    /// Rechecks actual saved topology/lifts, then computes sum integral z dxdy.
    /// The result is algebraic signed flux. Self-intersecting shells can also
    /// have positive flux, so this operation deliberately never creates a body.
    pub fn signed_flux(&self, b: &mut Budget) -> Result<SignedFlux> {
        let top = self.topology();
        let fresh = inspect_roof_geometry(top, b)?;
        require(
            top.shells.len() == 1 && top.shells[0].closed && fresh.open_edges.is_empty(),
            "Flux requires the exact closed shell",
        )?;

        let mut faces = Vec::new();
        for use_ in &top.shells[0].faces {
            let face = &top.faces[use_.face];
            let value = match &face.surface {
                FaceGeometry::Transition(p) => p.flux_polynomial(b)?.integrate_rectangle(b)?,
                FaceGeometry::Retained(r) => {
                    let curves = r
                        .uses()
                        .iter()
                        .map(|(u, reverse)| {
                            let mut points = u.uv_points().to_vec();
                            if *reverse {
                                points.reverse();
                            }
                            points
                        })
                        .collect();
                    polynomial_surface_flux(r.source(), b)?
                        .integrate_loop(&normalize(r.source(), curves)?, b)?
                }
                FaceGeometry::Imported(f) => {
                    let curves = f.uses.iter().map(|u| u.uv.to_vec()).collect();
                    polynomial_surface_flux(f.source(), b)?
                        .integrate_loop(&normalize(f.source(), curves)?, b)?
                }
            };
            let value = if use_.reversed { -value } else { value };
            faces.push(value);
        }
        let total = sum_fluxes(&faces, b)?;
        Ok(SignedFlux { faces, total })
    }
}
