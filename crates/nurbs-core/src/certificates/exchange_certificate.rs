//! Round-trip exchange certificates: verify that a curve or surface survived
//! an export/reimport cycle (or any other transport) and report exactly what
//! was preserved and what was lost.
//!
//! Comparison model:
//! * Knot vectors are compared after affine normalization of the active
//!   domain to `[0, 1]` (via [`crate::curve::knot_ops::normalize_knots`] for
//!   curves, the same rescale per axis for surfaces), so a linear
//!   reparameterization does not count as a knot change.
//! * The linear reparameterization `u' = s·u + t` between the two active
//!   domains is estimated from the domain endpoints; `param_shift` reports
//!   the constant term `t` (zero when both domains start at the same value).
//! * `same_param_deviation` is the maximum point distance when the
//!   reimported geometry is evaluated at the reparameterized parameters —
//!   it vanishes for pure reparameterizations.
//! * `max_deviation` is a symmetric sampled pseudo-Hausdorff estimate:
//!   samples of each geometry are measured against the nearest sample of
//!   the other (64 samples for curves, 16×16 for surfaces). It is a
//!   sampling-based diagnostic, not a certified bound.
//! * `lost` lists every property that failed to round-trip within `tol`:
//!   degree, knot values/multiplicities, control points, weights, period
//!   flags.
use crate::{
    Result, check,
    curve::Curve,
    curve::normalize_knots,
    foundation::guards::{Fnv1a, require_finite_f64},
    numeric,
    surface::Surface,
};

/// Deterministic content digest of a curve for transport/round-trip
/// determinism checks (item 1094): FNV-1a over the canonical bits of degree,
/// knots, weights, control points and the periodic flag. Canonicalization
/// collapses `-0.0` into `+0.0` and all NaN payloads into one pattern, so
/// logically identical curves digest identically regardless of how the
/// values were produced.
pub fn curve_digest(curve: &Curve) -> Result<u64> {
    curve.validate()?;
    let mut h = Fnv1a::new();
    h.write(&(curve.degree as u64).to_le_bytes());
    h.write(&[u8::from(curve.periodic)]);
    for &k in &curve.knots {
        h.write_f64(k);
    }
    for &w in &curve.weights {
        h.write_f64(w);
    }
    for p in &curve.control_points {
        for &c in p {
            h.write_f64(c);
        }
    }
    Ok(h.finish())
}

/// Deterministic content digest of a surface, same construction as
/// [`curve_digest`]: degrees, both knot vectors, weights, control net
/// (row-major) and both periodic flags.
pub fn surface_digest(surface: &Surface) -> Result<u64> {
    surface.validate()?;
    let mut h = Fnv1a::new();
    h.write(&(surface.degree_u as u64).to_le_bytes());
    h.write(&(surface.degree_v as u64).to_le_bytes());
    h.write(&[u8::from(surface.periodic_u), u8::from(surface.periodic_v)]);
    for &k in &surface.knots_u {
        h.write_f64(k);
    }
    for &k in &surface.knots_v {
        h.write_f64(k);
    }
    for row in &surface.weights {
        for &w in row {
            h.write_f64(w);
        }
    }
    for row in &surface.control_points {
        for p in row {
            for &c in p {
                h.write_f64(c);
            }
        }
    }
    Ok(h.finish())
}

/// Natural active domain of one surface axis: `[knots[degree], knots[count]]`.
fn axis_domain(knots: &[f64], degree: usize, count: usize) -> [f64; 2] {
    [knots[degree], knots[count]]
}

/// Exchange verification report for a curve round-trip.
#[derive(Clone, Debug)]
pub struct CurveExchangeReport {
    pub knot_vector_preserved: bool,
    pub degree_preserved: bool,
    /// Constant term of the estimated linear reparameterization between the
    /// active domains (0 when the parameter origins coincide).
    pub param_shift: f64,
    /// Symmetric sampled pseudo-Hausdorff deviation estimate.
    pub max_deviation: f64,
    /// Max point distance at corresponding (reparameterized) parameters.
    pub same_param_deviation: f64,
    /// Human-readable list of properties lost in the exchange.
    pub lost: Vec<String>,
}

