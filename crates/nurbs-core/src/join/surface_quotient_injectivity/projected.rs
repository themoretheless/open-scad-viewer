//! Global injectivity of the ORIGINAL world projection on a polar collar.
use super::*;
use crate::surface_join::Boundary;
pub struct ProjectedPolarCollar<'a> {
    surface: &'a Surface,
    boundary: Boundary,
    dropped_axis: usize,
    inward_extent: f64,
    margin: f64,
    radial_factor: f64,
    outer_radius: f64,
}
impl<'a> ProjectedPolarCollar<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    pub fn dropped_axis(&self) -> usize {
        self.dropped_axis
    }
    pub fn inward_extent(&self) -> f64 {
        self.inward_extent
    }
    pub fn dominance_margin_lower(&self) -> f64 {
        self.margin
    }
    /// |projected S(u,v) - projected pole| >= factor*u in inward normalized u.
    pub fn radial_distance_factor_lower(&self) -> f64 {
        self.radial_factor
    }
    /// Every point on the outer radial boundary is at least this far from
    /// the projected pole. This does not assert a whole disk lies in the image.
    pub fn outer_radius_lower(&self) -> f64 {
        self.outer_radius
    }
}
pub struct ProjectedPolarReport<'a> {
    pub certificate: Option<ProjectedPolarCollar<'a>>,
    pub cells: usize,
    pub reason: &'static str,
}
/// Prove injectivity of the two-coordinate projection modulo only the original
/// collapsed boundary. The strict dominance map F,G is made from functionals
/// with the dropped coordinate identically absent BEFORE any arithmetic.
/// Equality of projected points forces equality of F,G, hence parameters.
///
/// F_u>=a>0 and F(0,v)=0 give F>=a*u. An outward upper bound for the exact
/// source functional's L1 norm gives an explicit projected radial separation.
/// The source is never cropped, resampled, or rounded into a projected proxy.
pub fn certify_projected_polar_collar(
    s: &Surface,
    boundary: Boundary,
    dropped_axis: usize,
    inward_extent: f64,
    subdivisions: usize,
    max_cells: usize,
) -> Result<ProjectedPolarReport<'_>> {
    s.validate()?;
    let view = collar::source_view(s, boundary);
    let end = usize::from(matches!(boundary, Boundary::UMax | Boundary::VMax));
    let proof = report::certify_projected_polar_collar_inner(
        &view,
        end,
        inward_extent,
        subdivisions,
        max_cells,
        dropped_axis,
    )?;
    let mut out = ProjectedPolarReport {
        certificate: None,
        cells: proof.cells,
        reason: proof.reason,
    };
    if !proof.proven {
        return Ok(out);
    }
    let p = view.degree_u;
    let q = view.degree_v;
    let origin = &view.control_points[if end == 0 { 0 } else { p }][0];
    let far = &view.control_points[if end == 0 { p } else { 0 }];
    let mut norm = I::point(0.);
    for k in 0..3 {
        if k == dropped_axis {
            continue;
        }
        let h = I::point(far[0][k])
            .sub(I::point(origin[k]))?
            .add(I::point(far[q][k]).sub(I::point(origin[k]))?)?;
        norm = norm.add(I::point(h.lo.abs().max(h.hi.abs())))?;
    }
    if norm.hi <= 0. {
        out.reason = "projected-polar-radial-functional-unproven";
        return Ok(out);
    }
    let a = proof.derivative_bounds.unwrap()[0];
    let radial_factor = I::point(a).div(I::point(norm.hi))?.lo;
    let outer_radius = I::point(radial_factor).mul(I::point(inward_extent))?.lo;
    if radial_factor <= 0. || outer_radius <= 0. {
        out.reason = "projected-polar-positive-radius-unproven";
        return Ok(out);
    }
    out.certificate = Some(ProjectedPolarCollar {
        surface: s,
        boundary,
        dropped_axis,
        inward_extent,
        margin: proof.dominance_margin_lower.unwrap(),
        radial_factor,
        outer_radius,
    });
    out.reason = "original-projected-polar-collar-injective-and-radially-separated";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        // S=(u-3u²/4, (u-3u²/4)*v, 16u). Original S is globally
        // injective, while its XY projection folds after u=2/3.
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![2., 2., 2., 5., 5., 5.],
            knots_v: vec![-4., -4., 8., 8.],
            control_points: vec![
                vec![vec![0.; 3]; 2],
                vec![vec![0.5, 0., 8.], vec![0.5, 0.5, 8.]],
                vec![vec![0.25, 0., 16.], vec![0.25, 0.25, 16.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn source_injectivity_is_not_projected_injectivity_and_radius_is_a_bound() {
        let s = source();
        let original = s.clone();
        let r = certify_polar_collar(&s, Boundary::UMin, 1., 8, 64).unwrap();
        assert!(r.certificate.is_some(), "{}", r.reason);
        assert!(
            certify_projected_polar_collar(&s, Boundary::UMin, 2, 1., 8, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        let r = certify_projected_polar_collar(&s, Boundary::UMin, 2, 0.125, 8, 64).unwrap();
        let p = r.certificate.expect(r.reason);
        assert!(std::ptr::eq(p.surface(), &s));
        assert_eq!(s, original);
        assert!(p.radial_distance_factor_lower() > 0.);
        assert!(p.radial_distance_factor_lower() <= 1. - 0.75 * 0.125);
        assert!(
            p.outer_radius_lower() > 0. && p.outer_radius_lower() <= 0.125 - 0.75 * 0.125 * 0.125
        );
        assert_eq!(r.cells, 64);
        assert!(
            certify_projected_polar_collar(&s, Boundary::UMin, 2, 0.125, 8, 63)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    #[test]
    fn all_world_projections_and_natural_sides_keep_original_identity() {
        for axis in 0..3 {
            for boundary in [
                Boundary::UMin,
                Boundary::UMax,
                Boundary::VMin,
                Boundary::VMax,
            ] {
                let mut s = source();
                for p in s.control_points.iter_mut().flatten() {
                    let old = p.clone();
                    for k in 0..3 {
                        p[(k + axis + 1) % 3] = old[k];
                    }
                }
                if matches!(boundary, Boundary::UMax | Boundary::VMax) {
                    s.control_points.reverse();
                    s.weights.reverse();
                }
                if matches!(boundary, Boundary::VMin | Boundary::VMax) {
                    s = collar::source_view(&s, Boundary::VMin);
                }
                for r in &mut s.weights {
                    for w in r {
                        *w = 8.;
                    }
                }
                let before = s.clone();
                let r = certify_projected_polar_collar(&s, boundary, axis, 0.125, 8, 64).unwrap();
                let p = r.certificate.expect(r.reason);
                assert!(std::ptr::eq(p.surface(), &s));
                assert_eq!(p.dropped_axis(), axis);
                assert!(p.dominance_margin_lower() > 0.);
                assert_eq!(s, before);
            }
        }
    }
    #[test]
    fn collapsed_projection_invalid_inputs_and_partial_work_never_admit() {
        let mut s = source();
        for r in &mut s.control_points {
            for p in r {
                p[0] = 0.;
                p[1] = 0.;
            }
        }
        assert!(
            certify_projected_polar_collar(&s, Boundary::UMin, 2, 0.125, 8, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            certify_projected_polar_collar(&source(), Boundary::UMin, 3, 0.125, 8, 64).is_err()
        );
        assert!(certify_projected_polar_collar(&source(), Boundary::UMin, 2, 0., 8, 64).is_err());
    }
}
