//! Fat-line Bézier clipping (Sederberg–Nishita) as a certified pre-filter for
//! curve/curve intersection.
//!
//! For a Bézier segment `a`, the fat line is the slab around the chord through
//! `a`'s endpoints whose half-widths are the min/max signed perpendicular
//! distance of `a`'s control polygon. By the convex-hull property (a weighted
//! average of the control distances with positive Bernstein/weight
//! coefficients), the exact curve stays inside the slab, so any parameter of
//! `b` whose curve point lies outside the slab cannot be an intersection.
//! Clipping `b`'s (t, dist) control polygon convex hull against the slab
//! yields a shrunken parameter interval; alternating roles converges
//! quadratically on transversal intersections.
//!
//! Soundness: the fat-line band bound is exact for any positive weights (the
//! curve's signed distance is a convex combination of the control distances).
//! The (t, dist) hull clip of the *target* additionally requires uniform
//! weights: with non-uniform rational weights the parameter axis of the graph
//! is distorted, so such targets are skipped and left to bisection. Bands are
//! widened by outward rounding and a pad scaled by the coordinate magnitude,
//! so clipping never drops a true intersection. Curves are
//! 3D; the pair is projected onto the coordinate plane that maximizes the
//! projected chord length of the clipping curve, where the signed distance is
//! a linear functional and the hull argument applies.
//!
//! Degenerate inputs are honest: a coincident/collinear pair has all distances
//! near zero, the band contains the whole hull, and `clip_pair` returns the
//! full box (never `None`); a zero-length chord (pole-like segment) skips the
//! clip in that direction and also returns the current box unchanged.
use super::{next_down, next_up};
use crate::{Result, check, curve::Curve};

/// Hard bound on alternating clip rounds inside one `clip_pair` call; the
/// caller's subdivision loop owns the overall termination argument.
const MAX_ROUNDS: usize = 12;
/// A round that fails to shrink the larger relative width below this factor
/// stops clipping; the caller falls back to bisection.
const SHRINK_LIMIT: f64 = 0.8;
/// Relative pad on the fat-line band so binary64 rounding of the distance
/// samples can never exclude a true contact.
const BAND_PAD: f64 = 1e-12;

/// Outcome of clipping one curve against the other curve's fat line.
enum ClipOutcome {
    /// The fat-line band separates the convex hulls: no intersection exists.
    Separated,
    /// The chord is degenerate (zero projected length); no clip was attempted.
    Indeterminate,
    /// Surviving parameter interval of the clipped curve (outward rounded).
    Clipped([f64; 2]),
}

/// Euclidean control points and weights of the curve's Bézier piece over
/// `range`. Control points are stored as Euclidean coordinates; weights are
/// returned separately (positive weights keep the convex-hull bounds valid).
fn euclidean_controls(curve: &Curve, range: [f64; 2]) -> Result<(Vec<[f64; 3]>, Vec<f64>)> {
    check(
        range[0].is_finite() && range[1].is_finite() && range[0] < range[1],
        "Bezier clip requires a nonempty parameter interval",
    )?;
    let piece = curve.trim(range[0], range[1])?;
    check(
        piece.weights.iter().all(|&w| w > 0.),
        "Bezier clip requires positive weights",
    )?;
    let mut points = Vec::with_capacity(piece.control_points.len());
    for p in &piece.control_points {
        check(p.len() == 3, "Bezier clip requires 3D curves")?;
        points.push([p[0], p[1], p[2]]);
    }
    Ok((points, piece.weights.clone()))
}

/// The (t, dist) graph-hull clip is exact only when the target's parameter
/// coincides with the Bézier abscissa, i.e. when all weights are equal.
/// A non-uniform rational target distorts the parameter axis of the hull, so
/// clipping it could drop a true intersection; such targets are skipped (the
/// caller's bisection path stays correct for them).
fn weights_uniform(weights: &[f64]) -> bool {
    weights.iter().all(|&w| w == weights[0])
}

/// Coordinate planes are tried independently: the convex-hull band argument
/// is valid in every projection where the chord does not degenerate, so a
/// separation witness in any plane is sound and the tightest surviving
/// interval across planes is kept.
const PLANES: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];