/// Exchange verification report for a surface round-trip.
#[derive(Clone, Debug)]
pub struct SurfaceExchangeReport {
    pub knot_vector_preserved: bool,
    pub degree_preserved: bool,
    /// Largest per-axis constant reparameterization term (u and v).
    pub param_shift: f64,
    /// Symmetric sampled pseudo-Hausdorff deviation estimate.
    pub max_deviation: f64,
    /// Max point distance at corresponding (reparameterized) parameters.
    pub same_param_deviation: f64,
    /// Human-readable list of properties lost in the exchange.
    pub lost: Vec<String>,
}

const CURVE_SAMPLES: usize = 64;
const SURFACE_SAMPLES: usize = 16;

/// Linear map between two domains: returns `(scale, shift)` with
/// `u' = scale·u + shift`.
fn domain_map(from: [f64; 2], to: [f64; 2]) -> Result<(f64, f64)> {
    numeric(
        from[1] > from[0] && to[1] > to[0],
        "Exchange domains must have positive length",
    )?;
    let scale = (to[1] - to[0]) / (from[1] - from[0]);
    Ok((scale, to[0] - scale * from[0]))
}

/// Affine-rescale a knot vector so `[knots[first], knots[last]]` maps to
/// `[0, 1]`; returns `None` for a degenerate span.
fn normalize_axis(knots: &[f64], domain: [f64; 2]) -> Option<Vec<f64>> {
    let length = domain[1] - domain[0];
    if length <= 0. {
        return None;
    }
    Some(knots.iter().map(|k| (k - domain[0]) / length).collect())
}

fn knots_close(a: &[f64], b: &[f64], tol: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
}

fn flatten3(points: &[Vec<f64>]) -> Option<Vec<[f64; 3]>> {
    points
        .iter()
        .map(|p| {
            (p.len() == 3).then_some([p[0], p[1], p[2]])
        })
        .collect()
}

fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1]) + (a[2] - b[2]) * (a[2] - b[2]))
        .sqrt()
}

