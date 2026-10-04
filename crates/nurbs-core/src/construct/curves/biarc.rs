//! G1 biarc approximation of planar curves within chord tolerance (item 306).
//!
//! Every biarc is a pair of circular arcs meeting at a joint on the
//! perpendicular bisector of the segment chord, with a shared tangent at the
//! joint (computed as a mirror reflection of the endpoint tangent across the
//! chord, which is the standard G1 biarc condition). Curves are split at
//! curvature inflections first — a biarc cannot change bending direction —
//! and then recursively bisected in parameter until the sampled deviation
//! from the source curve stays within `tolerance`.
//!
//! **Dimensionality.** 2D curves are fitted in the XY plane. 3D curves must
//! be planar within `tolerance / 64` against the least-squares plane of 33
//! samples; fitting then proceeds directly in 3D with every arc lying in that
//! plane. Truly non-planar 3D curves are refused with `NURBS_INVALID_INPUT`,
//! because circular arcs are planar by nature and silently projecting would
//! change the geometry.
//!
//! Practically straight pieces become exact linear quadratic segments with
//! `radius = f64::INFINITY` in the [`BiArc`] record.
use crate::{Result, check, numeric_err, resource};
use crate::curve::Curve;
use crate::foundation::bracketed::brent_bracketed;
use math_core::{add, cross, dot, next_up, norm, scale, sub};

/// One G1 pair of circular arcs: `start → joint` with `radii[0]`, then
/// `joint → end` with `radii[1]`. An infinite radius marks a straight piece.
#[derive(Clone, Debug)]
pub struct BiArc {
    pub start: [f64; 3],
    pub joint: [f64; 3],
    pub end: [f64; 3],
    pub radii: [f64; 2],
    /// Largest sampled deviation of this arc pair from the source curve.
    pub sagitta_error: f64,
}

#[derive(Clone, Debug)]
pub struct BiArcReport {
    pub arcs: Vec<BiArc>,
    /// Exact rational quadratic arcs via [`crate::primitives::circle_arc`],
    /// two per [`BiArc`] (straight pieces are exact linear quadratics).
    pub as_nurbs: Vec<Curve>,
    /// Outward-rounded maximum sampled deviation from the source curve.
    pub max_error: f64,
}

const MAX_DEPTH: usize = 16;
const SAMPLES: usize = 32;

fn point3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], *p.get(2).unwrap_or(&0.)]
}

/// One circular arc embedded in the fitting plane. `radius = INFINITY`
/// marks a straight piece (center unused).
#[derive(Clone, Debug)]
struct Arc {
    start: [f64; 3],
    end: [f64; 3],
    center: [f64; 3],
    radius: f64,
    tangent: [f64; 3],
    normal: [f64; 3],
}

/// Point/arc distance: radial gap to the circle, or perpendicular gap to
/// the tangent line for straight pieces.
fn deviation(arc: &Arc, q: [f64; 3]) -> f64 {
    if arc.radius.is_infinite() {
        return dot(cross(arc.normal, arc.tangent), sub(q, arc.start)).abs();
    }
    (norm(sub(q, arc.center)) - arc.radius).abs()
}

/// Circular arc from `a` with unit tangent `ta` passing through `b`, lying in
/// the plane with unit `normal`. Falls back to a straight piece when the
/// chord is tangent-parallel within rounding.
fn arc_through(a: [f64; 3], ta: [f64; 3], b: [f64; 3], normal: [f64; 3]) -> Result<Arc> {
    let chord = sub(b, a);
    let len = norm(chord);
    if !(len > 0.) || !len.is_finite() {
        return Err(numeric_err("Biarc arc endpoints coincide"));
    }
    let side = cross(normal, ta);
    let denom = dot(side, chord);
    if denom.abs() <= 1e-14 * len {
        return Ok(Arc {
            start: a,
            end: b,
            center: [f64::INFINITY; 3],
            radius: f64::INFINITY,
            tangent: ta,
            normal,
        });
    }
    // Center: intersection of the normal line at `a` with the chord bisector.
    let s = 0.5 * len * len / denom;
    Ok(Arc {
        start: a,
        end: b,
        center: add(a, scale(side, s)),
        radius: s.abs(),
        tangent: ta,
        normal,
    })
}

