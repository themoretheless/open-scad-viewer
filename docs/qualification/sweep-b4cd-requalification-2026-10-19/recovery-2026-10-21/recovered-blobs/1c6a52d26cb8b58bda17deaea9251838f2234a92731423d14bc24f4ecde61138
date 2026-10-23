//! Radial ruled helicoid using the existing angular Hermite approximation.
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
        value_codec::json!({"surface":self.surface,"report":{
            "spans":self.spans,"budget":self.budget,
            "realArithmeticErrorEstimate":self.real_arithmetic_error_estimate,
            "continuousBound":false,"roundingCertified":false,
            "method":"radial-ruled-helix-Hermite"}})
    }
}

/// P(u,v)=center+[r(u)*cos(theta(v)),r(u)*sin(theta(v)),height*v].
/// r(u)=inner_radius+(outer_radius-inner_radius)*u;
/// theta(v)=phase+2*pi*turns*v. The axis is Z, domains are [0,1].
/// This approximation preserves linear axial advance with angle, unlike a
/// rational-circle twist whose angle is nonlinear in its curve parameter.
pub fn approximate(
    center: [f64; 3],
    inner_radius: f64,
    outer_radius: f64,
    height: f64,
    turns: f64,
    phase_degrees: f64,
    budget: f64,
) -> Result<Approximation> {
    let fit = angular_profile(
        center,
        inner_radius,
        outer_radius,
        height,
        turns,
        phase_degrees,
        budget,
    )?;
    check(
        fit.curve.control_points.len() <= 32,
        "Helicoid exceeds 32 angular controls; multipatch is required",
    )?;
    let surface = ruled_patch(&fit.curve, center, inner_radius / outer_radius)?;
    Ok(Approximation {
        surface,
        spans: fit.spans,
        budget,
        real_arithmetic_error_estimate: fit.real_arithmetic_error_estimate,
    })
}

