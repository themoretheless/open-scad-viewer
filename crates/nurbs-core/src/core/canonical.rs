//! Canonical curve/surface forms and tolerance-stable hashes.
//!
//! Canonicalization normalizes the representation so that two models of the
//! "same" geometry compare and hash identically:
//!
//! * periodic storage is unwrapped (clamped), dropping exterior knots;
//! * knots are affinely rescaled so the active domain is [0, 1];
//! * near-duplicate knots are merged with [`merge_near_knots`];
//! * (curves) endpoint weights are normalized to w₀ = wₙ = 1 via
//!   [`normalize_endpoint_weights_report`], which also fixes the uniform
//!   weight-scaling freedom; (surfaces) weights are scaled so the corner
//!   weight is exactly 1;
//! * knots, control coordinates and weights are rounded to the tolerance
//!   grid, with domain endpoints pinned to exactly 0 and 1 (and endpoint
//!   weights pinned to exactly 1).
//!
//! The hash is self-contained FNV-1a 64 over the canonical serialization
//! bytes (no external dependencies). It is invariant to:
//!
//! * affine knot rescaling (any active domain maps to [0, 1]);
//! * representation noise below `tolerance` (grid rounding absorbs it);
//! * uniform weight scaling (endpoint/corner normalization removes it);
//! * re-running endpoint-weight normalization or canonicalization itself.
//!
//! It is NOT invariant to: geometry-preserving representation changes that
//! alter control counts (knot insertion, degree elevation, degree changes),
//! trims or splits, orientation reversal, rigid motions or any geometric
//! change above `tolerance`, periodic-vs-clamped semantic differences beyond
//! storage, or a different choice of `tolerance`. Values sitting within half
//! an ulp of a grid boundary may round either way, so inputs must genuinely
//! differ by less than `tolerance` for invariance to be meaningful.
use crate::{
    Result, check,
    curve::{Curve, clamped, merge_near_knots, normalize_knots},
    foundation::parameter_mapping::normalize_endpoint_weights_report,
    numeric,
    surface::{Axis, Surface},
};

/// Round to the tolerance grid; negative zero canonicalizes to positive zero.
fn quantize(value: f64, grid: f64) -> f64 {
    let quantized = (value / grid).round() * grid;
    if quantized == 0. { 0. } else { quantized }
}

fn check_tolerance(tolerance: f64) -> Result<()> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Canonicalization tolerance must be positive and finite",
    )
}

/// Merge epsilon in the normalized [0, 1] parameter coordinate, derived from
/// the geometric tolerance and clamped to a sane window.
fn merge_epsilon(tolerance: f64) -> f64 {
    tolerance.clamp(1e-12, 1e-3)
}

/// Quantize a [0, 1]-domain knot vector in place, pinning the clamped end
/// knots to exactly 0 and 1.
fn quantize_knots(knots: &mut [f64], degree: usize, grid: f64) {
    for knot in knots.iter_mut() {
        *knot = quantize(*knot, grid);
    }
    for knot in knots.iter_mut().take(degree + 1) {
        *knot = 0.;
    }
    for knot in knots.iter_mut().rev().take(degree + 1) {
        *knot = 1.;
    }
}

/// Canonical form of a curve: non-periodic, clamped, knots on [0, 1] with
/// near-duplicates merged, endpoint weights exactly 1, all values rounded to
/// the tolerance grid — stable for compare + hash. See the module docs for
/// the invariance contract.
pub fn canonicalize_curve(curve: &Curve, tolerance: f64) -> Result<Curve> {
    curve.validate()?;
    check_tolerance(tolerance)?;
    let base = clamped(curve)?;
    let normalized = normalize_knots(&base)?;
    let merged = merge_near_knots(&normalized, merge_epsilon(tolerance))?;
    let report = normalize_endpoint_weights_report(&merged)?;
    let mut canonical = report.curve;
    quantize_knots(&mut canonical.knots, canonical.degree, tolerance);
    for weight in &mut canonical.weights {
        *weight = quantize(*weight, tolerance);
    }
    let last = canonical.weights.len() - 1;
    canonical.weights[0] = 1.;
    canonical.weights[last] = 1.;
    for point in &mut canonical.control_points {
        for coordinate in point.iter_mut() {
            *coordinate = quantize(*coordinate, tolerance);
        }
    }
    canonical.validate()?;
    Ok(canonical)
}