/// Verify that `reimported` is the same geometric curve as `original`
/// within `tol`, allowing a linear reparameterization.
pub fn verify_curve_roundtrip(
    original: &Curve,
    reimported: &Curve,
    tol: f64,
) -> Result<CurveExchangeReport> {
    original.validate()?;
    reimported.validate()?;
    check(
        tol.is_finite() && tol > 0. && tol <= 1.,
        "Exchange tolerance must be in (0, 1]",
    )?;
    check(
        original.control_points[0].len() == 3 && reimported.control_points[0].len() == 3,
        "Exchange verification expects 3D curves",
    )?;
    let mut lost = Vec::new();
    let degree_preserved = original.degree == reimported.degree;
    if !degree_preserved {
        lost.push(format!(
            "degree changed from {} to {}",
            original.degree, reimported.degree
        ));
    }
    if original.periodic != reimported.periodic {
        lost.push("periodic flag changed".to_string());
    }
    let da = original.domain();
    let db = reimported.domain();
    let (scale, shift) = domain_map(da, db)?;
    let param_shift = shift;
    // Normalized knot comparison (linear reparameterizations cancel out).
    let knot_vector_preserved = match (normalize_knots(original), normalize_knots(reimported)) {
        (Ok(a), Ok(b)) => knots_close(&a.knots, &b.knots, tol),
        _ => false,
    };
    if !knot_vector_preserved {
        lost.push("knot values or multiplicities changed beyond tolerance".to_string());
    }
    // Weight comparison (invariant under reparameterization).
    let weight_diff = if original.weights.len() == reimported.weights.len() {
        original
            .weights
            .iter()
            .zip(&reimported.weights)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max)
    } else {
        f64::INFINITY
    };
    if weight_diff > tol {
        lost.push("weights changed beyond tolerance".to_string());
    }
    // Control-point comparison when the discretization survived.
    if degree_preserved
        && knot_vector_preserved
        && original.control_points.len() == reimported.control_points.len()
    {
        let diff = original
            .control_points
            .iter()
            .zip(&reimported.control_points)
            .map(|(a, b)| {
                a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0., f64::max)
            })
            .fold(0., f64::max);
        if diff > tol {
            lost.push("control points changed beyond tolerance".to_string());
        }
    } else if original.control_points.len() != reimported.control_points.len() {
        lost.push(format!(
            "control point count changed from {} to {}",
            original.control_points.len(),
            reimported.control_points.len()
        ));
    }
    // Same-parameter deviation: C2(s·u + t) vs C1(u).
    let mut same_param = 0_f64;
    for i in 0..CURVE_SAMPLES {
        let u = da[0] + (da[1] - da[0]) * i as f64 / (CURVE_SAMPLES - 1) as f64;
        let a = flatten3(&[original.evaluate(u)?.point]).unwrap()[0];
        let b = flatten3(&[reimported.evaluate(scale * u + shift)?.point]).unwrap()[0];
        same_param = same_param.max(dist3(a, b));
    }
    // Symmetric sampled pseudo-Hausdorff deviation.
    let samples_a = sample_curve(original, da, CURVE_SAMPLES)?;
    let samples_b = sample_curve(reimported, db, CURVE_SAMPLES)?;
    let max_deviation = pseudo_hausdorff(&samples_a, &samples_b);
    if max_deviation > tol {
        lost.push("geometry deviates beyond tolerance".to_string());
    }
    // Метрики отчёта обязаны быть конечными: NaN в отчёте — баг.
    require_finite_f64(param_shift, "exchange param_shift")?;
    require_finite_f64(max_deviation, "exchange max_deviation")?;
    require_finite_f64(same_param, "exchange same_param_deviation")?;
    Ok(CurveExchangeReport {
        knot_vector_preserved,
        degree_preserved,
        param_shift,
        max_deviation,
        same_param_deviation: same_param,
        lost,
    })
}

fn sample_curve(curve: &Curve, domain: [f64; 2], count: usize) -> Result<Vec<[f64; 3]>> {
    (0..count)
        .map(|i| {
            let u = domain[0] + (domain[1] - domain[0]) * i as f64 / (count - 1) as f64;
            Ok(flatten3(&[curve.evaluate(u)?.point]).unwrap()[0])
        })
        .collect()
}

fn pseudo_hausdorff(a: &[[f64; 3]], b: &[[f64; 3]]) -> f64 {
    let one_sided = |from: &[[f64; 3]], to: &[[f64; 3]]| {
        from.iter()
            .map(|p| to.iter().map(|q| dist3(*p, *q)).fold(f64::INFINITY, f64::min))
            .fold(0., f64::max)
    };
    one_sided(a, b).max(one_sided(b, a))
}

