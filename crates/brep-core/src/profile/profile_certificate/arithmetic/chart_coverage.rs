//! Exact polynomial minima on actual algebraic original carrier intervals.
use super::*;
use branch_neighbourhood::{ParameterValue, local};
use std::cmp::Ordering;
fn cut_sign(g: &Poly, domain: &[Q; 2], cut: &ParameterValue, refinements: usize) -> Result<i8> {
    let p = local(&cut.value, domain);
    let value = event_coordinates::compose(g, &p, &cut.primitive)?;
    event_coordinates::sign_at(&value, &cut.primitive, &cut.interval, refinements)
}
const MAX_REFINEMENTS: usize = 512;
const MAX_MATRIX: usize = 128;
/// The part of one cell's parameter domain between the two algebraic bounds.
/// `cut[k]` is set when bound `k` lies strictly inside the cell, so that end
/// is the bound itself and not the cell end.
struct CarrierInterval<'a> {
    domain: &'a [Q; 2],
    bounds: &'a [ParameterValue; 2],
    cut: [bool; 2],
    refinements: usize,
    matrix: usize,
}
impl CarrierInterval<'_> {
    /// Sign of `g` at one end of the interval; an exact zero at a cell end
    /// counts as inside.
    fn end_sign(&self, g: &Poly, end: usize) -> Result<i8> {
        if self.cut[end] {
            return cut_sign(g, self.domain, &self.bounds[end], self.refinements);
        }
        let local_end = if end == 0 { Q::zero() } else { Q::one() };
        Ok(if at(g, &local_end) < Q::zero() { -1 } else { 1 })
    }
    /// Number of critical points of `g` inside the interval; `g` must be
    /// nonnegative at each of them.
    fn interior_extrema(&self, g: &Poly) -> Result<usize> {
        let critical = derivative(g);
        if critical.is_empty() {
            return Ok(0);
        }
        let width = Q::new(1.into(), 1024.into());
        let mut count = 0;
        for root in algebraic_events::isolates(&critical, Q::zero(), Q::one(), &width)? {
            let order = |bound: &ParameterValue| {
                selected_winding::compare(
                    &critical,
                    &root,
                    self.domain,
                    bound,
                    self.refinements,
                    self.matrix,
                )
            };
            if order(&self.bounds[0])? == Ordering::Less
                || order(&self.bounds[1])? == Ordering::Greater
            {
                continue;
            }
            count += 1;
            if event_coordinates::sign_at(g, &critical, &root, self.refinements)? < 0 {
                return Err(refused(
                    "Original carrier has an interior excursion outside the actual surface chart",
                ));
            }
        }
        Ok(count)
    }
    /// Proves the cell stays inside the chart on this interval and counts
    /// the interior extrema against each of the four chart sides.
    fn extrema_counts(&self, net: &[[Q; 3]], chart: &[[Q; 2]; 2]) -> Result<[usize; 4]> {
        let w = power(net, 2);
        let mut counts = [0; 4];
        for axis in 0..2 {
            for side in 0..2 {
                let x = power(net, axis);
                let bound = w.iter().map(|w| w * &chart[axis][side]).collect::<Poly>();
                // Nonnegative exactly where the cell is on the chart side of this bound.
                let g = if side == 0 {
                    sub(&x, &bound)
                } else {
                    sub(&bound, &x)
                };
                let (lower, upper) = (self.end_sign(&g, 0)?, self.end_sign(&g, 1)?);
                if lower < 0 || upper < 0 {
                    return Err(refused(
                        "Original algebraic carrier endpoint lies outside the actual surface chart",
                    ));
                }
                counts[2 * axis + side] = self.interior_extrema(&g)?;
            }
        }
        Ok(counts)
    }
}
pub(in crate::profile::profile_certificate) fn prove(
    cells: &[(Vec<[Q; 3]>, [Q; 2])],
    bounds: &[ParameterValue; 2],
    chart: &[[Q; 2]; 2],
    refinements: usize,
    matrix: usize,
) -> Result<Vec<(usize, [usize; 4])>> {
    if refinements > MAX_REFINEMENTS
        || !(1..=MAX_MATRIX).contains(&matrix)
        || chart.iter().any(|a| a[0] >= a[1])
    {
        return Err(refused(
            "Invalid original chart coverage allowance or domain",
        ));
    }
    let mut result = Vec::new();
    for (index, (net, domain)) in cells.iter().enumerate() {
        if shore_carrier::bound_sign(&bounds[0], &domain[1], refinements)? >= 0
            || shore_carrier::bound_sign(&bounds[1], &domain[0], refinements)? <= 0
        {
            continue;
        }
        if net.iter().any(|p| p[2] <= Q::zero()) {
            return Err(refused(
                "Original chart cell has nonpositive rational weights",
            ));
        }
        let interval = CarrierInterval {
            domain,
            bounds,
            cut: [
                shore_carrier::bound_sign(&bounds[0], &domain[0], refinements)? > 0,
                shore_carrier::bound_sign(&bounds[1], &domain[1], refinements)? < 0,
            ],
            refinements,
            matrix,
        };
        result.push((index, interval.extrema_counts(net, chart)?));
    }
    if result.is_empty() {
        return Err(refused(
            "Original chart coverage omits every actual carrier interior",
        ));
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_algebraic_cuts_exclude_unretained_outside_extrema_and_refine_endpoints() {
        let q = |n: i64| Q::from_integer(n.into());
        let lower = ParameterValue {
            primitive: vec![q(0), q(1)],
            value: vec![q(0), q(1)],
            interval: [q(0), q(0)],
        };
        let upper = ParameterValue {
            primitive: vec![q(-1), q(0), q(32)],
            value: vec![q(0), q(1)],
            interval: [q(0), q(1)],
        };
        let cells = vec![(
            vec![
                [q(0), q(0), q(1)],
                [Q::new(1.into(), 2.into()), q(2), q(1)],
                [q(1), q(0), q(1)],
            ],
            [q(0), q(1)],
        )];
        let chart = [[q(0), q(1)], [q(0), Q::new(3.into(), 4.into())]];
        assert!(prove(&cells, &[lower.clone(), upper.clone()], &chart, 0, 64).is_err());
        let result = prove(&cells, &[lower.clone(), upper], &chart, 192, 64).unwrap();
        assert_eq!(result, vec![(0, [0; 4])]);
        let outside = ParameterValue {
            primitive: vec![q(-1), q(0), q(2)],
            value: vec![q(0), q(1)],
            interval: [q(0), q(1)],
        };
        assert!(prove(&cells, &[lower, outside], &chart, 192, 64).is_err());
    }
}