/// Canonical form of a surface: both knot axes normalized to [0, 1] and
/// merged, weights scaled so the [0][0] corner weight is exactly 1, all
/// values rounded to the tolerance grid. Periodic storage is unwrapped.
pub fn canonicalize_surface(surface: &Surface, tolerance: f64) -> Result<Surface> {
    surface.validate()?;
    check_tolerance(tolerance)?;
    let mut canonical = surface.clone();
    for axis in [Axis::U, Axis::V] {
        let periodic = match axis {
            Axis::U => canonical.periodic_u,
            Axis::V => canonical.periodic_v,
        };
        if periodic {
            canonical = canonical.edit_axis(axis, clamped)?;
        }
        canonical = canonical.edit_axis(axis, normalize_knots)?;
        canonical = canonical.edit_axis(axis, |curve| {
            merge_near_knots(curve, merge_epsilon(tolerance))
        })?;
    }
    quantize_knots(&mut canonical.knots_u, canonical.degree_u, tolerance);
    quantize_knots(&mut canonical.knots_v, canonical.degree_v, tolerance);
    let corner = canonical.weights[0][0];
    numeric(
        corner.is_finite() && corner > 0.,
        "Surface corner weight must be positive and finite",
    )?;
    for row in &mut canonical.weights {
        for weight in row.iter_mut() {
            *weight = quantize(*weight / corner, tolerance);
        }
    }
    canonical.weights[0][0] = 1.;
    for row in &mut canonical.control_points {
        for point in row.iter_mut() {
            for coordinate in point.iter_mut() {
                *coordinate = quantize(*coordinate, tolerance);
            }
        }
    }
    canonical.validate()?;
    Ok(canonical)
}

/// FNV-1a 64 over a byte stream.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f64(bytes: &mut Vec<u8>, value: f64) {
    // Canonical values are grid-rounded; still normalize any residual -0.0.
    push_u64(bytes, (if value == 0. { 0. } else { value }).to_bits());
}

fn curve_bytes(curve: &Curve) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, curve.degree as u64);
    push_u64(&mut bytes, curve.periodic as u64);
    push_u64(&mut bytes, curve.control_points.len() as u64);
    push_u64(&mut bytes, curve.knots.len() as u64);
    for &knot in &curve.knots {
        push_f64(&mut bytes, knot);
    }
    for point in &curve.control_points {
        push_u64(&mut bytes, point.len() as u64);
        for &coordinate in point {
            push_f64(&mut bytes, coordinate);
        }
    }
    for &weight in &curve.weights {
        push_f64(&mut bytes, weight);
    }
    bytes
}

fn surface_bytes(surface: &Surface) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u64(&mut bytes, surface.degree_u as u64);
    push_u64(&mut bytes, surface.degree_v as u64);
    push_u64(&mut bytes, surface.periodic_u as u64);
    push_u64(&mut bytes, surface.periodic_v as u64);
    push_u64(&mut bytes, surface.control_points.len() as u64);
    push_u64(&mut bytes, surface.control_points[0].len() as u64);
    for &knot in surface.knots_u.iter().chain(&surface.knots_v) {
        push_f64(&mut bytes, knot);
    }
    for row in &surface.control_points {
        for point in row {
            for &coordinate in point {
                push_f64(&mut bytes, coordinate);
            }
        }
    }
    for row in &surface.weights {
        for &weight in row {
            push_f64(&mut bytes, weight);
        }
    }
    bytes
}

/// FNV-1a 64 hash of the canonical serialization; invariant to affine knot
/// rescaling, sub-tolerance representation noise and uniform weight scaling.
/// See the module docs for what it is NOT invariant to.
pub fn curve_hash(curve: &Curve, tolerance: f64) -> Result<u64> {
    Ok(fnv1a_64(&curve_bytes(&canonicalize_curve(curve, tolerance)?)))
}

