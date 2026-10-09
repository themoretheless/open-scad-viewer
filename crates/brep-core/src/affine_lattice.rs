//! Sufficient exact-placement certificate for binary-lattice B-reps.
//! Unsupported arithmetic refuses; geometric admission is a separate audit.
use crate::{Model, Result, invalid};
use nurbs_core::interval_eval::Interval;
pub struct Report {
    pub model: Option<Model>,
    pub operator_norm_upper: Option<f64>,
    pub arithmetic_error_upper: Option<f64>,
    pub work: u64,
    pub reason: &'static str,
}
/// Transport a proposal only; no source certificate is inherited.
pub fn transport_projection(
    projection: [[i8; 3]; 2],
    matrix: [[f64; 4]; 4],
) -> Option<[[i8; 3]; 2]> {
    let [a, b, c] = std::array::from_fn::<_, 3, _>(|i| [matrix[i][0], matrix[i][1], matrix[i][2]]);
    let determinant = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    if !determinant.is_finite() || determinant == 0. {
        return None;
    }
    let inverse = [
        [
            b[1] * c[2] - b[2] * c[1],
            a[2] * c[1] - a[1] * c[2],
            a[1] * b[2] - a[2] * b[1],
        ],
        [
            b[2] * c[0] - b[0] * c[2],
            a[0] * c[2] - a[2] * c[0],
            a[2] * b[0] - a[0] * b[2],
        ],
        [
            b[0] * c[1] - b[1] * c[0],
            a[1] * c[0] - a[0] * c[1],
            a[0] * b[1] - a[1] * b[0],
        ],
    ];
    let values: [[f64; 3]; 2] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (0..3)
                .map(|k| projection[i][k] as f64 * inverse[k][j] / determinant)
                .sum()
        })
    });
    let maximum = values.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    if !maximum.is_finite() || maximum == 0. {
        return None;
    }
    let scale = if maximum > 64. { 64. / maximum } else { 1. };
    let candidate = values.map(|row| row.map(|x| (x * scale).round() as i8));
    let independent = (0..3).any(|k| {
        candidate[0][k] as i16 * candidate[1][(k + 1) % 3] as i16
            != candidate[1][k] as i16 * candidate[0][(k + 1) % 3] as i16
    });
    independent.then_some(candidate)
}
pub fn place(model: &Model, matrix: [[f64; 4]; 4], quantum: f64, max_work: u64) -> Result<Report> {
    model.validate()?;
    if !quantum.is_finite()
        || quantum <= 0.
        || max_work > 1_000_000
        || matrix.iter().flatten().any(|x| !x.is_finite())
        || matrix[3] != [0., 0., 0., 1.]
    {
        return Err(invalid("Invalid exact affine lattice request"));
    }
    let bits = quantum.to_bits();
    // Normal powers of two keep the integer arithmetic and rescaling proof simple.
    if bits & ((1u64 << 52) - 1) != 0 || (bits >> 52) & 2047 == 0 {
        return Err(invalid(
            "Affine lattice quantum must be a normal power of two",
        ));
    }
    let mut report = Report {
        model: None,
        operator_norm_upper: None,
        arithmetic_error_upper: None,
        work: 0,
        reason: "work-limit",
    };
    let mut squared = Interval::point(0.);
    for row in &matrix[..3] {
        if row[..3].iter().any(|x| x.fract() != 0. || x.abs() > 16.) {
            report.reason = "unsupported-matrix";
            return Ok(report);
        }
        let n = row[3] / quantum;
        if !n.is_finite() || n.fract() != 0. || n.abs() >= 2_f64.powi(46) || n * quantum != row[3] {
            report.reason = "translation-lattice";
            return Ok(report);
        }
        for &x in &row[..3] {
            squared = squared.add(Interval::point(x).mul(Interval::point(x))?)?;
        }
    }
    // Integer entries bounded by 16 make this determinant exact in binary64.
    let [a, b, c] = std::array::from_fn::<_, 3, _>(|i| [matrix[i][0], matrix[i][1], matrix[i][2]]);
    let determinant = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    if determinant == 0. {
        report.reason = "singular-matrix";
        return Ok(report);
    }
    // A signed axis permutation with zero translation is exact for every
    // finite binary64 coordinate, independently of the requested lattice.
    let signed_permutation = matrix[..3].iter().all(|row| {
        row[3] == 0.
            && row[..3].iter().filter(|&&x| x != 0.).count() == 1
            && row[..3].iter().all(|&x| matches!(x, -1. | 0. | 1.))
    });
    let points = model
        .vertices
        .iter()
        .map(|v| v.point.as_slice())
        .chain(
            model
                .edges
                .iter()
                .flat_map(|e| e.curve.control_points.iter().map(Vec::as_slice)),
        )
        .chain(
            model
                .faces
                .iter()
                .flat_map(|f| f.surface.control_points.iter().flatten().map(Vec::as_slice)),
        );
    for p in points {
        let mut n = [0.; 3];
        for k in 0..3 {
            if report.work == max_work {
                return Ok(report);
            }
            report.work += 1;
            if signed_permutation {
                continue;
            }
            n[k] = p[k] / quantum;
            if !n[k].is_finite()
                || n[k].fract() != 0.
                || n[k].abs() >= 2_f64.powi(46)
                || n[k] * quantum != p[k]
            {
                report.reason = "point-lattice";
                return Ok(report);
            }
        }
        for row in &matrix[..3] {
            if report.work == max_work {
                return Ok(report);
            }
            report.work += 1;
            if signed_permutation {
                continue;
            }
            // Each integer product is below 2^50, their sum below 2^52.
            let expected = row[3] / quantum + (0..3).map(|k| row[k] * n[k]).sum::<f64>();
            let actual = row[3] + (0..3).map(|k| row[k] * p[k]).sum::<f64>();
            if !actual.is_finite() || actual / quantum != expected || expected * quantum != actual {
                report.reason = "numeric-range";
                return Ok(report);
            }
        }
    }
    // This also rejects singular/unresolved matrices and handles reflected uses.
    let transformed = crate::transform::affine(model, matrix)?;
    report.model = Some(transformed);
    // Signed axis permutations preserve Euclidean distance exactly. The
    // generic Frobenius bound would unnecessarily inflate source error.
    report.operator_norm_upper = Some(if signed_permutation {
        1.
    } else {
        squared.hi.max(0.).sqrt().next_up()
    });
    report.arithmetic_error_upper = Some(0.);
    report.reason = if signed_permutation {
        "exact-binary-axis-permutation"
    } else {
        "exact-binary-lattice-placement"
    };
    Ok(report)
}
/// Bounded integer-matrix placement for non-lattice source coordinates.
/// Positive rational bases extend the maximum pole displacement to the
/// complete curve/surface image. UV trims and topology are unchanged; their
/// geometric validity must still be audited on the returned actual model.
pub fn place_bounded(
    model: &Model,
    matrix: [[f64; 4]; 4],
    quantum: f64,
    max_work: u64,
) -> Result<Report> {
    let exact = place(model, matrix, quantum, max_work)?;
    if exact.reason != "point-lattice" {
        return Ok(exact);
    }
    let mut report = Report {
        model: None,
        operator_norm_upper: None,
        arithmetic_error_upper: None,
        work: exact.work,
        reason: "work-limit",
    };
    if model
        .edges
        .iter()
        .any(|e| e.curve.weights.iter().any(|&w| w <= 0.))
        || model
            .faces
            .iter()
            .any(|f| f.surface.weights.iter().flatten().any(|&w| w <= 0.))
    {
        report.reason = "positive-basis-unproved";
        return Ok(report);
    }
    let transformed = crate::transform::affine(model, matrix)?;
    let points = |m: &Model| {
        m.vertices
            .iter()
            .map(|v| v.point.to_vec())
            .chain(
                m.edges
                    .iter()
                    .flat_map(|e| e.curve.control_points.iter().cloned()),
            )
            .chain(
                m.faces
                    .iter()
                    .flat_map(|f| f.surface.control_points.iter().flatten().cloned()),
            )
            .collect::<Vec<_>>()
    };
    let mut maximum: f64 = 0.;
    for (source, actual) in points(model).iter().zip(points(&transformed)) {
        let mut distance = Interval::point(0.);
        for i in 0..3 {
            if report.work >= max_work {
                return Ok(report);
            }
            report.work += 1;
            let mut ideal = Interval::point(matrix[i][3]);
            for k in 0..3 {
                ideal =
                    ideal.add(Interval::point(matrix[i][k]).mul(Interval::point(source[k]))?)?;
            }
            let difference = ideal.sub(Interval::point(actual[i]))?;
            let upper = difference.lo.abs().max(difference.hi.abs());
            distance = distance.add(Interval::point(upper).mul(Interval::point(upper))?)?;
        }
        maximum = maximum.max(distance.hi.max(0.).sqrt().next_up());
    }
    let mut squared = Interval::point(0.);
    for row in &matrix[..3] {
        for &x in &row[..3] {
            squared = squared.add(Interval::point(x).mul(Interval::point(x))?)?;
        }
    }
    if !maximum.is_finite() {
        report.reason = "numeric-range";
        return Ok(report);
    }
    report.model = Some(transformed);
    report.operator_norm_upper = Some(squared.hi.max(0.).sqrt().next_up());
    report.arithmetic_error_upper = Some(maximum);
    report.reason = "bounded-binary-affine-placement";
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_off_lattice_shear_charges_rounding_and_refuses_work() {
        let model = crate::cuboid([0.1, 0.2, 0.3], [0.6, 0.7, 1.1]).unwrap();
        let matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [1., 1., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let report = place_bounded(&model, matrix, 2_f64.powi(-40), 100000).unwrap();
        assert_eq!(report.reason, "bounded-binary-affine-placement");
        assert!(report.model.is_some());
        assert!(report.arithmetic_error_upper.unwrap() > 0.);
        assert!(report.arithmetic_error_upper.unwrap() < 1e-12);
        assert!(
            place_bounded(&model, matrix, 2_f64.powi(-40), report.work - 1)
                .unwrap()
                .model
                .is_none()
        );
        let premises = crate::sweep_affine_boundary::Premises {
            wall: Some(0.001),
            caps: Some([0.001; 2]),
            closed: false,
            source_budget: Some(0.01),
        };
        let bounded = crate::sweep_affine_boundary::place(
            &model,
            &premises,
            matrix,
            2_f64.powi(-40),
            100000,
            Some(0.01),
        )
        .unwrap();
        assert!(
            bounded.boundary.unwrap().error_upper.unwrap()
                >= report.arithmetic_error_upper.unwrap()
        );
        let refused = crate::sweep_affine_boundary::place(
            &model,
            &premises,
            matrix,
            2_f64.powi(-40),
            100000,
            Some(0.),
        )
        .unwrap();
        assert!(refused.placement.unwrap().model.is_none());
    }
    #[test]
    fn off_lattice_axis_reflection_is_exact_and_budgeted() {
        let model = crate::cuboid([0.1, 0.2, 0.3], [0.6, 0.7, 1.1]).unwrap();
        let matrix = [
            [0., -1., 0., 0.],
            [1., 0., 0., 0.],
            [0., 0., -1., 0.],
            [0., 0., 0., 1.],
        ];
        let report = place(&model, matrix, 2_f64.powi(-40), 100000).unwrap();
        assert_eq!(report.reason, "exact-binary-axis-permutation");
        assert_eq!(report.operator_norm_upper, Some(1.));
        assert_eq!(report.arithmetic_error_upper, Some(0.));
        let actual = report.model.unwrap();
        for (source, target) in model.vertices.iter().zip(&actual.vertices) {
            assert_eq!(
                target.point,
                [-source.point[1], source.point[0], -source.point[2]]
            );
        }
        assert_ne!(
            model.shells[0].faces[0].reversed,
            actual.shells[0].faces[0].reversed
        );
        assert!(
            place(&model, matrix, 2_f64.powi(-40), report.work - 1)
                .unwrap()
                .model
                .is_none()
        );
    }
    #[test]
    fn exact_shear_reflection_and_atomic_refusal() {
        let model = crate::cuboid([0., 0., 0.], [0.5, 0.25, 10.]).unwrap();
        let matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [1., 1., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let r = place(&model, matrix, 2_f64.powi(-20), 100000).unwrap();
        assert!(r.model.is_some());
        assert_eq!(r.arithmetic_error_upper, Some(0.));
        assert!(r.operator_norm_upper.unwrap() >= 5_f64.sqrt());
        assert!(
            place(&model, matrix, 2_f64.powi(-20), r.work - 1)
                .unwrap()
                .model
                .is_none()
        );
        let off_grid = crate::cuboid([0.1, 0., 0.], [0.6, 0.25, 10.]).unwrap();
        assert_eq!(
            place(&off_grid, matrix, 2_f64.powi(-20), 100000)
                .unwrap()
                .reason,
            "point-lattice"
        );
        let mut fractional = matrix;
        fractional[2][0] = 0.1;
        assert_eq!(
            place(&model, fractional, 2_f64.powi(-20), 100000)
                .unwrap()
                .reason,
            "unsupported-matrix"
        );
        let mut reflection = matrix;
        reflection[0][0] = -1.;
        let reflected = place(&model, reflection, 2_f64.powi(-20), 100000)
            .unwrap()
            .model
            .unwrap();
        assert_ne!(
            model.shells[0].faces[0].reversed,
            reflected.shells[0].faces[0].reversed
        );
    }
}
