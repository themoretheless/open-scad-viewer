//! Exact polynomial minima on actual algebraic original carrier intervals.
use super::*;
use branch_neighbourhood::{ParameterValue, local};
use std::cmp::Ordering;
fn cut_sign(g: &Poly, domain: &[Q; 2], cut: &ParameterValue, refinements: usize) -> Result<i8> {
    let p = local(&cut.value, domain);
    let value = event_coordinates::compose(g, &p, &cut.primitive)?;
    event_coordinates::sign_at(&value, &cut.primitive, &cut.interval, refinements)
}
pub(in crate::profile_certificate) fn prove(
    cells: &[(Vec<[Q; 3]>, [Q; 2])],
    bounds: &[ParameterValue; 2],
    chart: &[[Q; 2]; 2],
    refinements: usize,
    matrix: usize,
) -> Result<Vec<(usize, [usize; 4])>> {
    if refinements > 512 || !(1..=128).contains(&matrix) || chart.iter().any(|a| a[0] >= a[1]) {
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
        let lower_cut = shore_carrier::bound_sign(&bounds[0], &domain[0], refinements)? > 0;
        let upper_cut = shore_carrier::bound_sign(&bounds[1], &domain[1], refinements)? < 0;
        let w = power(net, 2);
        let mut counts = [0; 4];
        for axis in 0..2 {
            for side in 0..2 {
                let x = power(net, axis);
                let bound = w.iter().map(|w| w * &chart[axis][side]).collect::<Poly>();
                let g = if side == 0 {
                    sub(&x, &bound)
                } else {
                    sub(&bound, &x)
                };
                let lower = if lower_cut {
                    cut_sign(&g, domain, &bounds[0], refinements)?
                } else {
                    let value = at(&g, &Q::zero());
                    if value < Q::zero() { -1 } else { 1 }
                };
                let upper = if upper_cut {
                    cut_sign(&g, domain, &bounds[1], refinements)?
                } else {
                    let value = at(&g, &Q::one());
                    if value < Q::zero() { -1 } else { 1 }
                };
                if lower < 0 || upper < 0 {
                    return Err(refused(
                        "Original algebraic carrier endpoint lies outside the actual surface chart",
                    ));
                }
                let critical = derivative(&g);
                if !critical.is_empty() {
                    for root in algebraic_events::isolates(
                        &critical,
                        Q::zero(),
                        Q::one(),
                        &Q::new(1.into(), 1024.into()),
                    )? {
                        let lo = selected_winding::compare(
                            &critical,
                            &root,
                            domain,
                            &bounds[0],
                            refinements,
                            matrix,
                        )?;
                        let hi = selected_winding::compare(
                            &critical,
                            &root,
                            domain,
                            &bounds[1],
                            refinements,
                            matrix,
                        )?;
                        if lo == Ordering::Less || hi == Ordering::Greater {
                            continue;
                        }
                        counts[2 * axis + side] += 1;
                        if event_coordinates::sign_at(&g, &critical, &root, refinements)? < 0 {
                            return Err(refused(
                                "Original carrier has an interior excursion outside the actual surface chart",
                            ));
                        }
                    }
                }
            }
        }
        result.push((index, counts));
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