/// Verify that `reimported` is the same geometric surface as `original`
/// within `tol`, allowing independent linear reparameterizations per axis.
pub fn verify_surface_roundtrip(
    original: &Surface,
    reimported: &Surface,
    tol: f64,
) -> Result<SurfaceExchangeReport> {
    original.validate()?;
    reimported.validate()?;
    check(
        tol.is_finite() && tol > 0. && tol <= 1.,
        "Exchange tolerance must be in (0, 1]",
    )?;
    check(
        original.control_points[0][0].len() == 3 && reimported.control_points[0][0].len() == 3,
        "Exchange verification expects 3D surfaces",
    )?;
    let mut lost = Vec::new();
    let degree_preserved = original.degree_u == reimported.degree_u
        && original.degree_v == reimported.degree_v;
    if !degree_preserved {
        lost.push(format!(
            "degrees changed from ({}, {}) to ({}, {})",
            original.degree_u, original.degree_v, reimported.degree_u, reimported.degree_v
        ));
    }
    if original.periodic_u != reimported.periodic_u || original.periodic_v != reimported.periodic_v
    {
        lost.push("periodic flags changed".to_string());
    }
    let (ua, ub) = (
        axis_domain(&original.knots_u, original.degree_u, original.control_points.len()),
        axis_domain(
            &reimported.knots_u,
            reimported.degree_u,
            reimported.control_points.len(),
        ),
    );
    let (va, vb) = (
        axis_domain(
            &original.knots_v,
            original.degree_v,
            original.control_points[0].len(),
        ),
        axis_domain(
            &reimported.knots_v,
            reimported.degree_v,
            reimported.control_points[0].len(),
        ),
    );
    let (su, tu) = domain_map(ua, ub)?;
    let (sv, tv) = domain_map(va, vb)?;
    let param_shift = if tu.abs() >= tv.abs() { tu } else { tv };
    let knot_vector_preserved = match (
        normalize_axis(&original.knots_u, ua),
        normalize_axis(&reimported.knots_u, ub),
        normalize_axis(&original.knots_v, va),
        normalize_axis(&reimported.knots_v, vb),
    ) {
        (Some(a1), Some(b1), Some(a2), Some(b2)) => {
            knots_close(&a1, &b1, tol) && knots_close(&a2, &b2, tol)
        }
        _ => false,
    };
    if !knot_vector_preserved {
        lost.push("knot values or multiplicities changed beyond tolerance".to_string());
    }
    let weight_diff = if original.weights.len() == reimported.weights.len()
        && original.weights[0].len() == reimported.weights[0].len()
    {
        original
            .weights
            .iter()
            .flatten()
            .zip(reimported.weights.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max)
    } else {
        f64::INFINITY
    };
    if weight_diff > tol {
        lost.push("weights changed beyond tolerance".to_string());
    }
    if original.control_points.len() != reimported.control_points.len()
        || original.control_points[0].len() != reimported.control_points[0].len()
    {
        lost.push("control grid dimensions changed".to_string());
    } else if degree_preserved && knot_vector_preserved {
        let diff = original
            .control_points
            .iter()
            .flatten()
            .zip(reimported.control_points.iter().flatten())
            .map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0., f64::max))
            .fold(0., f64::max);
        if diff > tol {
            lost.push("control points changed beyond tolerance".to_string());
        }
    }
    // Same-parameter deviation on a grid.
    let mut same_param = 0_f64;
    for i in 0..SURFACE_SAMPLES {
        for j in 0..SURFACE_SAMPLES {
            let u = ua[0] + (ua[1] - ua[0]) * i as f64 / (SURFACE_SAMPLES - 1) as f64;
            let v = va[0] + (va[1] - va[0]) * j as f64 / (SURFACE_SAMPLES - 1) as f64;
            let a: [f64; 3] = original.evaluate(u, v)?.point;
            let b: [f64; 3] = reimported.evaluate(su * u + tu, sv * v + tv)?.point;
            same_param = same_param.max(dist3(a, b));
        }
    }
    let samples_a = sample_surface(original, ua, va)?;
    let samples_b = sample_surface(reimported, ub, vb)?;
    let max_deviation = pseudo_hausdorff(&samples_a, &samples_b);
    if max_deviation > tol {
        lost.push("geometry deviates beyond tolerance".to_string());
    }
    // Метрики отчёта обязаны быть конечными: NaN в отчёте — баг.
    require_finite_f64(param_shift, "exchange param_shift")?;
    require_finite_f64(max_deviation, "exchange max_deviation")?;
    require_finite_f64(same_param, "exchange same_param_deviation")?;
    Ok(SurfaceExchangeReport {
        knot_vector_preserved,
        degree_preserved,
        param_shift,
        max_deviation,
        same_param_deviation: same_param,
        lost,
    })
}