/// Curve point at `u` plus a unit tangent, nudging off continuity breaks.
fn frame(curve: &Curve, u: f64) -> Result<([f64; 3], [f64; 3])> {
    let [a, b] = curve.domain();
    let point = point3(&curve.evaluate(u)?.point);
    let delta = 1e-9 * (b - a);
    for trial in [u, u + delta, u - delta, u + 16. * delta, u - 16. * delta] {
        if trial < a || trial > b {
            continue;
        }
        if let Some(d1) = curve.evaluate(trial)?.d1 {
            let t = point3(&d1);
            let n = norm(t);
            if n > 0. && n.is_finite() {
                return Ok((point, scale(t, 1. / n)));
            }
        }
    }
    Err(numeric_err(
        "Biarc fitting needs a nonzero curve tangent",
    ))
}

/// G1 biarc for one segment: joint on the chord's perpendicular bisector,
/// joint tangent shared by both arcs (tangents mirror across their chords).
/// Returns the joint, the shared unit joint tangent, and both arcs.
fn biarc_pair(
    p0: [f64; 3],
    t0: [f64; 3],
    p1: [f64; 3],
    t1: [f64; 3],
    normal: [f64; 3],
) -> Result<([f64; 3], [f64; 3], Arc, Arc)> {
    let w = sub(p1, p0);
    let d = norm(w);
    if !(d > 0.) || !d.is_finite() {
        return Err(numeric_err("Biarc segment endpoints coincide"));
    }
    let nhat = cross(normal, scale(w, 1. / d));
    let m = scale(add(p0, p1), 0.5);
    let joint_at = |t: f64| add(m, scale(nhat, t));
    // Tangent at the far end of an arc entering with tangent `t` along chord `c`.
    let reflected = |t: [f64; 3], c: [f64; 3]| -> Option<[f64; 3]> {
        let cn = norm(c);
        if cn <= 1e-13 * d {
            return None;
        }
        let cu = scale(c, 1. / cn);
        Some(sub(scale(cu, 2. * dot(t, cu)), t))
    };
    // G1 condition: the two joint tangents are parallel in the plane.
    let condition = |t: f64| -> f64 {
        let j = joint_at(t);
        let (Some(s1), Some(s2)) = (reflected(t0, sub(j, p0)), reflected(t1, sub(p1, j))) else {
            return f64::NAN;
        };
        dot(cross(s1, s2), normal)
    };
    // Bracket sign changes of the condition along the bisector; prefer the
    // root nearest the chord midpoint (the shorter of the two arc pairs).
    let grid = 64;
    let mut brackets = Vec::new();
    let mut prev = (-2. * d, condition(-2. * d));
    for i in 1..=grid {
        let t = -2. * d + 4. * d * i as f64 / grid as f64;
        let ft = condition(t);
        if ft == 0. {
            brackets.push((t, t));
        } else if ft.is_finite() && prev.1.is_finite() && ft.signum() != prev.1.signum() {
            brackets.push((prev.0, t));
        }
        if ft.is_finite() {
            prev = (t, ft);
        }
    }
    brackets.sort_by(|x, y| {
        (0.5 * (x.0 + x.1))
            .abs()
            .total_cmp(&(0.5 * (y.0 + y.1)).abs())
    });
    for (lo, hi) in brackets {
        let tstar = if lo == hi {
            lo
        } else {
            match brent_bracketed(&condition, lo, hi, 1e-12 * d, 100) {
                Ok(t) => t,
                Err(_) => continue,
            }
        };
        let joint = joint_at(tstar);
        let (Some(s1), Some(_)) = (
            reflected(t0, sub(joint, p0)),
            reflected(t1, sub(p1, joint)),
        ) else {
            continue;
        };
        // Reject cusp solutions where the joint tangent flips direction.
        if dot(s1, reflected(t1, sub(p1, joint)).unwrap_or(s1)) <= 0. {
            continue;
        }
        let sj = scale(s1, 1. / norm(s1));
        let first = arc_through(p0, t0, joint, normal)?;
        let second = arc_through(joint, sj, p1, normal)?;
        return Ok((joint, sj, first, second));
    }
    Err(numeric_err(
        "No G1 biarc exists for this segment; it needs a finer split",
    ))
}

