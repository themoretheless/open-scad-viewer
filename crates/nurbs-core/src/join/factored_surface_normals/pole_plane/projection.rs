//! Original-source projection transversality on a complete pole collar.
use super::*;
pub struct ProjectionCollar<'a> {
    surface: &'a Surface,
    boundary: Boundary,
    inward_extent: f64,
    axis: usize,
    factor_order: usize,
    component: [f64; 2],
}
impl ProjectionCollar<'_> {
    pub fn surface(&self) -> &Surface {
        self.surface
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    pub fn inward_extent(&self) -> f64 {
        self.inward_extent
    }
    /// Dropped world coordinate of the two-coordinate projection.
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn factor_order(&self) -> usize {
        self.factor_order
    }
    /// Original normal numerator divided by a positive inward parameter power.
    /// Not a bound separated from zero for the actual Jacobian AT the pole.
    pub fn component_bounds(&self) -> [f64; 2] {
        self.component
    }
}
pub struct ProjectionReport<'a> {
    pub certificate: Option<ProjectionCollar<'a>>,
    pub columns: usize,
    pub reason: &'static str,
}
impl<'a> PlaneLimit<'a> {
    /// Bound the complete retained original normal polynomial on [0, extent]
    /// inward and the FULL angular interval. A separated component proves a
    /// nonzero projected Jacobian everywhere away from the collapsed side.
    /// It does not prove a globally injective projection or a regular pole.
    ///
    /// Each charged column is one original angular Bernstein coefficient
    /// column. Degrees are bounded by the private plane-limit factory. Failed
    /// or unfinished columns can never yield a partial positive token.
    pub fn certify_projection_collar(
        &self,
        axis: usize,
        inward_extent: f64,
        max_columns: usize,
    ) -> Result<ProjectionReport<'a>> {
        check(
            axis < 3
                && inward_extent.is_finite()
                && inward_extent > 0.
                && inward_extent <= 1.
                && (1..=100000).contains(&max_columns),
            "Choose a world projection and bounded original collar columns",
        )?;
        let mut out = ProjectionReport {
            certificate: None,
            columns: 0,
            reason: "pole-projection-column-limit",
        };
        let mut component = [f64::INFINITY, f64::NEG_INFINITY];
        for v in 0..self.net[0].len() {
            if out.columns == max_columns {
                return Ok(out);
            }
            out.columns += 1;
            let column = self.net.iter().map(|r| r[v]).collect::<Vec<_>>();
            let left = split(&column, I::point(inward_extent))?.0;
            for p in left {
                component[0] = component[0].min(p[axis].lo);
                component[1] = component[1].max(p[axis].hi);
            }
        }
        if !(component[0] > 0. || component[1] < 0.) {
            out.reason = "pole-projection-transversality-unproven";
            return Ok(out);
        }
        out.certificate = Some(ProjectionCollar {
            surface: self.surface,
            boundary: self.boundary,
            inward_extent,
            axis,
            factor_order: self.order,
            component,
        });
        out.reason = "original-punctured-pole-collar-projection-transverse";
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn folded() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![2., 2., 2., 5., 5., 5.],
            knots_v: vec![-4., -4., 8., 8.],
            control_points: [0., 0.5, 0.25]
                .iter()
                .map(|&r| vec![vec![r, 0., 0.], vec![r, r, 0.]])
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn a_uniform_pole_plane_does_not_hide_a_projected_fold() {
        let s = folded();
        let original = s.clone();
        let p = prepare_plane_limit(&s, Boundary::UMin, 1000000).unwrap();
        let p = p.certificate.expect(p.reason);
        let r = p.certify_projection_collar(2, 0.125, 100).unwrap();
        let proof = r.certificate.expect(r.reason);
        assert!(std::ptr::eq(proof.surface(), &s));
        assert_eq!(proof.axis(), 2);
        assert_eq!(proof.factor_order(), 1);
        assert!(proof.component_bounds()[0] > 0.);
        assert_eq!(proof.inward_extent(), 0.125);
        assert!(matches!(proof.boundary(), Boundary::UMin));
        assert!(
            p.certify_projection_collar(2, 1., 100)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            p.certify_projection_collar(0, 0.125, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        let short = p
            .certify_projection_collar(2, 0.125, r.columns - 1)
            .unwrap();
        assert!(short.certificate.is_none());
        assert_eq!(short.columns, r.columns - 1);
        assert_eq!(s, original);
    }
    #[test]
    fn retained_high_order_normal_nets_cover_all_natural_sides_and_rational_weights() {
        for m in [1, 2, 3] {
            for weighted in [false, true] {
                for boundary in [
                    Boundary::UMin,
                    Boundary::UMax,
                    Boundary::VMin,
                    Boundary::VMax,
                ] {
                    let s = super::super::tests::oriented(
                        super::super::tests::source(m, weighted),
                        boundary,
                    );
                    let p = prepare_plane_limit(&s, boundary, 100000000).unwrap();
                    let p = p.certificate.expect(p.reason);
                    let r = p.certify_projection_collar(2, 0.125, 100).unwrap();
                    let proof = r.certificate.expect(r.reason);
                    assert!(std::ptr::eq(proof.surface(), &s));
                    let n = proof.component_bounds();
                    assert!(n[0] > 0. || n[1] < 0.);
                    assert_eq!(proof.factor_order(), 2 * m - 1);
                }
            }
        }
    }
    #[test]
    fn high_order_poles_do_not_erase_interior_projected_folds() {
        for m in [1, 2, 3] {
            let p = m + 1;
            // R=(m+1)u^m(1-u), S=(R,R*v,0). The exact projected
            // Jacobian changes sign at u=m/(m+1), after a regular collar.
            let s = Surface {
                degree_u: p,
                degree_v: 1,
                knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
                knots_v: vec![0., 0., 1., 1.],
                control_points: (0..=p)
                    .map(|u| {
                        let r = f64::from(u == m);
                        vec![vec![r, 0., 0.], vec![r, r, 0.]]
                    })
                    .collect(),
                weights: vec![vec![1.; 2]; p + 1],
                periodic_u: false,
                periodic_v: false,
            };
            let report = prepare_plane_limit(&s, Boundary::UMin, 100000000).unwrap();
            let proof = report.certificate.expect(report.reason);
            assert_eq!(proof.factor_order(), 2 * m - 1);
            assert!(
                proof
                    .certify_projection_collar(2, 0.125, 100)
                    .unwrap()
                    .certificate
                    .is_some()
            );
            assert!(
                proof
                    .certify_projection_collar(2, 1., 100)
                    .unwrap()
                    .certificate
                    .is_none()
            );
        }
    }
    #[test]
    fn invalid_projection_requests_refuse() {
        let s = folded();
        let p = prepare_plane_limit(&s, Boundary::UMin, 1000000)
            .unwrap()
            .certificate
            .unwrap();
        for extent in [0., -1., 1.01, f64::NAN, f64::INFINITY] {
            assert!(p.certify_projection_collar(2, extent, 100).is_err());
        }
        assert!(p.certify_projection_collar(3, 0.125, 100).is_err());
        assert!(p.certify_projection_collar(2, 0.125, 0).is_err());
    }
}