fn angular_profile(
    center: [f64; 3],
    inner_radius: f64,
    outer_radius: f64,
    height: f64,
    turns: f64,
    phase_degrees: f64,
    budget: f64,
) -> Result<crate::helix::Approximation> {
    check(
        center.iter().all(|x| x.is_finite())
            && inner_radius.is_finite()
            && outer_radius.is_finite()
            && inner_radius >= 0.
            && outer_radius > inner_radius
            && height.is_finite()
            && height != 0.,
        "Helicoid needs finite center, 0 <= inner < outer radii and nonzero height",
    )?;
    crate::helix::approximate([0.; 3], outer_radius, height, turns, phase_degrees, budget)
}
fn ruled_patch(curve: &crate::curve::Curve, center: [f64; 3], ratio: f64) -> Result<Surface> {
    let rows: Vec<Vec<Vec<f64>>> = [ratio, 1.]
        .into_iter()
        .map(|ratio| {
            curve
                .control_points
                .iter()
                .map(|p| {
                    vec![
                        center[0] + ratio * p[0],
                        center[1] + ratio * p[1],
                        center[2] + p[2],
                    ]
                })
                .collect()
        })
        .collect();
    let surface = Surface {
        degree_u: 1,
        degree_v: curve.degree,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: curve.knots.clone(),
        control_points: rows,
        weights: vec![curve.weights.clone(), curve.weights.clone()],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
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
        value_codec::json!({"patches":self.patches,"report":{"spans":self.spans,"budget":self.budget,"realArithmeticErrorEstimate":self.real_arithmetic_error_estimate,"continuousBound":false,"roundingCertified":false,"method":"radial-ruled-helix-Hermite"}})
    }
}
/// Dense angular curves split at their existing knots, retaining V subdomains.
/// No budget relaxation, refitting or sewn-solid topology certificate.
pub fn approximate_patches(
    center: [f64; 3],
    inner_radius: f64,
    outer_radius: f64,
    height: f64,
    turns: f64,
    phase_degrees: f64,
    budget: f64,
) -> Result<PatchesApproximation> {
    let fit = angular_profile(
        center,
        inner_radius,
        outer_radius,
        height,
        turns,
        phase_degrees,
        budget,
    )?;
    let patches = if fit.curve.control_points.len() <= 32 {
        vec![ruled_patch(
            &fit.curve,
            center,
            inner_radius / outer_radius,
        )?]
    } else {
        fit.curve
            .decompose()?
            .iter()
            .map(|span| ruled_patch(span.definition(), center, inner_radius / outer_radius))
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
    fn dense_signed_helicoids_keep_domains_accuracy_and_neighbor_seams() {
        for (inner, turns, height) in [(0., 2., 8.), (1., -2., -8.)] {
            assert!(approximate([3., 4., 5.], inner, 2., height, turns, 31., 1e-4).is_err());
            let fit =
                approximate_patches([3., 4., 5.], inner, 2., height, turns, 31., 1e-4).unwrap();
            assert_eq!(fit.patches.len(), fit.spans);
            assert!(fit.spans > 10);
            for (i, patch) in fit.patches.iter().enumerate() {
                let a = i as f64 / fit.spans as f64;
                let b = (i + 1) as f64 / fit.spans as f64;
                assert_eq!(patch.knots_v[0], a);
                assert_eq!(*patch.knots_v.last().unwrap(), b);
                for t in [0., 0.13, 0.5, 0.87, 1.] {
                    let v = a + (b - a) * t;
                    let theta = 31_f64.to_radians() + std::f64::consts::TAU * turns * v;
                    for u in [0., 0.13, 0.5, 0.87, 1.] {
                        let p = patch.evaluate(u, v).unwrap().point;
                        let r = inner + (2. - inner) * u;
                        assert!(
                            (p[0] - 3. - r * theta.cos()).hypot(p[1] - 4. - r * theta.sin())
                                <= fit.real_arithmetic_error_estimate + 1e-11
                        );
                        assert!((p[2] - 5. - height * v).abs() < 1e-11);
                        if i > 0 && t == 0. {
                            let previous = fit.patches[i - 1].evaluate(u, v).unwrap().point;
                            assert!(p.iter().zip(previous).all(|(x, y)| (x - y).abs() < 1e-11));
                        }
                    }
                }
            }
            assert!(approximate_patches([0.; 3], inner, 2., height, turns, 0., 1e-12).is_err());
        }
        assert_eq!(
            approximate_patches([0.; 3], 0., 2., 1., 0.1, 0., 1e-3)
                .unwrap()
                .patches
                .len(),
            1
        );
    }
    #[test]
    fn independent_angle_height_equation_for_axis_and_annular_signed_helicoids() {
        for (inner, turns, height) in [(0., 0.25, 3.), (1., -0.5, -4.)] {
            let a = approximate([3., 4., 5.], inner, 2., height, turns, 31., 1e-4).unwrap();
            for i in 0..=200 {
                let v = i as f64 / 200.;
                let theta = 31_f64.to_radians() + std::f64::consts::TAU * turns * v;
                for u in [0., 0.13, 0.5, 0.87, 1.] {
                    let r = inner + (2. - inner) * u;
                    let p = a.surface.evaluate(u, v).unwrap().point;
                    let e = (p[0] - 3. - r * theta.cos()).hypot(p[1] - 4. - r * theta.sin());
                    assert!(e <= a.real_arithmetic_error_estimate + 1e-12);
                    assert!((p[2] - 5. - height * v).abs() < 1e-12);
                }
            }
        }
    }
    #[test]
    fn refuses_degenerate_or_over_budget_surfaces() {
        for (inner, outer, height, turns, budget) in [
            (2., 2., 3., 1., 1e-4),
            (-1., 2., 3., 1., 1e-4),
            (0., 2., 0., 1., 1e-4),
            (0., 2., 3., 0., 1e-4),
            (0., 2., 3., 2., 1e-10),
        ] {
            assert!(approximate([0.; 3], inner, outer, height, turns, 0., budget).is_err());
        }
    }
}