/// Signed-distance samples of `points` against the directed line through the
/// endpoints of `base`, both projected onto the `(i, j)` coordinate plane.
/// `None` when the projected chord degenerates to a point.
fn projected_distances(
    base: &[[f64; 3]],
    points: &[[f64; 3]],
    i: usize,
    j: usize,
) -> Option<Vec<f64>> {
    let p0 = base[0];
    let p1 = *base.last().unwrap();
    let e = [p1[i] - p0[i], p1[j] - p0[j]];
    let len = (e[0] * e[0] + e[1] * e[1]).sqrt();
    let chord = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let chord_len = super::norm3(chord);
    // Degenerate projected chord: this plane cannot build a fat line.
    if !len.is_finite() || len <= 1e-14 * chord_len.max(f64::from_bits(1)) {
        return None;
    }
    // Unit in-plane normal of the projected chord.
    let n = [-e[1] / len, e[0] / len];
    let distances = points
        .iter()
        .map(|p| (p[i] - p0[i]) * n[0] + (p[j] - p0[j]) * n[1])
        .collect();
    Some(distances)
}

/// Parameter range of the convex hull of the Bézier control polygon
/// `(j/m, d_j)` intersected with the horizontal band `[dlo, dhi]`.
/// `None` when the hull misses the band entirely.
fn clip_against_band(distances: &[f64], dlo: f64, dhi: f64) -> Option<[f64; 2]> {
    let m = distances.len() - 1;
    let pts: Vec<[f64; 2]> = distances
        .iter()
        .enumerate()
        .map(|(k, &d)| [k as f64 / m as f64, d])
        .collect();
    // Monotone chain convex hull.
    let mut hull: Vec<[f64; 2]> = Vec::with_capacity(2 * pts.len());
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    for &p in &pts {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0. {
            hull.pop();
        }
        hull.push(p);
    }
    let lower_len = hull.len();
    for &p in pts.iter().rev().skip(1) {
        while hull.len() > lower_len && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.
        {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop();
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    let mut admit = |t: f64| {
        lo = lo.min(t);
        hi = hi.max(t);
    };
    let edges: Vec<([f64; 2], [f64; 2])> = hull
        .iter()
        .zip(hull.iter().cycle().skip(1))
        .map(|(&a, &b)| (a, b))
        .take(hull.len())
        .collect();
    for (a, b) in edges {
        if dlo <= a[1] && a[1] <= dhi {
            admit(a[0]);
        }
        for &band in &[dlo, dhi] {
            let da = a[1] - band;
            let db = b[1] - band;
            if da * db < 0. {
                admit(a[0] + (b[0] - a[0]) * (band - a[1]) / (b[1] - a[1]));
            }
        }
    }
    if !lo.is_finite() {
        return None;
    }
    Some([next_down(lo), next_up(hi)])
}

/// Clip `target`'s interval against the fat line of `base`, trying every
/// coordinate plane whose projected chord is non-degenerate. A separation
/// witness in any plane is sound; otherwise the tightest surviving interval
/// across planes is kept.
fn clip_one_direction(
    base: &[[f64; 3]],
    target: &[[f64; 3]],
    target_weights: &[f64],
    target_range: [f64; 2],
) -> ClipOutcome {
    if !weights_uniform(target_weights) {
        // Rational target with non-uniform weights: the (t, dist) hull clip
        // would distort the parameter axis. Skip; bisection stays correct.
        return ClipOutcome::Indeterminate;
    }
    let (t0, t1) = (target_range[0], target_range[1]);
    let width = t1 - t0;
    // Coordinate scale of both polygons: distance arithmetic rounds at
    // ~scale*eps, so the band pad must grow with it to stay conservative.
    let scale = base
        .iter()
        .chain(target.iter())
        .flat_map(|p| p.iter())
        .copied()
        .map(f64::abs)
        .fold(1., f64::max);
    let mut best: Option<[f64; 2]> = None;
    let mut any_plane = false;
    for &(i, j) in &PLANES {
        let Some(base_d) = projected_distances(base, base, i, j) else {
            continue;
        };
        let Some(target_d) = projected_distances(base, target, i, j) else {
            continue;
        };
        any_plane = true;
        let dmin = base_d.iter().copied().fold(f64::INFINITY, f64::min);
        let dmax = base_d.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let pad = (dmax - dmin).abs().max(dmax.abs()).max(dmin.abs()).max(scale) * BAND_PAD;
        let dlo = next_down(dmin - pad);
        let dhi = next_up(dmax + pad);
        let Some([u0, u1]) = clip_against_band(&target_d, dlo, dhi) else {
            return ClipOutcome::Separated;
        };
        let lo = next_down(t0 + u0.max(0.) * width).max(t0);
        let hi = next_up(t0 + u1.min(1.) * width).min(t1);
        let candidate = if lo < hi { [lo, hi] } else { target_range };
        best = match best {
            None => Some(candidate),
            Some(b) if candidate[1] - candidate[0] < b[1] - b[0] => Some(candidate),
            other => other,
        };
    }
    if !any_plane {
        return ClipOutcome::Indeterminate;
    }
    ClipOutcome::Clipped(best.unwrap_or(target_range))
}

/// Clips curve `b`'s parameter interval against the fat line of `a`'s current
/// Bézier segment (and vice versa per iteration). Returns the shrunk parameter
/// box, or None when the fat line separates the curves (no intersection).
/// Quadratic convergence on transversal intersections.
///
/// `budget` bounds the alternating clip rounds (one unit per round); when it
/// is exhausted the current box is returned unchanged-in-part. The function is
/// bounded independently of the caller's subdivision loop. A coincident or
/// collinear-overlapping pair never returns `None`: its band contains the
/// whole hull, so the full (or an honest partial) box comes back.
pub(crate) fn clip_pair(
    a: &Curve,
    b: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    budget: &mut usize,
) -> Result<Option<[f64; 4]>> {
    let mut cur_a = ta;
    let mut cur_b = tb;
    for _ in 0..MAX_ROUNDS {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        let start_a = cur_a;
        let start_b = cur_b;
        let (ca, wa) = euclidean_controls(a, cur_a)?;
        let (cb, wb) = euclidean_controls(b, cur_b)?;
        match clip_one_direction(&ca, &cb, &wb, cur_b) {
            ClipOutcome::Separated => return Ok(None),
            ClipOutcome::Indeterminate => {}
            ClipOutcome::Clipped(range) => cur_b = range,
        }
        // Re-clip in the opposite direction against the (possibly shrunk) box.
        let (ca, wa) = euclidean_controls(a, cur_a)?;
        let (cb, wb) = euclidean_controls(b, cur_b)?;
        match clip_one_direction(&cb, &ca, &wa, cur_a) {
            ClipOutcome::Separated => return Ok(None),
            ClipOutcome::Indeterminate => {}
            ClipOutcome::Clipped(range) => cur_a = range,
        }
        let width_a = (start_a[1] - start_a[0]).max(f64::from_bits(1));
        let width_b = (start_b[1] - start_b[0]).max(f64::from_bits(1));
        let shrink = ((cur_a[1] - cur_a[0]) / width_a).max((cur_b[1] - cur_b[0]) / width_b);
        if shrink >= SHRINK_LIMIT {
            break;
        }
    }
    Ok(Some([cur_a[0], cur_a[1], cur_b[0], cur_b[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::intersect_curve_curve_report_with_clip;

    fn bezier(points: &[[f64; 3]]) -> Curve {
        let n = points.len();
        let degree = n - 1;
        let knots = [vec![0.; n], vec![1.; n]].concat();
        Curve {
            degree,
            knots,
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; n],
            periodic: false,
        }
    }

    fn bspline(points: &[[f64; 3]], degree: usize) -> Curve {
        let n = points.len();
        let interior = n - degree - 1;
        let mut knots = vec![0.; degree + 1];
        for k in 1..=interior {
            knots.push(k as f64 / (interior + 1) as f64);
        }
        knots.extend(vec![1.; degree + 1]);
        Curve {
            degree,
            knots,
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; n],
            periodic: false,
        }
    }

    /// Two cubics crossing transversally near (1.5, 0, 0).
    fn crossing_cubics() -> (Curve, Curve) {
        let a = bezier(&[[0., 0., 0.], [1., 1., 0.], [2., -1., 0.], [3., 0., 0.]]);
        let b = bezier(&[[1.2, -1., 0.], [1.4, -0.3, 0.], [1.6, 0.3, 0.], [1.8, 1., 0.]]);
        (a, b)
    }

    #[test]
    fn transversal_crossing_clips_and_matches_unclipped_report() {
        let (a, b) = crossing_cubics();
        let mut budget = 16;
        let clipped = clip_pair(&a, &b, [0., 1.], [0., 1.], &mut budget)
            .unwrap()
            .expect("crossing curves must not be separated");
        assert!(budget < 16, "at least one clip round should run");
        // The shrunken box must still contain the true intersection parameters.
        let full = intersect_curve_curve_report_with_clip(&a, &b, None, 0).unwrap();
        let fast = intersect_curve_curve_report_with_clip(&a, &b, None, 16).unwrap();
        assert_eq!(full.report.components.len(), fast.report.components.len());
        assert_eq!(full.report.unresolved.len(), fast.report.unresolved.len());
        for component in &fast.report.components {
            assert!(
                clipped[0] <= component.first && component.first <= clipped[1],
                "clipped box must contain first parameter {}",
                component.first
            );
            assert!(
                clipped[2] <= component.second && component.second <= clipped[3],
                "clipped box must contain second parameter {}",
                component.second
            );
        }
        assert!(!fast.report.components.is_empty());
        assert!(fast.report.bezier_clipped > 0);
    }

    #[test]
    fn near_tangent_grazing_is_not_excluded() {
        // Parabola touching y = 0 at its vertex; the line is the tangent.
        let a = bezier(&[[0., 1., 0.], [1., 0., 0.], [2., 1., 0.]]);
        let b = bezier(&[[0., 0., 0.], [2., 0., 0.]]);
        let mut budget = 16;
        let clipped = clip_pair(&a, &b, [0., 1.], [0., 1.], &mut budget)
            .unwrap()
            .expect("grazing contact must never be excluded");
        assert!(clipped[0] <= 0.5 && 0.5 <= clipped[1]);
        assert!(clipped[2] <= 0.5 && 0.5 <= clipped[3]);
    }

    #[test]
    fn coincident_collinear_segments_never_separate() {
        let a = bezier(&[[0., 0., 0.], [2., 0., 0.]]);
        let b = bezier(&[[1., 0., 0.], [3., 0., 0.]]);
        let mut budget = 16;
        let clipped = clip_pair(&a, &b, [0., 1.], [0., 1.], &mut budget)
            .unwrap()
            .expect("coincident collinear segments must return an honest box");
        // Zero band contains the whole hull: the box comes back full.
        assert_eq!(clipped, [0., 1., 0., 1.]);
    }

    /// Regression: a clip-shrunk box is re-trimmed on its next visit; its
    /// collapsed hulls must not trigger a false Coincident overlap admission.
    /// Crossing lines must resolve to one transverse Point, with and without
    /// the clip pre-filter, at unit and adversarial (1e4) scales.
    #[test]
    fn crossing_lines_resolve_to_point_not_overlap() {
        for &scale in &[1., 1e4] {
            let a = bezier(&[[0., 0., 0.], [scale, scale, 0.]]);
            let mut b = bezier(&[[0., scale, 0.], [scale, 0., 0.]]);
            b.weights = vec![2., 3.];
            for budget in [0usize, 16] {
                let r = intersect_curve_curve_report_with_clip(&a, &b, None, budget).unwrap();
                assert_eq!(r.report.unresolved.len(), 0, "scale={scale} budget={budget}");
                assert_eq!(r.report.components.len(), 1, "scale={scale} budget={budget}");
                let c = &r.report.components[0];
                assert_eq!(c.kind, crate::intersection::CurveCurveComponentKind::Point);
                assert_eq!(c.contact, crate::intersection::ContactClass::Transverse);
                // a is uniformly weighted (t = 1/2); b carries weights [2, 3],
                // so its rational parameter at the crossing is u = 0.4.
                assert!((c.first - 0.5).abs() < 1e-8 && (c.second - 0.4).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn separated_curves_clip_to_none_quickly() {
        let a = bezier(&[[0., 0., 0.], [1., 0., 0.]]);
        let b = bezier(&[[0., 10., 0.], [1., 10., 0.]]);
        let mut budget = 16;
        assert_eq!(clip_pair(&a, &b, [0., 1.], [0., 1.], &mut budget).unwrap(), None);
        assert!(budget >= 15, "separation should be decided in one round");
    }

    #[test]
    fn zero_budget_returns_full_box_immediately() {
        let (a, b) = crossing_cubics();
        let mut budget = 0;
        let clipped = clip_pair(&a, &b, [0., 1.], [0., 1.], &mut budget)
            .unwrap()
            .expect("budget exhaustion must not exclude");
        assert_eq!(clipped, [0., 1., 0., 1.]);
    }

    #[test]
    fn wavy_pair_box_count_does_not_increase_with_clipping() {
        let a = bspline(
            &[
                [0., 0., 0.],
                [1., 1.5, 0.],
                [2., -1.5, 0.],
                [3., 1.5, 0.],
                [4., -1.5, 0.],
                [5., 0., 0.],
            ],
            3,
        );
        let b = bspline(
            &[
                [0., 0.4, 0.],
                [1., -0.4, 0.],
                [2., 0.4, 0.],
                [3., -0.4, 0.],
                [4., 0.4, 0.],
                [5., -0.4, 0.],
            ],
            3,
        );
        let plain = intersect_curve_curve_report_with_clip(&a, &b, None, 0).unwrap();
        let clipped = intersect_curve_curve_report_with_clip(&a, &b, None, 16).unwrap();
        println!(
            "wavy pair boxes_visited: without clip = {}, with clip = {} (bezier_clipped = {})",
            plain.report.boxes_visited, clipped.report.boxes_visited, clipped.report.bezier_clipped
        );
        assert_eq!(plain.report.components.len(), clipped.report.components.len());
        assert_eq!(plain.report.unresolved.len(), clipped.report.unresolved.len());
        assert!(clipped.report.boxes_visited <= plain.report.boxes_visited);
    }
}