struct Fitter<'a> {
    curve: &'a Curve,
    normal: [f64; 3],
    tolerance: f64,
    budget: usize,
    used: usize,
    max_error: f64,
    pairs: Vec<(BiArc, Arc, Arc)>,
}

impl Fitter<'_> {
    fn measure(&self, u0: f64, u1: f64, first: &Arc, second: &Arc) -> Result<f64> {
        let mut error = 0_f64;
        for i in 0..=SAMPLES {
            let u = u0 + (u1 - u0) * i as f64 / SAMPLES as f64;
            let q = point3(&self.curve.evaluate(u)?.point);
            let arc = if 2 * i <= SAMPLES { first } else { second };
            error = error.max(deviation(arc, q));
        }
        Ok(error)
    }

    fn fit(&mut self, u0: f64, u1: f64, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(numeric_err(
                "Biarc fitting did not converge within the tolerance",
            ));
        }
        let (p0, t0) = frame(self.curve, u0)?;
        let (p1, t1) = frame(self.curve, u1)?;
        if let Ok((joint, _, first, second)) = biarc_pair(p0, t0, p1, t1, self.normal) {
            let error = self.measure(u0, u1, &first, &second)?;
            if error <= self.tolerance * (1. - 1e-9) {
                self.used += 2;
                if self.used > self.budget {
                    return Err(resource("Biarc fitting exceeds the arc budget"));
                }
                self.max_error = self
                    .max_error
                    .max(next_up(error * (1. + 8. * f64::EPSILON)));
                self.pairs.push((
                    BiArc {
                        start: p0,
                        joint,
                        end: p1,
                        radii: [first.radius, second.radius],
                        sagitta_error: error,
                    },
                    first,
                    second,
                ));
                return Ok(());
            }
        }
        let mid = 0.5 * (u0 + u1);
        self.fit(u0, mid, depth + 1)?;
        self.fit(mid, u1, depth + 1)
    }
}

/// Least-squares plane normal of sampled 3D points (smallest covariance
/// eigenvector).
fn fitting_plane(points: &[[f64; 3]]) -> [f64; 3] {
    let n = points.len() as f64;
    let centroid = scale(points.iter().fold([0.; 3], |a, p| add(a, *p)), 1. / n);
    let mut cov = [[0.; 3]; 3];
    for p in points {
        let d = sub(*p, centroid);
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    let (values, vectors) = math_core::eigen(cov);
    let k = (0..3).min_by(|&i, &j| values[i].total_cmp(&values[j])).unwrap();
    [vectors[0][k], vectors[1][k], vectors[2][k]]
}

/// Replicates the deterministic reference frame of
/// [`crate::primitives::circle_arc`] so authored start angles line up exactly.
fn circle_frame(normal: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let scale_m = normal.iter().fold(0_f64, |a, x| a.max(x.abs()));
    let n = normal.map(|x| x / scale_m);
    let len = norm(n);
    let n = n.map(|x| x / len);
    let mut reference = 0;
    for i in 1..3 {
        if n[i].abs() < n[reference].abs() {
            reference = i;
        }
    }
    let u = std::array::from_fn::<_, 3, _>(|i| {
        if i == reference {
            1. - n[i] * n[reference]
        } else {
            -n[i] * n[reference]
        }
    });
    let len = norm(u);
    let u = u.map(|x| x / len);
    (u, cross(n, u))
}

/// Exact rational quadratic NURBS for one fitted arc.
fn arc_nurbs(arc: &Arc, normal: [f64; 3]) -> Result<Curve> {
    if arc.radius.is_infinite() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                arc.start.to_vec(),
                scale(add(arc.start, arc.end), 0.5).to_vec(),
                arc.end.to_vec(),
            ],
            weights: vec![1., 1., 1.],
            periodic: false,
        };
        curve.validate()?;
        return Ok(curve);
    }
    let r0 = sub(arc.start, arc.center);
    let r1 = sub(arc.end, arc.center);
    // Motion sign around the normal fixed by the authored start tangent.
    let ccw = cross(normal, scale(r0, 1. / arc.radius));
    let dir = dot(arc.tangent, ccw).signum();
    let raw = dot(cross(r0, r1), normal).atan2(dot(r0, r1));
    if raw == 0. {
        return Err(numeric_err("Biarc arc has zero sweep"));
    }
    let sweep = if raw.signum() == dir {
        raw
    } else {
        raw - 2. * std::f64::consts::PI * raw.signum()
    };
    let (u, v) = circle_frame(normal);
    let start = dot(r0, v).atan2(dot(r0, u)).to_degrees();
    crate::primitives::circle_arc(arc.center, normal, arc.radius, start, sweep.to_degrees())
}

