//! Exact contraction of an original UV boundary to a single world point.
//! This owns the source restriction; it does not authorize shell or body closure.
use crate::source_boundary_fragment::Fragment;
use cad_predicates::BezierIdentity;
use nurbs_core::{Error, Result, curve::Curve, curve_surface_agreement};

#[derive(Clone)]
pub struct CollapsedBoundary {
    source: Fragment,
    point: [f64; 3],
}
impl CollapsedBoundary {
    pub fn source(&self) -> &Fragment {
        &self.source
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
}
pub struct Report {
    pub boundary: Option<CollapsedBoundary>,
    pub work_used: u64,
    pub reason: &'static str,
}
/// The supplied point is a candidate. Fresh homogeneous identity must prove
/// S(P(t)) equals that constant on the original source definition. Root-valued
/// restrictions retain their original endpoints; no sampled vertex is authority.
pub fn qualify(source: &Fragment, point: [f64; 3], max_work: u64) -> Result<Report> {
    if point.iter().any(|v| !v.is_finite()) || !(1..=cad_predicates::MAX_WORK).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_POLE",
            "Choose a finite pole candidate and bounded identity work",
        ));
    }
    let world = Curve::from_polyline(vec![point.to_vec(), point.to_vec()])?;
    let proof = curve_surface_agreement::verify_exact_algebraic(
        &world,
        source.curve(),
        source.surface(),
        false,
        max_work,
    )?;
    let mut report = Report {
        boundary: None,
        work_used: 0,
        reason: "source-pole-layout-unproven",
    };
    if let Some(proof) = proof {
        report.work_used = proof.work_used;
        if proof.outcome == BezierIdentity::Equal {
            report.boundary = Some(CollapsedBoundary {
                source: source.clone(),
                point,
            });
            report.reason = "source-pole-identity-qualified";
        } else {
            report.reason = "source-pole-identity-unproven";
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::Endpoint;
    use nurbs_core::surface::Surface;
    fn source(deviation: f64) -> Fragment {
        let surface = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![
                    vec![3., -7., 5.],
                    vec![3. + deviation, -7., 5.],
                    vec![3., -7., 5.],
                ],
                vec![vec![4., -7., 6.], vec![4., -6., 6.], vec![3., -6., 6.]],
            ],
            weights: vec![vec![1., 0.5, 1.], vec![1., 0.5, 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let uv = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
        Fragment::new(
            &surface,
            &uv,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap()
    }
    #[test]
    fn rational_pole_owns_unchanged_original_restriction() {
        let source = source(0.).split_at_parameter(0.375).unwrap()[1].clone();
        let report = qualify(&source, [3., -7., 5.], cad_predicates::MAX_WORK).unwrap();
        let pole = report.boundary.unwrap();
        assert_eq!(pole.source().definition(), source.definition());
        assert_eq!(pole.point(), [3., -7., 5.]);
        assert!(report.work_used > 0);
    }
    #[test]
    fn coincident_endpoints_and_small_deviation_do_not_prove_collapse() {
        for deviation in [0.5, 1e-12] {
            assert!(
                qualify(&source(deviation), [3., -7., 5.], cad_predicates::MAX_WORK)
                    .unwrap()
                    .boundary
                    .is_none()
            );
        }
        assert!(
            qualify(
                &source(0.),
                [3., -7., 5.000000000001],
                cad_predicates::MAX_WORK
            )
            .unwrap()
            .boundary
            .is_none()
        );
    }
    #[test]
    fn exhausted_identity_work_and_invalid_candidates_are_not_authority() {
        assert!(
            qualify(&source(0.), [3., -7., 5.], 1)
                .unwrap()
                .boundary
                .is_none()
        );
        assert!(qualify(&source(0.), [f64::NAN, 0., 0.], 10).is_err());
        assert!(qualify(&source(0.), [3., -7., 5.], 0).is_err());
    }
}