fn sample_surface(
    surface: &Surface,
    domain_u: [f64; 2],
    domain_v: [f64; 2],
) -> Result<Vec<[f64; 3]>> {
    let mut out = Vec::with_capacity(SURFACE_SAMPLES * SURFACE_SAMPLES);
    for i in 0..SURFACE_SAMPLES {
        for j in 0..SURFACE_SAMPLES {
            let u = domain_u[0] + (domain_u[1] - domain_u[0]) * i as f64 / (SURFACE_SAMPLES - 1) as f64;
            let v = domain_v[0] + (domain_v[1] - domain_v[0]) * j as f64 / (SURFACE_SAMPLES - 1) as f64;
            out.push(surface.evaluate(u, v)?.point);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_curve() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.],
                vec![2., -1., 1.],
                vec![3., 1., 0.],
                vec![4., 0., 2.],
            ],
            weights: vec![1., 0.8, 1.2, 1., 0.9],
            periodic: false,
        }
    }

    fn test_surface() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.5]],
                vec![vec![1., 0., 1.], vec![1., 1., 0.]],
                vec![vec![2., 0., 0.], vec![2., 1., 1.]],
            ],
            weights: vec![vec![1., 1.], vec![0.7, 1.], vec![1., 1.3]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn identical_curve_roundtrips_cleanly() {
        let curve = test_curve();
        let report = verify_curve_roundtrip(&curve, &curve.clone(), 1e-9).unwrap();
        assert!(report.knot_vector_preserved);
        assert!(report.degree_preserved);
        assert_eq!(report.param_shift, 0.);
        assert!(report.max_deviation <= 1e-12);
        assert!(report.same_param_deviation <= 1e-12);
        assert!(report.lost.is_empty(), "lost: {:?}", report.lost);
    }

    #[test]
    fn reparameterized_curve_reports_shift_not_loss() {
        let original = test_curve();
        // Affine knot rescale: domain [0,1] -> [2,5]; identical geometry.
        let mut moved = original.clone();
        for k in &mut moved.knots {
            *k = 2. + 3. * *k;
        }
        let report = verify_curve_roundtrip(&original, &moved, 1e-9).unwrap();
        assert!(report.knot_vector_preserved);
        assert!(report.degree_preserved);
        assert!((report.param_shift - 2.).abs() < 1e-12);
        assert!(report.same_param_deviation <= 1e-9);
        assert!(report.max_deviation <= 1e-9);
        assert!(report.lost.is_empty(), "lost: {:?}", report.lost);
    }

    #[test]
    fn distorted_curves_are_diagnosed() {
        let original = test_curve();
        // Weight change beyond tolerance.
        let mut weighted = original.clone();
        weighted.weights[2] = 2.5;
        let report = verify_curve_roundtrip(&original, &weighted, 1e-9).unwrap();
        assert!(report.lost.iter().any(|l| l.contains("weights")));
        assert!(report.max_deviation > 1e-3);
        // Knot multiplicity change.
        let mut knotted = original.clone();
        knotted.knots[4] = 0.7;
        let report = verify_curve_roundtrip(&original, &knotted, 1e-9).unwrap();
        assert!(!report.knot_vector_preserved);
        assert!(report.lost.iter().any(|l| l.contains("knot")));
        // Degree change.
        let mut degraded = original.clone();
        degraded.degree = 2;
        degraded.knots = vec![0., 0., 0., 1., 1., 1.];
        degraded.control_points.truncate(3);
        degraded.weights.truncate(3);
        let report = verify_curve_roundtrip(&original, &degraded, 1e-9).unwrap();
        assert!(!report.degree_preserved);
        assert!(report.lost.iter().any(|l| l.contains("degree")));
        // Bad tolerance is rejected.
        assert!(verify_curve_roundtrip(&original, &original.clone(), 0.).is_err());
    }

    #[test]
    fn identical_surface_roundtrips_cleanly() {
        let surface = test_surface();
        let report = verify_surface_roundtrip(&surface, &surface.clone(), 1e-9).unwrap();
        assert!(report.knot_vector_preserved);
        assert!(report.degree_preserved);
        assert_eq!(report.param_shift, 0.);
        assert!(report.max_deviation <= 1e-12);
        assert!(report.same_param_deviation <= 1e-12);
        assert!(report.lost.is_empty(), "lost: {:?}", report.lost);
    }

    #[test]
    fn digests_are_deterministic_and_canonical() {
        let curve = test_curve();
        // Детерминизм: два прогона одного объекта дают один дайджест.
        assert_eq!(curve_digest(&curve).unwrap(), curve_digest(&curve).unwrap());
        // Канонизация -0.0: логически та же кривая, тот же дайджест.
        let mut neg_zero = curve.clone();
        neg_zero.control_points[0][0] = -0.0;
        assert_eq!(curve_digest(&curve).unwrap(), curve_digest(&neg_zero).unwrap());
        // Иные узлы/веса/точки — иной дайджест.
        let mut moved = curve.clone();
        moved.knots[4] = 0.6;
        assert_ne!(curve_digest(&curve).unwrap(), curve_digest(&moved).unwrap());
        let mut weighted = curve.clone();
        weighted.weights[1] = 0.81;
        assert_ne!(curve_digest(&curve).unwrap(), curve_digest(&weighted).unwrap());
        let surface = test_surface();
        assert_eq!(surface_digest(&surface).unwrap(), surface_digest(&surface).unwrap());
        let mut s2 = surface.clone();
        s2.control_points[1][1][2] += 1e-9;
        assert_ne!(surface_digest(&surface).unwrap(), surface_digest(&s2).unwrap());
        // Дайджест кривой и поверхности не пересекаются конструктивно.
        assert_ne!(curve_digest(&curve).unwrap(), surface_digest(&surface).unwrap());
    }

    #[test]
    fn digests_reject_invalid_geometry() {
        let mut curve = test_curve();
        curve.weights[0] = f64::NAN;
        assert!(curve_digest(&curve).is_err());
    }

    #[test]
    fn distorted_surfaces_are_diagnosed() {
        let original = test_surface();
        // Reparameterized u axis only.
        let mut moved = original.clone();
        for k in &mut moved.knots_u {
            *k = 1. + 2. * *k;
        }
        let report = verify_surface_roundtrip(&original, &moved, 1e-9).unwrap();
        assert!(report.knot_vector_preserved);
        assert!((report.param_shift - 1.).abs() < 1e-12);
        assert!(report.same_param_deviation <= 1e-9);
        assert!(report.lost.is_empty(), "lost: {:?}", report.lost);
        // Knot change.
        let mut knotted = original.clone();
        knotted.knots_v[1] = 0.25;
        let report = verify_surface_roundtrip(&original, &knotted, 1e-9).unwrap();
        assert!(!report.knot_vector_preserved);
        assert!(report.lost.iter().any(|l| l.contains("knot")));
        // Weight change.
        let mut weighted = original.clone();
        weighted.weights[1][0] = 3.;
        let report = verify_surface_roundtrip(&original, &weighted, 1e-9).unwrap();
        assert!(report.lost.iter().any(|l| l.contains("weights")));
        assert!(report.max_deviation > 1e-3);
    }

    #[cfg(feature = "codec")]
    #[test]
    fn codec_roundtrip_verifies_clean() {
        use value_codec::{Deserialize, Serialize};
        let curve = test_curve();
        let value = curve.to_value();
        let reimported = Curve::from_value(value).unwrap();
        let report = verify_curve_roundtrip(&curve, &reimported, 1e-12).unwrap();
        assert!(report.knot_vector_preserved && report.degree_preserved);
        assert!(report.max_deviation <= 1e-12);
        assert!(report.lost.is_empty(), "lost: {:?}", report.lost);
        let surface = test_surface();
        let value = surface.to_value();
        let reimported = Surface::from_value(value).unwrap();
        let report = verify_surface_roundtrip(&surface, &reimported, 1e-12).unwrap();
        assert!(report.knot_vector_preserved && report.degree_preserved);
        assert!(report.max_deviation <= 1e-12);
        assert!(report.lost.is_empty(), "lost: {:?}", report.lost);
    }
}