/// FNV-1a 64 hash of the canonical surface serialization; same invariance
/// contract as [`curve_hash`].
pub fn surface_hash(surface: &Surface, tolerance: f64) -> Result<u64> {
    Ok(fnv1a_64(&surface_bytes(&canonicalize_surface(
        surface, tolerance,
    )?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_curve() -> Curve {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 2.],
                vec![3., 2.],
                vec![4., 0.],
            ],
            weights: vec![1., 0.7, 1.4, 2.],
            periodic: false,
        };
        curve.validate().unwrap();
        curve
    }

    fn sample_surface() -> Surface {
        let surface = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![1., 0., 1.], vec![2., 0., 0.]],
                vec![vec![0., 2., 0.], vec![1., 2., 1.], vec![2., 2., 0.]],
            ],
            weights: vec![vec![1., 0.8, 1.], vec![1., 1.2, 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        surface.validate().unwrap();
        surface
    }

    #[test]
    fn canonicalize_is_idempotent() {
        let curve = sample_curve();
        let once = canonicalize_curve(&curve, 1e-6).unwrap();
        let twice = canonicalize_curve(&once, 1e-6).unwrap();
        assert_eq!(once, twice);
        assert_eq!(curve_hash(&curve, 1e-6).unwrap(), curve_hash(&once, 1e-6).unwrap());
        assert_eq!(once.domain(), [0., 1.]);
        assert!(!once.periodic);
        assert_eq!(once.weights[0], 1.);
        assert_eq!(*once.weights.last().unwrap(), 1.);
        let surface = sample_surface();
        let s_once = canonicalize_surface(&surface, 1e-6).unwrap();
        let s_twice = canonicalize_surface(&s_once, 1e-6).unwrap();
        assert_eq!(s_once, s_twice);
        assert_eq!(
            surface_hash(&surface, 1e-6).unwrap(),
            surface_hash(&s_once, 1e-6).unwrap()
        );
    }

    #[test]
    fn hash_is_invariant_under_affine_knot_rescaling() {
        let curve = sample_curve();
        let rescaled = Curve {
            knots: curve.knots.iter().map(|k| 2. + 3. * k).collect(),
            ..curve.clone()
        };
        rescaled.validate().unwrap();
        assert_eq!(
            curve_hash(&curve, 1e-6).unwrap(),
            curve_hash(&rescaled, 1e-6).unwrap()
        );
        let surface = sample_surface();
        let s_rescaled = Surface {
            knots_u: surface.knots_u.iter().map(|k| -1. + 2. * k).collect(),
            knots_v: surface.knots_v.iter().map(|k| 5. + 10. * k).collect(),
            ..surface.clone()
        };
        s_rescaled.validate().unwrap();
        assert_eq!(
            surface_hash(&surface, 1e-6).unwrap(),
            surface_hash(&s_rescaled, 1e-6).unwrap()
        );
    }

    #[test]
    fn hash_is_invariant_under_weight_scaling_and_renormalization() {
        let curve = sample_curve();
        let scaled = Curve {
            weights: curve.weights.iter().map(|w| 7. * w).collect(),
            ..curve.clone()
        };
        scaled.validate().unwrap();
        assert_eq!(
            curve_hash(&curve, 1e-6).unwrap(),
            curve_hash(&scaled, 1e-6).unwrap()
        );
        let renormalized = normalize_endpoint_weights_report(&curve).unwrap().curve;
        assert_eq!(
            curve_hash(&curve, 1e-6).unwrap(),
            curve_hash(&renormalized, 1e-6).unwrap()
        );
        let surface = sample_surface();
        let s_scaled = Surface {
            weights: surface
                .weights
                .iter()
                .map(|row| row.iter().map(|w| 3. * w).collect())
                .collect(),
            ..surface.clone()
        };
        s_scaled.validate().unwrap();
        assert_eq!(
            surface_hash(&surface, 1e-6).unwrap(),
            surface_hash(&s_scaled, 1e-6).unwrap()
        );
    }

    #[test]
    fn hash_is_invariant_under_sub_tolerance_noise() {
        let curve = sample_curve();
        let mut noisy = curve.clone();
        noisy.control_points[1][0] += 1e-9;
        noisy.control_points[2][1] -= 1e-9;
        noisy.knots[3] += 1e-10;
        noisy.weights[1] += 1e-10;
        noisy.validate().unwrap();
        assert_eq!(
            curve_hash(&curve, 1e-6).unwrap(),
            curve_hash(&noisy, 1e-6).unwrap()
        );
    }

    #[test]
    fn hash_differs_for_genuinely_different_geometry() {
        let curve = sample_curve();
        let mut moved = curve.clone();
        moved.control_points[1][1] += 0.1;
        assert_ne!(
            curve_hash(&curve, 1e-6).unwrap(),
            curve_hash(&moved, 1e-6).unwrap()
        );
        let mut rewired = curve.clone();
        rewired.weights[1] = 2.5;
        assert_ne!(
            curve_hash(&curve, 1e-6).unwrap(),
            curve_hash(&rewired, 1e-6).unwrap()
        );
        let surface = sample_surface();
        let mut s_moved = surface.clone();
        s_moved.control_points[0][1][2] += 0.5;
        assert_ne!(
            surface_hash(&surface, 1e-6).unwrap(),
            surface_hash(&s_moved, 1e-6).unwrap()
        );
    }

    #[test]
    fn canonicalize_validates_tolerance() {
        let curve = sample_curve();
        assert!(canonicalize_curve(&curve, 0.).is_err());
        assert!(canonicalize_curve(&curve, f64::NAN).is_err());
        assert!(canonicalize_surface(&sample_surface(), -1.).is_err());
        assert!(curve_hash(&curve, f64::INFINITY).is_err());
    }
}