/// G1 biarc approximation of `curve` within chord `tolerance`, at most
/// `max_arcs` (≤ 256) arcs. Splits at curvature inflections before recursive
/// bisection. See the module documentation for the 3D planarity policy.
pub fn fit_biares_report(
    curve: &Curve,
    tolerance: f64,
    max_arcs: usize,
) -> Result<BiArcReport> {
    curve.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Biarc tolerance must be positive and finite",
    )?;
    check((1..=256).contains(&max_arcs), "Biarc budget must be in 1..=256")?;
    let [a, b] = curve.domain();
    let samples = (0..=SAMPLES)
        .map(|i| {
            curve
                .evaluate(a + (b - a) * i as f64 / SAMPLES as f64)
                .map(|evaluation| point3(&evaluation.point))
        })
        .collect::<Result<Vec<_>>>()?;
    let normal = if curve.control_points[0].len() == 2 {
        [0., 0., 1.]
    } else {
        let normal = fitting_plane(&samples);
        let offset = samples.iter().map(|p| dot(*p, normal)).sum::<f64>() / samples.len() as f64;
        let deviation = samples
            .iter()
            .map(|p| (dot(*p, normal) - offset).abs())
            .fold(0., f64::max);
        check(
            deviation <= tolerance / 64.,
            "Biarc fitting requires a curve planar within tolerance/64; circular arcs are planar",
        )?;
        normal
    };
    // Split at curvature inflections first: a biarc cannot change bending
    // direction, so crossing an inflection would force endless bisection.
    let mut cuts = vec![a];
    for u in crate::curve_analysis::inflection_parameters(curve, tolerance)? {
        if u > a + 1e-9 * (b - a) && u < b - 1e-9 * (b - a) {
            cuts.push(u);
        }
    }
    cuts.push(b);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let mut fitter = Fitter {
        curve,
        normal,
        tolerance,
        budget: max_arcs,
        used: 0,
        max_error: 0.,
        pairs: Vec::new(),
    };
    for window in cuts.windows(2) {
        fitter.fit(window[0], window[1], 0)?;
    }
    let mut arcs = Vec::with_capacity(fitter.pairs.len());
    let mut as_nurbs = Vec::with_capacity(2 * fitter.pairs.len());
    for (biarc, first, second) in fitter.pairs {
        arcs.push(biarc);
        as_nurbs.push(arc_nurbs(&first, normal)?);
        as_nurbs.push(arc_nurbs(&second, normal)?);
    }
    Ok(BiArcReport {
        arcs,
        as_nurbs,
        max_error: fitter.max_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::circle_arc;

    fn unit(v: Vec<f64>) -> [f64; 3] {
        let t = point3(&v);
        scale(t, 1. / norm(t))
    }

    #[test]
    fn semicircle_fits_within_tolerance_with_g1_joints() {
        let curve = circle_arc([0.; 3], [0., 0., 1.], 1., 0., 180.).unwrap();
        let report = fit_biares_report(&curve, 1e-3, 256).unwrap();
        assert!((2..=4).contains(&report.as_nurbs.len()));
        assert!(report.max_error <= 1e-3);
        for biarc in &report.arcs {
            for radius in biarc.radii {
                assert!((radius - 1.).abs() < 1e-6, "radius {radius}");
            }
        }
        for pair in report.as_nurbs.windows(2) {
            let before = pair[0].evaluate(1.).unwrap();
            let after = pair[1].evaluate(0.).unwrap();
            for k in 0..3 {
                assert!((before.point[k] - after.point[k]).abs() < 1e-9);
            }
            let t0 = unit(before.d1.unwrap());
            let t1 = unit(after.d1.unwrap());
            assert!(dot(t0, t1) > 1. - 1e-9, "G1 break: {t0:?} vs {t1:?}");
        }
    }

    #[test]
    fn s_curve_splits_at_inflection() {
        let curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 1.],
                vec![2., -1.],
                vec![3., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        curve.validate().unwrap();
        let inflections = crate::curve_analysis::inflection_parameters(&curve, 1e-6).unwrap();
        assert_eq!(inflections.len(), 1);
        let report = fit_biares_report(&curve, 1e-4, 256).unwrap();
        assert!(report.arcs.len() >= 2);
        assert!(report.max_error <= 1e-4);
        let target = curve.evaluate(inflections[0]).unwrap().point;
        let hit = report.arcs.iter().any(|biarc| {
            [biarc.start, biarc.joint, biarc.end]
                .iter()
                .any(|p| (p[0] - target[0]).abs() < 1e-9 && (p[1] - target[1]).abs() < 1e-9)
        });
        assert!(hit, "no biarc boundary sits on the inflection point");
    }

    #[test]
    fn planar_3d_supported_nonplanar_refused() {
        // A 120-degree arc in an oblique plane: planar 3D works.
        let curve = circle_arc([1., 2., 3.], [1., 2., 3.], 2., 0., 120.).unwrap();
        let report = fit_biares_report(&curve, 1e-3, 256).unwrap();
        assert!(report.max_error <= 1e-3);
        // Genuinely non-planar 3D polyline: refused, not projected.
        let nonplanar = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![1., 1., 1.],
            vec![2., 1., 1.],
        ])
        .unwrap();
        assert!(fit_biares_report(&nonplanar, 1e-6, 256).is_err());
    }

    #[test]
    fn rejects_invalid_inputs_and_budget_exhaustion() {
        let curve = circle_arc([0.; 3], [0., 0., 1.], 1., 0., 180.).unwrap();
        assert!(fit_biares_report(&curve, 0., 256).is_err());
        assert!(fit_biares_report(&curve, f64::NAN, 256).is_err());
        assert!(fit_biares_report(&curve, 1e-3, 0).is_err());
        assert!(fit_biares_report(&curve, 1e-3, 257).is_err());
        // A semicircle needs at least two arcs; a budget of one must fail.
        assert!(fit_biares_report(&curve, 1e-3, 1).is_err());
    }

    #[test]
    fn nurbs_arcs_track_the_source_curve() {
        let curve = circle_arc([0.; 3], [0., 0., 1.], 1., 0., 180.).unwrap();
        let report = fit_biares_report(&curve, 1e-6, 256).unwrap();
        assert!(report.max_error <= 1e-6);
        // Endpoints of the whole chain match the source endpoints.
        let first = report.as_nurbs.first().unwrap().evaluate(0.).unwrap().point;
        let last = report.as_nurbs.last().unwrap().evaluate(1.).unwrap().point;
        let s0 = curve.evaluate(0.).unwrap().point;
        let s1 = curve.evaluate(1.).unwrap().point;
        for k in 0..3 {
            assert!((first[k] - s0[k]).abs() < 1e-9);
            assert!((last[k] - s1[k]).abs() < 1e-9);
        }
    }
}
