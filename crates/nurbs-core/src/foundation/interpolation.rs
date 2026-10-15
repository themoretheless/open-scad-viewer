//! General-degree global curve interpolation and adaptive fit-and-refine.
//!
//! Algorithms from The NURBS Book (Piegl & Tiller), chapter 9:
//! - A9.1: global interpolation with a clamped knot vector built by averaging
//!   `p` consecutive normalized parameters.
//! - End-tangent clamping: two extra derivative equations appended to the
//!   interpolation system.
//! - Fit-and-refine: least-squares fit from a minimal control net, inserting a
//!   knot at the worst-residual span until the max-norm deviation at the data
//!   sites meets the tolerance, then optional certified knot removal.
use super::approximate_edits::remove_curve_knot_report;
use super::{Curve, Result, check, distance, next_up, numeric, resource, solve};
use crate::curve::basis;
use crate::foundation::guards::{Budget, require_finite_at};

/// Site parameterization strategy for interpolation and fitting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameterization {
    Uniform,
    ChordLength,
    Centripetal,
}

/// Normalized increasing site parameters in `[0, 1]`.
///
/// Centripetal accumulation uses `sqrt(chord)` increments. Degenerate inputs
/// (fewer than two distinct points, zero total accumulation) fall back to the
/// uniform parameterization; callers that need strict increase must validate.
pub fn parameters(points: &[[f64; 3]], param: Parameterization) -> Vec<f64> {
    let n = points.len();
    if n < 2 {
        return vec![0.; n];
    }
    let uniform = || (0..n).map(|i| i as f64 / (n - 1) as f64).collect::<Vec<_>>();
    match param {
        Parameterization::Uniform => uniform(),
        Parameterization::ChordLength | Parameterization::Centripetal => {
            let mut result = vec![0.];
            for pair in points.windows(2) {
                let chord = distance(&pair[0], &pair[1]);
                let increment = match param {
                    Parameterization::Centripetal => chord.sqrt(),
                    _ => chord,
                };
                result.push(result.last().unwrap() + increment);
            }
            let total = *result.last().unwrap();
            if total.is_finite() && total > 0. {
                result.into_iter().map(|value| value / total).collect()
            } else {
                uniform()
            }
        }
    }
}

/// Clamped knot vector by parameter averaging (A9.1): with `count` control
/// points, interior knots are `U[p+j] = mean(u_j .. u_{j+p-1})`.
/// `sites` must be normalized to `[0, 1]` and have length `count` or more;
/// extra trailing sites only extend the averaging window (tangent case).
fn averaged_knots(degree: usize, count: usize, sites: &[f64]) -> Result<Vec<f64>> {
    check(
        (1..=25).contains(&degree) && count > degree && count <= 256,
        "Interpolation control count must be in (degree, 256]",
    )?;
    check(
        sites.len() >= count
            && sites.iter().all(|u| u.is_finite())
            && sites.windows(2).all(|w| w[0] <= w[1]),
        "Averaged knots need at least control-count nondecreasing sites",
    )?;
    let interior = count - degree - 1;
    let mut knots = vec![0.; degree + 1];
    for j in 1..=interior {
        let mean = sites[j..j + degree].iter().sum::<f64>() / degree as f64;
        knots.push(mean);
    }
    knots.extend(std::iter::repeat_n(1., degree + 1));
    Ok(knots)
}

/// Full design row of the clamped basis at one site.
fn basis_row(degree: usize, knots: &[f64], count: usize, u: f64) -> Result<Vec<f64>> {
    let b = basis(degree, knots, count, u, false)?;
    check(
        b.basis.len() == count && b.basis.iter().all(|v| v.is_finite()),
        "Interpolation basis row is malformed",
    )?;
    Ok(b.basis)
}

/// Schoenberg–Whitney collocation validation report.
pub struct CollocationReport {
    /// Minimum over rows of the diagonal value `N_i(t_i)`; positive means
    /// every parameter sits strictly inside its basis function's support.
    pub min_support_value: f64,
    /// Row attaining `min_support_value`.
    pub worst_row: usize,
    /// `min_support_value > min_basis`.
    pub ok: bool,
}

