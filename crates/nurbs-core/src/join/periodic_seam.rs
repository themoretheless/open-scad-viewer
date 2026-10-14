//! Periodic seam rotation at an existing authored knot, without refitting.
use crate::{Result, check, curve::Curve};
#[path = "periodic_seam_verification.rs"]
mod verification;
pub use verification::{CurveReport, Verification, curve_report, verify_curve};
#[path = "periodic_surface_verification.rs"]
mod surface_verification;
pub use surface_verification::{SurfaceReport, surface_report, verify_surface};
/// Rotate periodic controls and basis at an existing active knot. Output domain
/// is [parameter,parameter+period]. Arbitrary non-knot seams require refinement.
pub fn curve_at_knot(source: &Curve, parameter: f64) -> Result<Curve> {
    source.validate()?;
    check(source.periodic, "Seam rotation requires periodic storage")?;
    let [a, b] = source.domain();
    check(
        parameter.is_finite() && parameter >= a && parameter < b,
        "Seam knot must lie in the half-open active domain",
    )?;
    let p = source.degree;
    let period_controls = source.control_points.len() - p;
    let period = b - a;
    check(
        (0..source.knots.len() - period_controls)
            .all(|i| source.knots[i + period_controls] == source.knots[i] + period),
        "Seam rotation requires exact authored periodic knot repetitions",
    )?;
    rotate_validated(source, parameter)
}

fn rotate_validated(source: &Curve, parameter: f64) -> Result<Curve> {
    let p = source.degree;
    let period_controls = source.control_points.len() - p;
    let [a, b] = source.domain();
    let period = b - a;
    let index = (p..p + period_controls)
        .find(|&i| source.knots[i] == parameter)
        .ok_or_else(|| crate::input("Seam parameter must be an existing knot"))?;
    let shift = index - p;
    let mut result = source.clone();
    for i in 0..result.control_points.len() {
        result.control_points[i] = source.control_points[(i + shift) % period_controls].clone();
        result.weights[i] = source.weights[(i + shift) % period_controls];
    }
    for i in 0..result.knots.len() {
        let k = i + shift;
        result.knots[i] = source.knots[k % period_controls] + (k / period_controls) as f64 * period;
    }
    result.validate()?;
    Ok(result)
}

/// Arbitrary seam candidate using cyclic homogeneous Boehm insertion.
/// No refit or sampling is used. Geometry is preserved in exact arithmetic,
/// but binary64 rounding has NOT received a continuous deviation certificate.
/// Callers must not treat this candidate as tolerance-certified geometry.
pub fn curve_candidate(source: &Curve, parameter: f64) -> Result<Curve> {
    source.validate()?;
    check(source.periodic, "Seam refinement requires periodic storage")?;
    let [a, b] = source.domain();
    check(
        parameter.is_finite() && a <= parameter && parameter < b,
        "Seam parameter must lie in the half-open active domain",
    )?;
    if source.knots.iter().any(|&k| k == parameter) {
        return curve_at_knot(source, parameter);
    }
    let p = source.degree;
    let span = (p..source.control_points.len())
        .find(|&i| source.knots[i] < parameter && parameter < source.knots[i + 1])
        .ok_or_else(|| crate::input("No span contains the seam parameter"))?;
    // Rotation puts the insertion in the first active span. Boehm's affected
    // cyclic controls stay inside the stored cycle without wrap ambiguity.
    let base = curve_at_knot(source, source.knots[span])?;
    let n = base.control_points.len() - p;
    check(
        base.control_points.len() < 256,
        "Periodic seam refinement exceeds 256 controls",
    )?;
    let period = base.domain()[1] - base.domain()[0];
    let mut active = base.knots[p..p + n].to_vec();
    let k = (p..base.control_points.len())
        .find(|&i| base.knots[i] < parameter && parameter < base.knots[i + 1])
        .ok_or_else(|| crate::input("No rotated span contains the seam"))?;
    active.insert(k - p + 1, parameter);
    let cycle = n + 1;
    let knots = (0..cycle + 2 * p + 1)
        .map(|i| {
            let j = i as isize - p as isize;
            active[j.rem_euclid(cycle as isize) as usize]
                + j.div_euclid(cycle as isize) as f64 * period
        })
        .collect();
    let mut points = Vec::with_capacity(cycle + p);
    let mut weights = Vec::with_capacity(cycle + p);
    for i in 0..=k - p {
        points.push(base.control_points[i].clone());
        weights.push(base.weights[i]);
    }
    for i in k - p + 1..=k {
        let denominator = base.knots[i + p] - base.knots[i];
        check(
            denominator > 0.,
            "Periodic insertion has a collapsed basis denominator",
        )?;
        let alpha = (parameter - base.knots[i]) / denominator;
        check(
            alpha.is_finite() && (0. ..=1.).contains(&alpha),
            "Periodic insertion coefficient is outside its basis support",
        )?;
        let left = (1. - alpha) * base.weights[i - 1];
        let right = alpha * base.weights[i];
        let weight = left + right;
        let point = base.control_points[i - 1]
            .iter()
            .zip(&base.control_points[i])
            .map(|(&x, &y)| (left * x + right * y) / weight)
            .collect();
        points.push(point);
        weights.push(weight);
    }
    for i in k + 1..cycle {
        points.push(base.control_points[i - 1].clone());
        weights.push(base.weights[i - 1]);
    }
    for i in 0..p {
        points.push(points[i].clone());
        weights.push(weights[i]);
    }
    let refined = Curve {
        degree: p,
        knots,
        control_points: points,
        weights,
        periodic: true,
    };
    refined.validate()?;
    rotate_validated(&refined, parameter)
}

