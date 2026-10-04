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
    report.operator_norm_upper = Some(squared.hi.max(0.).sqrt().next_up());
    report.arithmetic_error_upper = Some(0.);
    report.reason = "exact-binary-lattice-placement";
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
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