/// Validates the Schoenberg–Whitney condition `t_i ∈ supp(N_i)` (i.e.
/// `N_i(t_i) > min_basis`) BEFORE solving the interpolation system, rejecting
/// degenerate collocation schemes early. The collocation matrix of a clamped
/// interpolation is nonsingular iff every parameter lies strictly inside the
/// support of its same-indexed basis function; near-zero diagonal values make
/// the system arbitrarily ill-conditioned even when formally nonsingular.
///
/// Requires a square scheme: `parameters.len() == knots.len() - degree - 1`.
pub fn validate_collocation(
    degree: usize,
    knots: &[f64],
    parameters: &[f64],
    min_basis: f64,
) -> Result<CollocationReport> {
    check((1..=25).contains(&degree), "Collocation degree must be in [1, 25]")?;
    check(
        knots.len() >= 2 * (degree + 1)
            && knots.len() - degree - 1 <= 256
            && knots.iter().all(|k| k.is_finite())
            && knots.windows(2).all(|w| w[0] <= w[1]),
        "Collocation knots must be finite, nondecreasing, and consistent with the degree",
    )?;
    let count = knots.len() - degree - 1;
    check(
        parameters.len() == count,
        "Collocation needs one parameter per basis function (square scheme)",
    )?;
    check(
        min_basis.is_finite() && min_basis > 0.,
        "Collocation threshold must be positive and finite",
    )?;
    check(
        parameters
            .iter()
            .all(|t| t.is_finite() && *t >= knots[degree] && *t <= knots[count]),
        "Collocation parameters must be finite and inside the knot domain",
    )?;
    let mut min_support_value = f64::INFINITY;
    let mut worst_row = 0;
    for (i, &t) in parameters.iter().enumerate() {
        let row = basis_row(degree, knots, count, t)?;
        let value = row[i];
        numeric(value.is_finite(), "Collocation diagonal is not finite")?;
        if value < min_support_value {
            min_support_value = value;
            worst_row = i;
        }
    }
    Ok(CollocationReport {
        min_support_value,
        worst_row,
        ok: min_support_value > min_basis,
    })
}