/// Policy for automatic periodic seam placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamPolicy {
    /// Put the seam at the highest-curvature point, where meshing density is
    /// needed anyway. Falls back to `LongestSegment` when the curvature
    /// report is degenerate (e.g. constant-curvature circles).
    MaxCurvature,
    /// Put the seam at the midpoint of the longest knot segment.
    LongestSegment,
}

/// Outcome of `relocate_seam_auto`.
pub struct SeamRelocationReport {
    /// The curve with the relocated seam; domain is [seam, seam + period].
    pub curve: Curve,
    /// The chosen seam parameter in the SOURCE curve's parameterization.
    pub seam: f64,
    /// True iff the move used the certified existing-knot rotation
    /// (`curve_at_knot`); false means the uncertified-rounding cyclic Boehm
    /// candidate path (`curve_candidate`) was used.
    pub certified: bool,
}

/// Chooses a new seam parameter for a periodic curve per policy and relocates
/// via the existing certified rotation (existing knot) or the cyclic Boehm
/// candidate path (interior of a span). Geometry is preserved; only the
/// certified flag distinguishes the rounding contract.
pub fn relocate_seam_auto(curve: &Curve, policy: SeamPolicy) -> Result<SeamRelocationReport> {
    curve.validate()?;
    check(curve.periodic, "Seam relocation requires periodic storage")?;
    let [a, b] = curve.domain();
    let period = b - a;
    check(
        period.is_finite() && period > 0.,
        "Periodic seam relocation needs a positive active period",
    )?;
    let longest_segment = || -> f64 {
        // Widest span of the active knot sequence; seam at its midpoint.
        let p = curve.degree;
        let n = curve.control_points.len();
        let mut best = (0., a); // (width, midpoint)
        for i in p..n {
            let (lo, hi) = (curve.knots[i], curve.knots[i + 1]);
            if hi - lo > best.0 {
                best = (hi - lo, (lo + hi) * 0.5);
            }
        }
        best.1
    };
    let seam = match policy {
        SeamPolicy::LongestSegment => longest_segment(),
        SeamPolicy::MaxCurvature => {
            let extrema = crate::curve_analysis::curvature_extrema(curve, 1e-9)?;
            extrema
                .iter()
                .filter(|e| {
                    e.kind == crate::curve_analysis::ExtremumKind::LocalMax
                        && e.kappa[0].is_finite()
                })
                .max_by(|x, y| x.kappa[0].total_cmp(&y.kappa[0]))
                .map(|e| {
                    // Clamp into the half-open active domain.
                    if e.u >= b {
                        a
                    } else {
                        e.u.max(a)
                    }
                })
                // Degenerate report (e.g. constant curvature): fall back.
                .unwrap_or_else(longest_segment)
        }
    };
    check(
        seam.is_finite() && seam >= a && seam < b,
        "Automatic seam selection left the active domain",
    )?;
    // The certified path applies exactly when the seam is an authored knot.
    let certified = curve.knots.iter().any(|&k| k == seam);
    let rotated = if certified {
        curve_at_knot(curve, seam)?
    } else {
        curve_candidate(curve, seam)?
    };
    Ok(SeamRelocationReport {
        curve: rotated,
        seam,
        certified,
    })
}

/// Arbitrary seam candidate for all curves of a periodic surface axis.
/// Same uncertified-rounding contract as curve_candidate; the result must still
/// fit the surface's 32x32 control-net limit.
pub fn surface_candidate(
    source: &crate::surface::Surface,
    axis: crate::surface::Axis,
    parameter: f64,
) -> Result<crate::surface::Surface> {
    source.validate()?;
    source.edit_axis(axis, |c| curve_candidate(c, parameter))
}

