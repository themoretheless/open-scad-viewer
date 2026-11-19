use super::*;

/// Deterministic uniform values in [-1, 1) (64-bit LCG).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 52) as f64 - 1.
    }
}

fn random_curve(rng: &mut Lcg, degree: usize, n: usize) -> Curve {
    let mut interior: Vec<f64> = (0..n - degree - 1)
        .map(|_| 0.5 + 0.45 * rng.next())
        .collect();
    interior.sort_by(f64::total_cmp);
    let mut knots = vec![0.; degree + 1];
    knots.extend(interior);
    knots.extend(vec![1.; degree + 1]);
    Curve {
        degree,
        knots,
        control_points: (0..n)
            .map(|_| (0..3).map(|_| 2. * rng.next()).collect())
            .collect(),
        weights: vec![1.; n],
        periodic: false,
    }
}

#[test]
fn every_curve_energy_gradient_matches_central_differences_on_random_curves() {
    let mut rng = Lcg(0x5eed_0847);
    for energy in [
        FairingEnergy::Stretch,
        FairingEnergy::Bending,
        FairingEnergy::Variation,
    ] {
        for trial in 0..6 {
            let degree = energy.derivative_order().max(3) + trial % 2;
            let curve = random_curve(&mut rng, degree, degree + 3 + trial);
            let form = assemble_energy_form(&curve, energy.derivative_order()).unwrap();
            let gradient = energy_gradient(&curve, &form);
            let base = energy_of(&curve, &form).unwrap();
            let h = 1e-6;
            for i in 0..curve.control_points.len() {
                for axis in 0..3 {
                    let mut plus = curve.clone();
                    plus.control_points[i][axis] += h;
                    let mut minus = curve.clone();
                    minus.control_points[i][axis] -= h;
                    let numeric_derivative = (energy_of(&plus, &form).unwrap()
                        - energy_of(&minus, &form).unwrap())
                        / (2. * h);
                    let analytic = gradient[i][axis];
                    let scale = analytic
                        .abs()
                        .max(numeric_derivative.abs())
                        .max(base)
                        .max(1.);
                    assert!(
                        (analytic - numeric_derivative).abs() / scale < 1e-6,
                        "{energy:?} trial {trial} ({i},{axis}): analytic={analytic} numeric={numeric_derivative}"
                    );
                }
            }
            // The faired curve is stationary in every free coordinate.
            let faired = fair_curve_report(&curve, energy, false, false)
                .unwrap()
                .curve;
            let at_rest = energy_gradient(&faired, &form);
            let scale = gradient
                .iter()
                .flatten()
                .fold(1f64, |m, g| m.max(g.abs()));
            for row in &at_rest[1..at_rest.len() - 1] {
                assert!(
                    row.iter().all(|g| g.abs() / scale < 1e-8),
                    "{energy:?} {row:?}"
                );
            }
        }
    }
}

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

#[test]
fn curvature_comb_samples_and_curve_torsion_at_work() {
    let circle = unit_circle();
    let comb = curvature_comb_samples(&circle, 9, 0.5);
    assert_eq!(comb.len(), 9);
    for (base, tip, kappa) in comb {
        assert!((kappa - 1.0).abs() < 1e-6);
        let tooth_len = ((tip[0] - base[0]).powi(2)
            + (tip[1] - base[1]).powi(2)
            + (tip[2] - base[2]).powi(2))
        .sqrt();
        assert!((tooth_len - 0.5).abs() < 1e-6);
    }
    // Planar circle has zero torsion; 3D noisy cubic has non-zero torsion.
    assert_eq!(curve_torsion_at(&circle, 0.5).unwrap(), 0.0);
    let tau = curve_torsion_at(&noisy_cubic(), 0.25).unwrap();
    assert!(tau.is_finite() && tau.abs() > 1e-3);
}
