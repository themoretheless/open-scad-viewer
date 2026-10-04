//! Curve fairing by quadratic energy minimization plus curvature-comb
//! measurement reports.
//!
//! Energies (items 207-209): `Stretch` assembles ∫‖r′‖²du, `Bending`
//! assembles ∫‖r″‖²du as the standard linearization of ∫κ²ds (exact for
//! arc-length parameterizations, the classical thin-plate surrogate
//! otherwise), and `Variation` assembles ∫‖r‴‖²du as the analogous
//! linearization of ∫(dκ/ds)²ds. For rational curves the same polynomial
//! quadratic form in the (non-homogeneous) control points is used; this is
//! a documented linearization, not an exact rational energy. Each quadratic
//! form A[i][j] = ∫ N_i^(q) N_j^(q) du is assembled by 4-point Gauss-Legendre
//! quadrature per active knot span, exact for the polynomial integrands of
//! degree 2(p-q) ≤ 2p whenever 4 Gauss points suffice (p ≤ q + 3); for
//! higher degrees it remains a consistent, documented quadrature
//! approximation. Derivative rows of order up to 3 are computed by the
//! recursion D^(r)_{i,d} = d·(D^(r-1)_{i,d-1}/Δ(i,d) − D^(r-1)_{i+1,d-1}/Δ(i+1,d))
//! with Δ(i,d) = knots[i+d] − knots[i] (0/0 defined as 0).
//!
//! `fair_curve_report` eliminates the pinned end control points exactly
//! (positions always; with `pin_tangents` also the adjacent control legs,
//! fixing end tangent direction and magnitude; with `pin_g2` also the next
//! legs, fixing end second derivatives) and solves the resulting
//! unconstrained symmetric quadratic system with the dense foundation
//! solver. Knots and weights are never modified, so a validated input
//! revalidates after fairing. Before/after energies are evaluated with the
//! identical assembled form, so the comparison is honest.
use crate::{
    Result, check,
    curve::{Curve, basis_funs_ders},
    curve_analysis::curvature_extrema,
    curve_differential::{self, Side},
    foundation::fitting::dense_solve,
    numeric,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FairingEnergy {
    /// ∫‖r′‖²du.
    Stretch,
    /// ∫‖r″‖²du, linearized ∫κ²ds.
    Bending,
    /// ∫‖r‴‖²du, linearized ∫(dκ/ds)²ds.
    Variation,
}

impl FairingEnergy {
    fn derivative_order(self) -> usize {
        match self {
            FairingEnergy::Stretch => 1,
            FairingEnergy::Bending => 2,
            FairingEnergy::Variation => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CombSample {
    pub u: f64,
    /// Certified curvature interval when resolvable, otherwise a degenerate
    /// point estimate interval from the local jet evaluation.
    pub kappa: [f64; 2],
    /// Finite-difference dκ/ds approximation across neighbouring samples.
    pub dkappa: f64,
}

#[derive(Clone, Debug)]
pub struct FairnessReport {
    /// Faired curve (or a clone of the input for measure-only reports).
    pub curve: Curve,
    pub energy_before: f64,
    pub energy_after: f64,
    /// Certified enclosure of the maximum curvature over detected curvature
    /// extrema and both domain endpoints.
    pub max_curvature: [f64; 2],
    pub comb_samples: Vec<CombSample>,
}

/// 4-point Gauss-Legendre nodes/weights on [0,1].
const GAUSS4: [(f64, f64); 4] = [
    (0.06943184420297371, 0.17392742256872692),
    (0.33000947820757187, 0.32607257743127305),
    (0.6699905217924281, 0.32607257743127305),
    (0.9305681557970262, 0.17392742256872692),
];

/// Full-width basis derivative row of the given order (0..=3) at `u`, which
/// must lie on the non-empty span `span`. Orders above the degree are
/// identically zero; order 3 requires degree ≥ 1 spans but only degrees ≥ 3
/// give a nonzero row.
fn derivative_row(
    degree: usize,
    knots: &[f64],
    span: usize,
    u: f64,
    order: usize,
) -> Result<Vec<f64>> {
    let p = degree;
    let nn = order.min(p);
    let base = p - nn;
    // Value rows for degrees base..=p, each full width for its degree.
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(nn + 1);
    for d in base..=p {
        let width = knots.len() - d - 1;
        let mut row = vec![0.; width];
        if d == 0 {
            // Degree-0 basis is the piecewise-constant span indicator.
            check(span < width, "Span is outside the valid knot range")?;
            row[span] = 1.;
        } else {
            let local = basis_funs_ders(d, knots, span, u, 0)?;
            for (j, &value) in local.values.iter().enumerate() {
                row[span - d + j] = value;
            }
        }
        rows.push(row);
    }
    let mut first_degree = base;
    for _ in 0..nn {
        let mut next: Vec<Vec<f64>> = Vec::with_capacity(rows.len() - 1);
        for j in 1..rows.len() {
            // After each level, rows[0] holds one degree higher.
            let d = first_degree + j;
            let previous = &rows[j - 1];
            let width = knots.len() - d - 1;
            let mut out = vec![0.; width];
            for (i, value) in out.iter_mut().enumerate() {
                let left = knots[i + d] - knots[i];
                let right = knots[i + d + 1] - knots[i + 1];
                let a = if left > 0. { previous[i] / left } else { 0. };
                let b = if right > 0. { previous[i + 1] / right } else { 0. };
                *value = d as f64 * (a - b);
            }
            next.push(out);
        }
        rows = next;
        first_degree += 1;
    }
    let row = rows
        .pop()
        .ok_or_else(|| crate::numeric_err("Derivative row assembly failed"))?;
    numeric(
        row.iter().all(|v| v.is_finite()),
        "Knot scale exhausted finite derivative precision",
    )?;
    Ok(row)
}

/// Assemble the quadratic form A[i][j] = ∫ N_i^(q) N_j^(q) du by 4-point
/// Gauss quadrature over every active knot span.
fn assemble_energy_form(curve: &Curve, order: usize) -> Result<Vec<Vec<f64>>> {
    let n = curve.control_points.len();
    let mut form = vec![vec![0.; n]; n];
    for span in curve.degree..n {
        let lo = curve.knots[span];
        let hi = curve.knots[span + 1];
        if lo >= hi {
            continue;
        }
        for &(node, weight) in &GAUSS4 {
            let u = lo + (hi - lo) * node;
            let row = derivative_row(curve.degree, &curve.knots, span, u, order)?;
            let scale = weight * (hi - lo);
            for i in 0..n {
                if row[i] == 0. {
                    continue;
                }
                for j in 0..n {
                    form[i][j] += scale * row[i] * row[j];
                }
            }
        }
    }
    Ok(form)
}

/// Energy value xᵀAx summed over coordinate axes, using the identical
/// assembled form for before/after honesty.
fn energy_of(curve: &Curve, form: &[Vec<f64>]) -> Result<f64> {
    let n = curve.control_points.len();
    let dimension = curve.control_points[0].len();
    let mut total = 0.;
    for axis in 0..dimension {
        for i in 0..n {
            let xi = curve.control_points[i][axis];
            if xi == 0. {
                continue;
            }
            let mut dot = 0.;
            for j in 0..n {
                dot += form[i][j] * curve.control_points[j][axis];
            }
            total += xi * dot;
        }
    }
    numeric(total.is_finite(), "Fairing energy exhausted numeric range")?;
    // Semi-definite form: roundoff may produce a tiny negative value.
    Ok(total.max(0.))
}

fn clamped(curve: &Curve) -> bool {
    let n = curve.control_points.len();
    let p = curve.degree;
    curve.knots[0] == curve.knots[p] && curve.knots[n] == curve.knots[n + p]
}

/// Point-estimate curvature from the local evaluation jet; `None` when the
/// second derivative is unavailable (insufficient continuity) or the speed
/// vanishes.
fn kappa_point(curve: &Curve, u: f64) -> Result<Option<f64>> {
    let evaluation = curve.evaluate(u)?;
    let (Some(d1), Some(d2)) = (evaluation.d1, evaluation.d2) else {
        return Ok(None);
    };
    let cross = [
        d1[1] * d2[2] - d1[2] * d2[1],
        d1[2] * d2[0] - d1[0] * d2[2],
        d1[0] * d2[1] - d1[1] * d2[0],
    ];
    let speed2: f64 = d1.iter().map(|x| x * x).sum();
    if speed2 <= 0. {
        return Ok(None);
    }
    Ok(Some(
        (cross.iter().map(|x| x * x).sum::<f64>()).sqrt() / speed2.powf(1.5),
    ))
}

/// Certified curvature interval at `u`, falling back to a degenerate point
/// interval when the certified query cannot resolve curvature (knot
/// continuity or degenerate speed).
fn kappa_interval(curve: &Curve, u: f64) -> Result<Option<[f64; 2]>> {
    if let Ok(report) = curve_differential::at(curve, u, Side::Automatic) {
        if let Some(kappa) = report.curvature {
            return Ok(Some(kappa));
        }
    }
    Ok(kappa_point(curve, u)?.map(|value| [value, value]))
}

/// Certified enclosure of the maximum curvature: union over detected
/// curvature extrema and both domain endpoints.
fn max_curvature(curve: &Curve) -> Result<[f64; 2]> {
    let [a, b] = curve.domain();
    let tolerance = ((b - a) * 1e-9).max(1e-12);
    let mut lo = 0_f64;
    let mut hi = 0_f64;
    let mut found = false;
    if let Ok(extrema) = curvature_extrema(curve, tolerance) {
        for extremum in extrema {
            lo = lo.max(extremum.kappa[0]);
            hi = hi.max(extremum.kappa[1]);
            found = true;
        }
    }
    for u in [a, b] {
        if let Some(kappa) = kappa_interval(curve, u)? {
            lo = lo.max(kappa[0]);
            hi = hi.max(kappa[1]);
            found = true;
        }
    }
    numeric(found, "No curvature sample could be resolved")?;
    Ok([lo, hi])
}

fn nudge_off_knot(curve: &Curve, mut u: f64) -> f64 {
    for _ in 0..8 {
        if curve.knots.iter().any(|&k| k == u) && u < curve.domain()[1] {
            u = u.next_up();
        } else {
            break;
        }
    }
    u
}

fn comb_samples(curve: &Curve, samples: usize) -> Result<Vec<CombSample>> {
    let [a, b] = curve.domain();
    let mut parameters = Vec::with_capacity(samples);
    let mut kappas: Vec<[f64; 2]> = Vec::with_capacity(samples);
    for i in 0..samples {
        // Cell midpoints keep samples away from domain endpoints and make
        // exact knot hits unlikely; an exact hit is still nudged off.
        let u = nudge_off_knot(curve, a + (b - a) * (2 * i + 1) as f64 / (2 * samples) as f64);
        parameters.push(u);
        kappas.push(kappa_interval(curve, u)?.ok_or_else(|| {
            crate::numeric_err("Curvature comb sample could not be resolved")
        })?);
    }
    let mut points = Vec::with_capacity(samples);
    for &u in &parameters {
        points.push(curve.evaluate(u)?.point);
    }
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let kappa = kappas[i];
        // Central dκ/ds approximation with arc-length chord estimates;
        // one-sided at the first/last sample.
        let (i0, i1) = if i == 0 {
            (0, 1.min(samples - 1))
        } else if i + 1 == samples {
            (samples - 2, samples - 1)
        } else {
            (i - 1, i + 1)
        };
        let dkappa = if i0 == i1 {
            0.
        } else {
            let dk = (kappas[i1][0] + kappas[i1][1]) * 0.5
                - (kappas[i0][0] + kappas[i0][1]) * 0.5;
            let ds: f64 = points[i0]
                .iter()
                .zip(&points[i1])
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt();
            if ds > 0. { dk / ds } else { 0. }
        };
        numeric(dkappa.is_finite(), "Curvature variation exceeded range")?;
        out.push(CombSample {
            u: parameters[i],
            kappa,
            dkappa,
        });
    }
    Ok(out)
}

/// Measurement-only fairness report: curvature comb, certified maximum
/// curvature enclosure, and the (linearized) bending energy reported in both
/// energy fields since no fairing is performed.
pub fn measure_fairness_report(curve: &Curve, samples: usize) -> Result<FairnessReport> {
    curve.validate()?;
    check(
        (2..=256).contains(&samples),
        "Fairness comb needs 2..=256 samples",
    )?;
    check(
        curve.degree >= 2,
        "Fairness measurement needs a curve of degree at least 2",
    )?;
    let form = assemble_energy_form(curve, FairingEnergy::Bending.derivative_order())?;
    let energy = energy_of(curve, &form)?;
    Ok(FairnessReport {
        curve: curve.clone(),
        energy_before: energy,
        energy_after: energy,
        max_curvature: max_curvature(curve)?,
        comb_samples: comb_samples(curve, samples)?,
    })
}

/// Fair a clamped, non-periodic curve by minimizing the chosen quadratic
/// energy over the unpinned control points. End control points are always
/// pinned (positions); `pin_tangents` additionally pins the adjacent legs
/// (end tangent direction and magnitude); `pin_g2` additionally pins the
/// second legs (end second derivatives). The reduced symmetric system is
/// solved with the dense foundation solver; a singular reduced system (too
/// few free controls for the chosen energy) is reported as a numeric error,
/// never silently regularized.
pub fn fair_curve_report(
    curve: &Curve,
    energy: FairingEnergy,
    pin_tangents: bool,
    pin_g2: bool,
) -> Result<FairnessReport> {
    curve.validate()?;
    check(!curve.periodic, "Fairing requires a non-periodic curve")?;
    check(
        clamped(curve),
        "Fairing requires clamped end knots so pinned control points fix end geometry",
    )?;
    let p = curve.degree;
    let n = curve.control_points.len();
    let order = energy.derivative_order();
    check(
        p >= order,
        "Curve degree is below the energy derivative order",
    )?;
    let mut pinned = vec![false; n];
    pinned[0] = true;
    pinned[n - 1] = true;
    if pin_tangents {
        check(n >= 4, "Tangent pinning needs at least 4 control points")?;
        pinned[1] = true;
        pinned[n - 2] = true;
    }
    if pin_g2 {
        check(n >= 6, "G2 pinning needs at least 6 control points")?;
        pinned[1] = true;
        pinned[n - 2] = true;
        pinned[2] = true;
        pinned[n - 3] = true;
    }
    let form = assemble_energy_form(curve, order)?;
    let energy_before = energy_of(curve, &form)?;
    let free: Vec<usize> = (0..n).filter(|&i| !pinned[i]).collect();
    let dimension = curve.control_points[0].len();
    let mut control_points = curve.control_points.clone();
    if !free.is_empty() {
        let m = free.len();
        let matrix: Vec<Vec<f64>> = (0..m)
            .map(|i| (0..m).map(|j| form[free[i]][free[j]]).collect())
            .collect();
        let rhs: Vec<Vec<f64>> = (0..m)
            .map(|i| {
                (0..dimension)
                    .map(|axis| {
                        -(0..n)
                            .filter(|&k| pinned[k])
                            .map(|k| form[free[i]][k] * curve.control_points[k][axis])
                            .sum::<f64>()
                    })
                    .collect()
            })
            .collect();
        let solved = dense_solve(matrix, rhs)?;
        for (i, &f) in free.iter().enumerate() {
            numeric(
                solved[i].iter().all(|v| v.is_finite()),
                "Fairing solve produced non-finite control data",
            )?;
            control_points[f] = solved[i].clone();
        }
    }
    let faired = Curve {
        control_points,
        ..curve.clone()
    };
    faired.validate()?;
    // Same knots and degree: the assembled form is shared by both energies.
    let energy_after = energy_of(&faired, &form)?;
    Ok(FairnessReport {
        max_curvature: max_curvature(&faired)?,
        comb_samples: comb_samples(&faired, 64)?,
        curve: faired,
        energy_before,
        energy_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn noisy_cubic() -> Curve {
        // Clamped cubic, 5 control points; interior controls perturbed off
        // the smooth line/arc they would otherwise follow.
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.3, 0.55, 0.05],
                vec![0.55, 0.35, -0.12],
                vec![0.8, 0.62, 0.09],
                vec![1., 1., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        }
    }

    fn unit_circle() -> Curve {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![-1., 1., 0.],
                vec![-1., 0., 0.],
                vec![-1., -1., 0.],
                vec![0., -1., 0.],
                vec![1., -1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
            periodic: false,
        }
    }

    #[test]
    fn fairing_noisy_cubic_improves_energy_and_preserves_ends() {
        let curve = noisy_cubic();
        let report = fair_curve_report(&curve, FairingEnergy::Bending, false, false).unwrap();
        assert!(
            report.energy_after < report.energy_before,
            "energy should drop: before={} after={}",
            report.energy_before,
            report.energy_after
        );
        assert!(report.energy_after >= 0.);
        // Clamped ends interpolate the pinned first/last control points.
        let [a, b] = curve.domain();
        let start_before = curve.evaluate(a).unwrap().point;
        let start_after = report.curve.evaluate(a).unwrap().point;
        let end_before = curve.evaluate(b).unwrap().point;
        let end_after = report.curve.evaluate(b).unwrap().point;
        for axis in 0..3 {
            assert!((start_before[axis] - start_after[axis]).abs() <= 1e-12);
            assert!((end_before[axis] - end_after[axis]).abs() <= 1e-12);
        }
        assert_eq!(report.curve.control_points[0], curve.control_points[0]);
        assert_eq!(report.curve.control_points[4], curve.control_points[4]);
    }

    #[test]
    fn fairing_stretch_and_variation_also_drop() {
        let curve = noisy_cubic();
        let stretch = fair_curve_report(&curve, FairingEnergy::Stretch, false, false).unwrap();
        assert!(stretch.energy_after < stretch.energy_before);
        let variation = fair_curve_report(&curve, FairingEnergy::Variation, false, false).unwrap();
        assert!(variation.energy_after < variation.energy_before);
    }

    #[test]
    fn measure_only_circle_reports_unit_curvature() {
        let circle = unit_circle();
        let samples = 24;
        let report = measure_fairness_report(&circle, samples).unwrap();
        assert_eq!(report.comb_samples.len(), samples);
        // κ = 1/R = 1 must lie inside the certified max enclosure.
        assert!(report.max_curvature[0] <= 1. + 1e-6);
        assert!(report.max_curvature[1] >= 1. - 1e-6);
        assert!(report.max_curvature[1] - report.max_curvature[0] < 1e-3);
        for sample in &report.comb_samples {
            assert!(sample.kappa[0] <= 1. + 1e-6 && sample.kappa[1] >= 1. - 1e-6);
            // Constant curvature: dκ/ds ≈ 0.
            assert!(sample.dkappa.abs() < 1e-4);
        }
        // Measure-only: energies identical.
        assert_eq!(report.energy_before, report.energy_after);
    }

    #[test]
    fn pin_options_are_respected() {
        let curve = noisy_cubic();
        let tangents = fair_curve_report(&curve, FairingEnergy::Bending, true, false).unwrap();
        assert_eq!(tangents.curve.control_points[1], curve.control_points[1]);
        assert_eq!(tangents.curve.control_points[3], curve.control_points[3]);
        assert!(tangents.energy_after < tangents.energy_before);

        let g2_curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1. / 3., 2. / 3., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.2, 0.4, 0.],
                vec![0.45, 0.3, 0.1],
                vec![0.6, 0.5, -0.1],
                vec![0.8, 0.7, 0.],
                vec![1., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        // With G2 pinning on a 6-control cubic only 2 controls stay free;
        // the bending system on them is definite.
        let g2 = fair_curve_report(&g2_curve, FairingEnergy::Bending, true, true).unwrap();
        assert_eq!(g2.curve.control_points[1], g2_curve.control_points[1]);
        assert_eq!(g2.curve.control_points[2], g2_curve.control_points[2]);
        assert_eq!(g2.curve.control_points[3], g2_curve.control_points[3]);
        assert_eq!(g2.curve.control_points[4], g2_curve.control_points[4]);
        assert!(g2.energy_after <= g2.energy_before + 1e-15);
    }

    #[test]
    fn fairing_rejects_unclamped_or_low_degree() {
        let mut curve = noisy_cubic();
        curve.knots = vec![-1., 0., 0., 0., 0.5, 1., 1., 1., 2.];
        assert!(fair_curve_report(&curve, FairingEnergy::Bending, false, false).is_err());
        let line = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        assert!(fair_curve_report(&line, FairingEnergy::Bending, false, false).is_err());
        assert!(measure_fairness_report(&line, 8).is_err());
        assert!(measure_fairness_report(&noisy_cubic(), 300).is_err());
    }
}

