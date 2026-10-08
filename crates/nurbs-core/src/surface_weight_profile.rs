//! Positional error bound for an exact separable original rational weight profile.
use crate::{Result, check, interval_eval::Interval as I, surface::Surface};
/// A proposal certificate, not a replacement Surface or a B-rep admission.
/// The target weights are the exact expressions row[i]*column[j]/pivot;
/// flattening those expressions to binary64 is not covered by this certificate.
pub struct WeightProfile<'a> {
    source: &'a Surface,
    row: Vec<f64>,
    column: Vec<f64>,
    pivot: f64,
    axis_deviation_upper: [f64; 3],
    weight_deviation_upper: f64,
}
impl WeightProfile<'_> {
    pub fn source(&self) -> &Surface {
        self.source
    }
    pub fn factors(&self) -> (&[f64], &[f64], f64) {
        (&self.row, &self.column, self.pivot)
    }
    pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
    pub fn weight_deviation_upper(&self) -> f64 {
        self.weight_deviation_upper
    }
    pub fn within_position_tolerance(&self, tolerance: f64) -> Result<bool> {
        check(
            tolerance.is_finite() && tolerance >= 0.,
            "Use a finite nonnegative positional tolerance",
        )?;
        let mut square = I::point(0.);
        for x in self.axis_deviation_upper {
            square = square.add(I::point(x).mul(I::point(x))?)?;
        }
        let required = I::point(tolerance).mul(I::point(tolerance))?;
        Ok(square.hi <= required.lo)
    }
}
/// Cover every original control. Whole-domain positional deviation follows from
/// positive rational basis weights, without sampling or derivative assumptions.
pub fn qualify_weight_profile<'a>(
    s: &'a Surface,
    anchor: [usize; 2],
    max_controls: usize,
) -> Result<(Option<WeightProfile<'a>>, usize)> {
    s.validate()?;
    check(
        max_controls <= 1_000_000 && anchor[0] < s.weights.len() && anchor[1] < s.weights[0].len(),
        "Choose an original profile anchor and bounded control work",
    )?;
    if s.periodic_u || s.periodic_v {
        return Ok((None, 0));
    }
    let row: Vec<_> = s.weights.iter().map(|r| r[anchor[1]]).collect();
    let column = s.weights[anchor[0]].clone();
    let pivot = s.weights[anchor[0]][anchor[1]];
    let mut delta = 0_f64;
    let mut lower = f64::INFINITY;
    let mut ranges = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spent = 0;
    for i in 0..row.len() {
        for j in 0..column.len() {
            if spent == max_controls {
                return Ok((None, spent));
            }
            let target = I::point(row[i])
                .mul(I::point(column[j]))?
                .div(I::point(pivot))?;
            let difference = target.sub(I::point(s.weights[i][j]))?;
            delta = delta.max(difference.lo.abs()).max(difference.hi.abs());
            lower = lower.min(target.lo);
            for k in 0..3 {
                let coordinate = s.control_points[i][j][k];
                ranges[k][0] = ranges[k][0].min(coordinate);
                ranges[k][1] = ranges[k][1].max(coordinate);
            }
            spent += 1;
        }
    }
    if lower <= 0. {
        return Ok((None, spent));
    }
    // T-S = sum B_ij (targetWeight-originalWeight)(P_ij-S)/targetW.
    // Positive original weights place S in the original control convex hull;
    // sum B_ij=1 and targetW>=min(targetWeight). No chart samples enter proof.
    let mut axis_deviation_upper = [0.; 3];
    for k in 0..3 {
        let extent = I::point(ranges[k][1]).sub(I::point(ranges[k][0]))?;
        axis_deviation_upper[k] = extent.mul(I::point(delta))?.div(I::point(lower))?.hi;
    }
    Ok((
        Some(WeightProfile {
            source: s,
            row,
            column,
            pivot,
            axis_deviation_upper,
            weight_deviation_upper: delta,
        }),
        spent,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn tiny_weight_change_is_bounded_without_mutating_source() {
        let mut s = surface();
        s.weights[1][1] = 1_f64.next_up();
        let original = s.weights.clone();
        let (proof, spent) = qualify_weight_profile(&s, [0, 0], 4).unwrap();
        let proof = proof.unwrap();
        assert_eq!(spent, 4);
        assert!(std::ptr::eq(proof.source(), &s));
        assert!(proof.within_position_tolerance(1e-12).unwrap());
        assert!(!proof.within_position_tolerance(0.).unwrap());
        assert!(proof.within_position_tolerance(f64::NAN).is_err());
        let (row, column, pivot) = proof.factors();
        assert_eq!(row, &[1., 1.]);
        assert_eq!(column, &[1., 1.]);
        assert_eq!(pivot, 1.);
        let bound = proof.axis_deviation_upper();
        for u in [0.1, 0.4, 0.9] {
            for v in [0.1, 0.4, 0.9] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] - u).abs() <= bound[0] && (p[1] - v).abs() <= bound[1]);
            }
        }
        assert_eq!(s.weights, original);
    }
    #[test]
    fn missing_controls_and_large_changes_do_not_pass_tolerance() {
        let mut s = surface();
        s.weights[1][1] = 2.;
        let (proof, spent) = qualify_weight_profile(&s, [0, 0], 3).unwrap();
        assert!(proof.is_none());
        assert_eq!(spent, 3);
        let proof = qualify_weight_profile(&s, [0, 0], 4).unwrap().0.unwrap();
        assert!(!proof.within_position_tolerance(1e-7).unwrap());
        assert!(qualify_weight_profile(&s, [2, 0], 4).is_err());
        assert!(qualify_weight_profile(&s, [0, 0], 0).unwrap().0.is_none());
    }
}
