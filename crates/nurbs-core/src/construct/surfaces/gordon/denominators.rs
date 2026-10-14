//! Common positive Bernstein denominators for Cartesian Gordon assembly.
//! This is algebraic preparation; rounded output still requires retention audit.
use crate::{Result, check, curve::Curve, numeric, resource};

pub(crate) struct Cell {
    pub domain: [f64; 2],
    pub denominator: Vec<f64>,
    pub numerators: Vec<Vec<[f64; 3]>>,
}
fn choose(n: usize, k: usize) -> f64 {
    (0..k.min(n - k)).fold(1u64, |v, i| v * (n - i) as u64 / (i + 1) as u64) as f64
}
pub(super) fn product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let (n, m) = (a.len() - 1, b.len() - 1);
    let mut result = vec![0.; n + m + 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            result[i + j] += x * y * choose(n, i) * choose(m, j) / choose(n + m, i + j);
        }
    }
    result
}
/// Every returned numerator uses the same denominator, without requiring equal
/// authored weights at any crossing. Source knot spans are partitioned together.
pub(crate) fn prepare(curves: &[Curve]) -> Result<Vec<Cell>> {
    check(!curves.is_empty(), "Common denominator needs curves")?;
    let mut breaks = vec![0., 1.];
    for c in curves {
        c.validate()?;
        check(
            !c.periodic && c.domain() == [0., 1.] && c.control_points[0].len() == 3,
            "Common denominator needs normalized open 3D curves",
        )?;
        breaks.extend(c.knots.iter().copied().filter(|x| *x > 0. && *x < 1.));
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if breaks.len() > 257 {
        return Err(resource("Common denominator exceeds span budget"));
    }
    let mut cells = Vec::new();
    for bounds in breaks.windows(2) {
        let spans: Vec<_> = curves
            .iter()
            .map(|c| c.trim(bounds[0], bounds[1]))
            .collect::<Result<_>>()?;
        let mut denominators = Vec::new();
        let mut numerators = Vec::new();
        for span in &spans {
            check(
                span.control_points.len() == span.degree + 1,
                "Common denominator partition did not isolate a Bezier span",
            )?;
            let scale = span.weights.iter().copied().fold(0., f64::max);
            let weights: Vec<_> = span.weights.iter().map(|w| w / scale).collect();
            let constant = weights.iter().all(|w| *w == weights[0]);
            denominators.push(if constant {
                vec![weights[0]]
            } else {
                weights.clone()
            });
            numerators.push(
                span.control_points
                    .iter()
                    .zip(weights)
                    .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w])
                    .collect::<Vec<_>>(),
            );
        }
        let denominator_degree: usize = denominators.iter().map(|d| d.len() - 1).sum();
        let max_degree = spans
            .iter()
            .enumerate()
            .map(|(i, c)| c.degree + denominator_degree - (denominators[i].len() - 1))
            .max()
            .unwrap();
        if max_degree > 25 {
            return Err(resource("Common denominator exceeds degree 25"));
        }
        let denominator = denominators.iter().fold(vec![1.], |a, b| product(&a, b));
        numeric(
            denominator.iter().all(|w| w.is_finite() && *w > 0.),
            "Common denominator lost positive finite coefficients",
        )?;
        let mut scaled = Vec::new();
        for (i, numerator) in numerators.iter().enumerate() {
            let others = denominators
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .fold(vec![1.], |a, (_, b)| product(&a, b));
            let axes: Vec<_> = (0..3)
                .map(|axis| {
                    product(
                        &numerator.iter().map(|p| p[axis]).collect::<Vec<_>>(),
                        &others,
                    )
                })
                .collect();
            let points: Vec<_> = (0..axes[0].len())
                .map(|k| [axes[0][k], axes[1][k], axes[2][k]])
                .collect();
            numeric(
                points.iter().flatten().all(|x| x.is_finite()),
                "Common denominator numerator overflow",
            )?;
            scaled.push(points);
        }
        cells.push(Cell {
            domain: [bounds[0], bounds[1]],
            denominator,
            numerators: scaled,
        });
    }
    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn evaluate(mut controls: Vec<f64>, t: f64) -> f64 {
        while controls.len() > 1 {
            controls = controls
                .windows(2)
                .map(|p| p[0] * (1. - t) + p[1] * t)
                .collect();
        }
        controls[0]
    }
    #[test]
    fn common_denominator_preserves_incompatible_weighted_multispan_curves() {
        let a = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            weights: vec![1., 0.25, 2.],
            periodic: false,
        };
        let mut b = a.clone();
        b.weights = vec![3., 2., 0.5];
        let curves = [a, b];
        let cells = prepare(&curves).unwrap();
        assert_eq!(cells.len(), 2);
        for cell in cells {
            for sample in 0..=100 {
                let t = sample as f64 / 100.;
                let parameter = cell.domain[0] + t * (cell.domain[1] - cell.domain[0]);
                let denominator = evaluate(cell.denominator.clone(), t);
                for (curve, numerator) in curves.iter().zip(&cell.numerators) {
                    let expected = curve.evaluate(parameter).unwrap().point;
                    for axis in 0..3 {
                        let actual =
                            evaluate(numerator.iter().map(|p| p[axis]).collect(), t) / denominator;
                        assert!((actual - expected[axis]).abs() < 1e-12);
                    }
                }
            }
        }
    }
    #[test]
    fn polynomial_denominators_do_not_consume_rational_degree_budget() {
        let c = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            None,
        )
        .unwrap();
        let cells = prepare(&vec![c; 86]).unwrap();
        assert_eq!(cells[0].denominator, vec![1.]);
        assert_eq!(cells[0].numerators[0].len(), 3);
    }
}