/// Apply an existing-knot seam rotation to all curves of a periodic surface axis.
pub fn surface_at_knot(
    source: &crate::surface::Surface,
    axis: crate::surface::Axis,
    parameter: f64,
) -> Result<crate::surface::Surface> {
    source.validate()?;
    source.edit_axis(axis, |c| curve_at_knot(c, parameter))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Periodic degree-3 closed curve: `unique` control points on a radial
    /// profile r(angle) over [0, count) with wrapped periodic storage.
    fn periodic_loop(count: usize, radius: impl Fn(f64) -> f64) -> Curve {
        let active: Vec<f64> = (0..=count).map(|i| i as f64).collect();
        let knots = crate::foundation::periodic_knots(&active, 3);
        let mut control_points: Vec<Vec<f64>> = (0..count)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / count as f64;
                let r = radius(angle);
                vec![r * angle.cos(), r * angle.sin()]
            })
            .collect();
        let tail: Vec<Vec<f64>> = control_points[..3].to_vec();
        control_points.extend(tail);
        let curve = Curve {
            degree: 3,
            knots,
            control_points,
            weights: vec![1.; count + 3],
            periodic: true,
        };
        curve.validate().unwrap();
        curve
    }

    /// Assert both curves sample to the same geometry. Seam rotation shifts the
    /// domain window to [seam, seam+period] but keeps the parameterization:
    /// moved(u) = source(a + (u - a) rem period).
    fn assert_same_geometry(source: &Curve, moved: &Curve, seam: f64, tolerance: f64) {
        let [a, b] = source.domain();
        let [na, nb] = moved.domain();
        assert!((na - seam).abs() <= 1e-9, "new domain starts at the seam");
        assert!(((nb - na) - (b - a)).abs() <= 1e-9, "period preserved");
        let period = b - a;
        for i in 0..=64 {
            let u = na + (nb - na) * i as f64 / 64.;
            let v = if u >= nb {
                seam
            } else {
                a + (u - a).rem_euclid(period)
            };
            let p = source.evaluate(v.min(b)).unwrap().point;
            let q = moved.evaluate(u.min(nb)).unwrap().point;
            let distance: f64 = p
                .iter()
                .zip(&q)
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt();
            assert!(
                distance <= tolerance,
                "geometry drifted {distance} at u={u} (source v={v})"
            );
        }
    }

    #[test]
    fn periodic_circle_relocation_preserves_sampled_geometry() {
        // Uniform knots: LongestSegment picks the first widest span midpoint
        // (u = 0.5), a non-knot parameter exercising the cyclic Boehm
        // candidate path.
        let circle = periodic_loop(8, |_| 2.);
        let report = relocate_seam_auto(&circle, SeamPolicy::LongestSegment).unwrap();
        assert!(
            (report.seam - 0.5).abs() < 1e-12,
            "uniform midspan seam should be u=0.5, got {}",
            report.seam
        );
        assert!(!report.certified, "midspan seam uses the candidate path");
        assert_same_geometry(&circle, &report.curve, report.seam, 1e-9);
    }

    #[test]
    fn max_curvature_picks_the_global_kappa_maximum() {
        // Wobbled loop; the global κ maximum found by `curvature_extrema`
        // sits at an authored knot (u = 0/6 tie at 0.608664…), so the move
        // must use the certified rotation.
        let wobbled = periodic_loop(8, |angle| 2. + 0.6 * angle.cos());
        let extrema = crate::curve_analysis::curvature_extrema(&wobbled, 1e-9).unwrap();
        let expected = extrema
            .iter()
            .filter(|e| e.kind == crate::curve_analysis::ExtremumKind::LocalMax)
            .max_by(|x, y| x.kappa[0].total_cmp(&y.kappa[0]))
            .expect("wobbled loop has κ maxima");
        let report = relocate_seam_auto(&wobbled, SeamPolicy::MaxCurvature).unwrap();
        assert!(
            (report.seam - expected.u).abs() < 1e-6,
            "seam should be the κ-max parameter {}, got {}",
            expected.u,
            report.seam
        );
        assert!(
            report.certified,
            "κ maximum at an authored knot must use the certified rotation"
        );
        assert_same_geometry(&wobbled, &report.curve, report.seam, 1e-12);
    }

    #[test]
    fn longest_segment_picks_the_midspan() {
        // Nonuniform active knots: widest span is [1, 4], midpoint u = 2.5.
        let active = [0., 0.5, 1., 4., 5., 6., 7., 8.];
        let knots = crate::foundation::periodic_knots(&active, 3);
        let count = active.len() - 1;
        let mut control_points: Vec<Vec<f64>> = (0..count)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / count as f64;
                vec![2. * angle.cos(), 2. * angle.sin()]
            })
            .collect();
        let tail: Vec<Vec<f64>> = control_points[..3].to_vec();
        control_points.extend(tail);
        let curve = Curve {
            degree: 3,
            knots,
            control_points,
            weights: vec![1.; count + 3],
            periodic: true,
        };
        curve.validate().unwrap();
        let report = relocate_seam_auto(&curve, SeamPolicy::LongestSegment).unwrap();
        assert!(
            (report.seam - 2.5).abs() < 1e-12,
            "seam should be the midspan u=2.5, got {}",
            report.seam
        );
        assert!(!report.certified, "2.5 is not an authored knot");
        assert_same_geometry(&curve, &report.curve, report.seam, 1e-9);
    }

    #[test]
    fn non_periodic_curve_is_rejected() {
        let clamped = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 2., 2., 2.],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![2., 0.], vec![3., 1.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        clamped.validate().unwrap();
        for policy in [SeamPolicy::MaxCurvature, SeamPolicy::LongestSegment] {
            let Err(error) = relocate_seam_auto(&clamped, policy) else {
                panic!("non-periodic curve must be rejected")
            };
            assert!(
                error.to_string().contains("periodic"),
                "unexpected error: {error}"
            );
        }
    }
}