/// A9.1 global interpolation of `points` with arbitrary `degree`.
///
/// With `end_tangents = Some((d0, d1))` two derivative equations are appended
/// (A9.x clamped end condition). For a clamped B-spline,
/// `C'(0) = p/Δ0 · (P1 − P0)` and `C'(1) = p/Δ1 · (Pn − Pn−1)` with
/// `Δ0 = U[p+1]`, `Δ1 = 1 − U[m−p−1]`. The equations are entered in the
/// control-difference form `P1 − P0 = d0·Δ0/p` (resp. `Pn − Pn−1 = d1·Δ1/p`),
/// i.e. each derivative row is scaled by `Δ/p`. This keeps row coefficients
/// O(1) like the position rows — weighting position and derivative
/// constraints comparably in the elimination — and is the consistent choice
/// because `Δ/p` is exactly the scale that makes a uniform Bézier control
/// polygon reproduce `C' = P1 − P0`. Tangents are `dC/du` on the normalized
/// `[0, 1]` parameter domain.
pub fn interpolate_curve(
    points: &[[f64; 3]],
    degree: usize,
    param: Parameterization,
    end_tangents: Option<([f64; 3], [f64; 3])>,
) -> Result<Curve> {
    let n = points.len();
    check(
        (2..=4096).contains(&n),
        "Curve interpolation needs 2..4096 data sites",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite() && x.abs() <= 1e9),
        "Interpolation sites must be finite and bounded by 1e9",
    )?;
    check((1..=25).contains(&degree), "Degree must be in [1, 25]")?;
    let extra = usize::from(end_tangents.is_some()) * 2;
    check(
        n + extra > degree,
        "Interpolation needs more equations than the degree",
    )?;
    let count = n + extra;
    check(count <= 256, "Interpolation exceeds 256 control points")?;
    if let Some((d0, d1)) = end_tangents {
        check(
            d0.iter().chain(&d1).all(|x| x.is_finite() && x.abs() <= 1e12),
            "Endpoint tangents must be finite and bounded",
        )?;
    }
    let u = parameters(points, param);
    check(
        u.windows(2).all(|w| w[0] < w[1]),
        "Interpolation parameters must strictly increase; sites must be distinct",
    )?;
    // With end tangents the system has two extra unknowns; extend the site
    // sequence by repeating the terminal parameters (A9.x: the derivative
    // rows act like phantom data at the ends) so the averaging window stays
    // in range and every interior knot remains strictly inside (0, 1).
    let sites: Vec<f64> = if extra == 0 {
        u.clone()
    } else {
        std::iter::once(u[0])
            .chain(u.iter().copied())
            .chain(std::iter::once(u[n - 1]))
            .collect()
    };
    let knots = averaged_knots(degree, count, &sites)?;
    // Schoenberg–Whitney pre-check on the square collocation block. With end
    // tangents the system carries two derivative rows and the plain positional
    // diagonal test does not apply, so it is limited to the pure square case.
    if extra == 0 {
        let collocation = validate_collocation(degree, &knots, &u, 1e-12)?;
        check(
            collocation.ok,
            "Schoenberg–Whitney collocation condition failed: a parameter lies (numerically) outside its basis support",
        )?;
    }
    let mut matrix: Vec<Vec<f64>> = Vec::with_capacity(count);
    let mut rhs: Vec<Vec<f64>> = Vec::with_capacity(count);
    for (i, point) in points.iter().enumerate() {
        matrix.push(basis_row(degree, &knots, count, u[i])?);
        rhs.push(point.to_vec());
    }
    if let Some((d0, d1)) = end_tangents {
        let m = knots.len() - 1;
        let delta0 = knots[degree + 1] - knots[0];
        let delta1 = knots[m - degree] - knots[m - degree - 1];
        check(
            delta0 > 0. && delta1 > 0.,
            "Clamped end spans collapsed during knot averaging",
        )?;
        let mut start = vec![0.; count];
        start[0] = -1.;
        start[1] = 1.;
        matrix.push(start);
        rhs.push(d0.map(|x| x * delta0 / degree as f64).to_vec());
        let mut end = vec![0.; count];
        end[count - 2] = -1.;
        end[count - 1] = 1.;
        matrix.push(end);
        rhs.push(d1.map(|x| x * delta1 / degree as f64).to_vec());
    }
    let controls = solve(matrix, rhs)?;
    let curve = Curve {
        degree,
        knots,
        control_points: controls,
        weights: vec![1.; count],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

/// Result of an adaptive fit-and-refine run.
pub struct AdaptiveFitReport {
    pub curve: Curve,
    /// Max-norm residual at the data sites, recomputed on the final curve
    /// (after any simplification), so it is an honest worst-case figure.
    pub max_deviation: f64,
    pub iterations: usize,
    pub converged: bool,
}

/// Weighted least-squares control net for fixed knots and site parameters.
/// Weights act as point confidence: row `i` of the design and its right-hand
/// side are scaled by `sqrt(w_i)`, which weights the squared residual of
/// site `i` by `w_i` in the normal equations.
fn lsq_controls(
    degree: usize,
    knots: &[f64],
    count: usize,
    sites: &[f64],
    points: &[[f64; 3]],
    weights: Option<&[f64]>,
) -> Result<Vec<Vec<f64>>> {
    let mut design = Vec::with_capacity(sites.len());
    for (i, &u) in sites.iter().enumerate() {
        let mut row = basis_row(degree, knots, count, u)?;
        if let Some(w) = weights {
            let scale = w[i].sqrt();
            for value in &mut row {
                *value *= scale;
            }
        }
        design.push(row);
    }
    let mut matrix = vec![vec![0.; count]; count];
    let mut rhs = vec![vec![0.; 3]; count];
    for (i, row) in design.iter().enumerate() {
        let scale = weights.map(|w| w[i].sqrt()).unwrap_or(1.);
        for a in 0..count {
            if row[a] == 0. {
                continue;
            }
            for b in 0..=a {
                matrix[a][b] += row[a] * row[b];
            }
            for axis in 0..3 {
                rhs[a][axis] += row[a] * scale * points[i][axis];
            }
        }
    }
    for a in 0..count {
        for b in 0..a {
            matrix[b][a] = matrix[a][b];
        }
    }
    numeric(
        matrix.iter().flatten().chain(rhs.iter().flatten()).all(|x| x.is_finite()),
        "Fit normal equations overflowed finite precision",
    )?;
    solve(matrix, rhs)
}

fn max_deviation(curve: &Curve, sites: &[f64], points: &[[f64; 3]]) -> Result<f64> {
    let mut deviation: f64 = 0.;
    for (i, &u) in sites.iter().enumerate() {
        let evaluation = curve.evaluate(u)?;
        deviation = deviation.max(distance(&evaluation.point, &points[i]));
    }
    numeric(deviation.is_finite(), "Fit deviation is not finite")?;
    Ok(deviation)
}

/// Fit-and-refine: least-squares fit starting from the minimal (Bézier)
/// control net; each iteration inserts a knot at the span holding the largest
/// max-norm residual until the worst-site deviation meets `tolerance` or the
/// control budget is exhausted. With `simplify`, each interior knot is then
/// offered to `remove_curve_knot_report` with the same tolerance budget and
/// accepted removals are kept.
pub fn fit_curve_adaptive(
    points: &[[f64; 3]],
    degree: usize,
    tolerance: f64,
    param: Parameterization,
    weights: Option<&[f64]>,
    simplify: bool,
) -> Result<AdaptiveFitReport> {
    let n = points.len();
    check(
        (2..=4096).contains(&n),
        "Adaptive curve fitting needs 2..4096 data sites",
    )?;
    for (i, point) in points.iter().enumerate() {
        for (axis, &x) in point.iter().enumerate() {
            require_finite_at(x, "fit_sites", i * 3 + axis)?;
        }
    }
    check(
        points.iter().flatten().all(|x| x.abs() <= 1e9),
        "Fit sites must be bounded by 1e9",
    )?;
    check(
        (1..=25).contains(&degree) && n > degree,
        "Adaptive fit needs degree in [1, 25] below the site count",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Fit tolerance must be positive and finite",
    )?;
    if let Some(w) = weights {
        check(
            w.len() == n && w.iter().all(|x| x.is_finite() && *x > 0. && *x <= 1e12),
            "Fit weights must match the sites and be positive, finite, and ≤ 1e12",
        )?;
    }
    let u = parameters(points, param);
    check(
        u.windows(2).all(|w| w[0] < w[1]),
        "Fit parameters must strictly increase; sites must be distinct",
    )?;
    let mut knots = averaged_knots(degree, degree + 1, &u)?;
    let mut iterations = 0;
    let mut converged = false;
    let mut fitted: Option<(Curve, f64)> = None;
    // Unified budget guard on knot insertions (the historical 256 budget).
    let mut refinement = Budget::with_iterations(256)?.guard("adaptive-fit-knot-refinement");
    loop {
        let count = knots.len() - degree - 1;
        let controls = match lsq_controls(degree, &knots, count, &u, points, weights) {
            Ok(controls) => controls,
            // A refined knot vector can make the least-squares system
            // singular (clustered knots leave a basis function without data
            // sites in its support). Like exhausting the control budget,
            // stop and report the last good fit with an honest deviation;
            // only the minimal first fit is a hard error.
            Err(_) if fitted.is_some() => break,
            Err(error) => return Err(error),
        };
        let curve = Curve {
            degree,
            knots: knots.clone(),
            control_points: controls,
            weights: vec![1.; count],
            periodic: false,
        };
        curve.validate()?;
        let deviation = max_deviation(&curve, &u, points)?;
        if deviation <= tolerance {
            converged = true;
        }
        fitted = Some((curve, deviation));
        if converged || count >= 256 {
            break;
        }
        // Refine at the worst-residual site; if its parameter coincides with
        // an existing knot, fall back to the span midpoint.
        let curve = &fitted.as_ref().unwrap().0;
        let worst = u
            .iter()
            .enumerate()
            .max_by(|(i, a), (j, b)| {
                let da = distance(
                    &curve.evaluate(**a).map(|e| e.point).unwrap_or_default(),
                    &points[*i],
                );
                let db = distance(
                    &curve.evaluate(**b).map(|e| e.point).unwrap_or_default(),
                    &points[*j],
                );
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, &u)| u)
            .unwrap();
        let mut candidate = worst;
        if knots.iter().any(|k| *k == candidate) || candidate <= 0. || candidate >= 1. {
            // Fallback: midpoint of the span containing the worst site. A
            // worst site on the clamped endpoint u = 1 matches no half-open
            // window, so start from the last non-degenerate span rather
            // than the empty [1, 1] window of the repeated end knots. If
            // the midpoint would round onto a span endpoint (an existing
            // knot), walk outward to the nearest span whose midpoint lands
            // strictly inside it, so refinement never duplicates a knot.
            let start = knots
                .windows(2)
                .position(|w| w[0] <= worst && worst < w[1])
                .or_else(|| knots.windows(2).rposition(|w| w[0] < w[1]));
            let placed = start.and_then(|start| {
                let order = std::iter::once(start)
                    .chain((0..start).rev())
                    .chain(start + 1..knots.len() - 1);
                let mut found = None;
                for i in order {
                    let (a, b) = (knots[i], knots[i + 1]);
                    let mid = 0.5 * (a + b);
                    if mid > a && mid < b {
                        found = Some(mid);
                        break;
                    }
                }
                found
            });
            candidate = placed.ok_or_else(|| {
                resource("Refinement could not place an interior knot")
            })?;
        }
        check(
            candidate > 0. && candidate < 1. && candidate.is_finite(),
            "Refinement could not place an interior knot",
        )?;
        check(
            knots.iter().filter(|k| **k == candidate).count() < degree,
            "Refinement knot would exceed interior multiplicity",
        )?;
        let slot = knots.partition_point(|k| *k < candidate);
        knots.insert(slot, candidate);
        iterations += 1;
        refinement.tick()?;
    }
    let (mut curve, mut deviation) = fitted.unwrap();
    if simplify {
        // One certified removal offer per distinct interior knot, repeated
        // while progress is made; each acceptance is bounded by `tolerance`.
        let mut simplify_budget = Budget::with_iterations(64)?.guard("adaptive-fit-knot-simplify");
        for _ in 0..64 {
            simplify_budget.tick()?;
            let mut removed = false;
            let interior: Vec<f64> = {
                let mut values: Vec<f64> = curve.knots[degree + 1..curve.control_points.len()]
                    .to_vec();
                values.dedup();
                values
            };
            for knot in interior {
                // A removal offer that cannot be certified (e.g. a singular
                // reduction system on tightly clustered knots) is simply
                // declined — simplification is best-effort and must not
                // discard a converged refinement.
                if let Ok(edit) = remove_curve_knot_report(&curve, knot, tolerance, None) {
                    if edit.certificate.accepted {
                        curve = edit.curve;
                        removed = true;
                    }
                }
            }
            if !removed {
                break;
            }
        }
        // Recompute honestly: simplification refits, so the deviation moved.
        deviation = max_deviation(&curve, &u, points)?;
        converged = deviation <= tolerance;
    }
    Ok(AdaptiveFitReport {
        curve,
        max_deviation: next_up(deviation),
        iterations,
        converged,
    })
}

#[cfg(test)]
#[path = "tests/interpolation.rs"]
mod tests;
