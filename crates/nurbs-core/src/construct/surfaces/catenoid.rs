//! Catenary profile approximation followed by rational circular revolution.
use crate::{Result, check, surface::Surface};
pub struct Approximation {
    pub surface: Surface,
    pub spans: usize,
    pub budget: f64,
    pub real_arithmetic_error_estimate: f64,
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for Approximation {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"surface":self.surface,"report":{"spans":self.spans,"budget":self.budget,
   "realArithmeticErrorEstimate":self.real_arithmetic_error_estimate,"continuousBound":false,
   "roundingCertified":false,"method":"catenary-profile-Hermite-rational-revolution"}})
    }
}
/// Z-axis catenoid, radial distance scale*cosh(local_z/scale).
/// Full rational circular revolution of an approximate cubic profile.
pub fn approximate(
    center: [f64; 3],
    scale: f64,
    start_z: f64,
    end_z: f64,
    budget: f64,
) -> Result<Approximation> {
    let fit = radial_profile(center, scale, start_z, end_z, budget)?;
    let surface = crate::surface::revolve(&fit.curve, center, [0., 0., 1.], 360.)?;
    Ok(Approximation {
        surface,
        spans: fit.spans,
        budget,
        real_arithmetic_error_estimate: fit.real_arithmetic_error_estimate,
    })
}
fn radial_profile(
    center: [f64; 3],
    scale: f64,
    start_z: f64,
    end_z: f64,
    budget: f64,
) -> Result<crate::helix::Approximation> {
    check(
        center.iter().all(|x| x.is_finite()),
        "Catenoid center must be finite",
    )?;
    let mut fit = crate::catenary::approximate([0.; 3], scale, start_z, end_z, budget)?.fit;
    for p in &mut fit.curve.control_points {
        let z = p[0];
        let radial = p[1] + scale;
        check(
            radial.is_finite() && radial > 0.,
            "Catenoid radial controls must be finite and positive",
        )?;
        *p = vec![center[0] + radial, center[1], center[2] + z];
    }
    fit.curve.validate()?;
    Ok(fit)
}
pub struct PatchesApproximation {
    pub patches: Vec<Surface>,
    pub spans: usize,
    pub budget: f64,
    pub real_arithmetic_error_estimate: f64,
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for PatchesApproximation {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patches":self.patches,"report":{"spans":self.spans,"budget":self.budget,
   "realArithmeticErrorEstimate":self.real_arithmetic_error_estimate,"continuousBound":false,
   "roundingCertified":false,"method":"catenary-profile-Hermite-rational-revolution"}})
    }
}
/// Retained U-domain patches for dense catenary profiles; no refitting.
/// At most85 cubic spans. Circle V remains [0,4], periodic_v=false.
/// Patches are not sewn into solid topology; knot-insertion rounding excluded.
pub fn approximate_patches(
    center: [f64; 3],
    scale: f64,
    start_z: f64,
    end_z: f64,
    budget: f64,
) -> Result<PatchesApproximation> {
    let fit = radial_profile(center, scale, start_z, end_z, budget)?;
    let patches = if fit.curve.control_points.len() <= 32 {
        vec![crate::surface::revolve(
            &fit.curve,
            center,
            [0., 0., 1.],
            360.,
        )?]
    } else {
        fit.curve
            .decompose()?
            .iter()
            .map(|segment| {
                crate::surface::revolve(segment.definition(), center, [0., 0., 1.], 360.)
            })
            .collect::<Result<Vec<_>>>()?
    };
    Ok(PatchesApproximation {
        patches,
        spans: fit.spans,
        budget,
        real_arithmetic_error_estimate: fit.real_arithmetic_error_estimate,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dense_catenoid_preserves_tight_profile_and_patch_domains() {
        let full = approximate([3., 4., 5.], 1., -5., 5., 1e-4).unwrap();
        let fit = approximate_patches([3., 4., 5.], 1., -5., 5., 1e-4).unwrap();
        assert_eq!(fit.patches.len(), fit.spans);
        assert!(fit.patches.len() > 10);
        for (i, patch) in fit.patches.iter().enumerate() {
            let lo = i as f64 / fit.spans as f64;
            let hi = (i + 1) as f64 / fit.spans as f64;
            assert_eq!(patch.knots_u[0], lo);
            assert_eq!(*patch.knots_u.last().unwrap(), hi);
            for t in [0., 0.13, 0.5, 0.87, 1.] {
                let u = lo + (hi - lo) * t;
                let z = -5. + 10. * u;
                let r = z.cosh();
                for v in [0., 0.3, 1., 2.7, 4.] {
                    let p = patch.evaluate(u, v).unwrap().point;
                    let whole = full.surface.evaluate(u, v).unwrap().point;
                    assert!(whole.iter().zip(&p).all(|(x, y)| (x - y).abs() < 1e-10));
                    assert!(
                        ((p[0] - 3.).hypot(p[1] - 4.) - r).abs()
                            <= fit.real_arithmetic_error_estimate + 1e-10
                    );
                    assert!((p[2] - 5. - z).abs() < 1e-11);
                    if i > 0 && t == 0. {
                        let a = fit.patches[i - 1].evaluate(u, v).unwrap().point;
                        assert!(a.iter().zip(&p).all(|(x, y)| (x - y).abs() < 1e-10));
                    }
                }
            }
            for row in &patch.control_points {
                assert_eq!(row.first(), row.last());
            }
        }
        assert!(approximate_patches([0.; 3], 1., -5., 5., 1e-10).is_err());
    }
    #[test]
    fn independent_catenoid_radial_equation_and_exact_revolution_seam() {
        let a = approximate([3., 4., 5.], 2., -2., 3., 1e-4).unwrap();
        for i in 0..=200 {
            let u = i as f64 / 200.;
            let z = -2. + 5. * u;
            let radius = 2. * (z / 2.).cosh();
            for v in [0., 0.3, 1., 2.7, 4.] {
                let p = a.surface.evaluate(u, v).unwrap().point;
                let r = (p[0] - 3.).hypot(p[1] - 4.);
                assert!((r - radius).abs() <= a.real_arithmetic_error_estimate + 1e-12);
                assert!((p[2] - (5. + z)).abs() < 1e-12);
            }
        }
        for row in &a.surface.control_points {
            assert_eq!(row.first(), row.last());
        }
    }
    #[test]
    fn refuses_precision_requests_exceeding_single_surface_budget() {
        assert!(approximate([0.; 3], 1., -5., 5., 1e-10).is_err());
        assert!(approximate([0.; 3], 0., -1., 1., 1e-4).is_err());
    }
}
