//! Original-source local quotient topology. A collar certificate identifies
//! only the complete collapsed side; it does not assert regularity of a full
//! incident fan, curvature limits, or exclusion of contacts outside the collar.
use super::*;
use crate::surface_join::Boundary;

pub struct PolarCollar<'a> {
    surface: &'a Surface,
    boundary: Boundary,
    inward_extent: f64,
    margin: f64,
}
impl PolarCollar<'_> {
    pub fn surface(&self) -> &Surface {
        self.surface
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    /// Normalized distance inward from the named original boundary.
    pub fn inward_extent(&self) -> f64 {
        self.inward_extent
    }
    pub fn dominance_margin_lower(&self) -> f64 {
        self.margin
    }
}
pub struct PolarCollarReport<'a> {
    pub certificate: Option<PolarCollar<'a>>,
    pub cells: usize,
    pub reason: &'static str,
}
/// Prove one-to-one mapping of an original closed collar modulo its collapsed
/// side. The extended radial/angular map is globally injective by strict
/// weighted diagonal dominance on that rectangle. Positive radial derivative
/// separates every interior point from the pole. Compactness then makes the
/// induced continuous bijection a homeomorphism onto its image.
///
/// The radius is normalized in the original parameter domain. Transposition
/// and row reversal only permute original controls; no rounded patch is built.
pub fn certify_polar_collar(
    s: &Surface,
    boundary: Boundary,
    inward_extent: f64,
    subdivisions: usize,
    max_cells: usize,
) -> Result<PolarCollarReport<'_>> {
    s.validate()?;
    let transpose = matches!(boundary, Boundary::VMin | Boundary::VMax);
    let view = if transpose {
        Surface {
            degree_u: s.degree_v,
            degree_v: s.degree_u,
            knots_u: s.knots_v.clone(),
            knots_v: s.knots_u.clone(),
            control_points: (0..s.control_points[0].len())
                .map(|v| s.control_points.iter().map(|r| r[v].clone()).collect())
                .collect(),
            weights: (0..s.weights[0].len())
                .map(|v| s.weights.iter().map(|r| r[v]).collect())
                .collect(),
            periodic_u: s.periodic_v,
            periodic_v: s.periodic_u,
        }
    } else {
        s.clone()
    };
    let end = usize::from(matches!(boundary, Boundary::UMax | Boundary::VMax));
    let proof =
        report::certify_polar_collar_inner(&view, end, inward_extent, subdivisions, max_cells)?;
    Ok(PolarCollarReport {
        certificate: if proof.proven {
            Some(PolarCollar {
                surface: s,
                boundary,
                inward_extent,
                margin: proof.dominance_margin_lower.unwrap(),
            })
        } else {
            None
        },
        cells: proof.cells,
        reason: if proof.proven {
            "original-polar-collar-quotient-homeomorphism"
        } else {
            proof.reason
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn folded() -> Surface {
        // S=(R(u), R(u)*v, 0), R=u-3u²/4: injective near u=0,
        // overlapping rays after u=2/3. A local token must not admit the whole.
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
    fn local_original_collar_succeeds_while_global_fold_refuses() {
        let s = folded();
        let original = s.clone();
        assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
        let p = certify_polar_collar(&s, Boundary::UMin, 0.125, 8, 64).unwrap();
        let token = p.certificate.expect(p.reason);
        assert!(std::ptr::eq(token.surface(), &s));
        assert!(matches!(token.boundary(), Boundary::UMin));
        assert_eq!(token.inward_extent(), 0.125);
        assert!(token.dominance_margin_lower() > 0.);
        assert_eq!(p.cells, 64);
        assert_eq!(s, original);
        assert!(
            certify_polar_collar(&s, Boundary::UMin, 1., 8, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            certify_polar_collar(&s, Boundary::UMin, 0.125, 8, 63)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    #[test]
    fn source_plane_and_topology_compose_without_admitting_the_remote_fold() {
        use crate::factored_surface_normals::prepare_tangent_pole_collar;
        let s = folded();
        let p = prepare_tangent_pole_collar(&s, Boundary::UMin, 0.125, 8, 1000000, 64).unwrap();
        let token = p.certificate.expect(p.reason);
        assert!(std::ptr::eq(token.plane().surface(), &s));
        assert!(std::ptr::eq(token.quotient().surface(), &s));
        assert!(p.exact_work > 0);
        assert!(
            prepare_tangent_pole_collar(&s, Boundary::UMin, 1., 8, 1000000, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            prepare_tangent_pole_collar(&s, Boundary::UMin, 0.125, 8, p.exact_work - 1, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            prepare_tangent_pole_collar(&s, Boundary::UMin, 0.125, 8, 1000000, 63)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    #[test]
    fn all_boundaries_preserve_original_identity_and_parameter_domains() {
        for boundary in [
            Boundary::UMin,
            Boundary::UMax,
            Boundary::VMin,
            Boundary::VMax,
        ] {
            let mut s = folded();
            if matches!(boundary, Boundary::UMax | Boundary::VMax) {
                s.control_points.reverse();
                s.weights.reverse();
            }
            if matches!(boundary, Boundary::VMin | Boundary::VMax) {
                s = Surface {
                    degree_u: s.degree_v,
                    degree_v: s.degree_u,
                    knots_u: s.knots_v,
                    knots_v: s.knots_u,
                    control_points: (0..2)
                        .map(|j| s.control_points.iter().map(|r| r[j].clone()).collect())
                        .collect(),
                    weights: (0..2)
                        .map(|j| s.weights.iter().map(|r| r[j]).collect())
                        .collect(),
                    periodic_u: false,
                    periodic_v: false,
                };
            }
            // Common positive rational scale must not change the original map.
            for row in &mut s.weights {
                for w in row {
                    *w = 8.;
                }
            }
            let original = s.clone();
            let proof = certify_polar_collar(&s, boundary, 0.125, 8, 64).unwrap();
            assert!(proof.certificate.is_some(), "{}", proof.reason);
            assert_eq!(s, original);
        }
    }
    #[test]
    fn repeated_rays_noncollapsed_and_invalid_requests_refuse() {
        let mut s = folded();
        for r in &mut s.control_points {
            r[1] = r[0].clone();
        }
        assert!(
            certify_polar_collar(&s, Boundary::UMin, 0.125, 8, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut s = folded();
        s.control_points[0][1][0] = 1e-12;
        assert!(
            certify_polar_collar(&s, Boundary::UMin, 0.125, 8, 64)
                .unwrap()
                .certificate
                .is_none()
        );
        for r in [0., -1., f64::NAN, f64::INFINITY, 1.01] {
            assert!(certify_polar_collar(&folded(), Boundary::UMin, r, 8, 64).is_err());
        }
    }
}
