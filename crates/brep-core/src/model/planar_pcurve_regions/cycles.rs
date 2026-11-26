use super::{Curve, PlanarOperation, Q, Result, reverse_exact, unsupported};
use crate::profile::pcurve_trim::materialize_span;
use crate::profile_certificate::{certify_rational_point_winding, exact_curve_bernstein_interval};
use nurbs_core::curve_partition_agreement::exact_bezier_span_partition;
use std::collections::BTreeMap;

type Point = [Q; 3];

fn split(mut poles: Vec<Point>, t: &Q) -> [Vec<Point>; 2] {
    let mut left = vec![poles[0].clone()];
    let mut right = vec![poles.last().unwrap().clone()];
    for width in (1..poles.len()).rev() {
        for i in 0..width {
            poles[i] = std::array::from_fn(|axis| {
                &poles[i][axis] + t * (&poles[i + 1][axis] - &poles[i][axis])
            });
        }
        left.push(poles[0].clone());
        right.push(poles[width - 1].clone());
    }
    right.reverse();
    [left, right]
}

fn homogeneous(curve: &Curve) -> Result<Vec<Point>> {
    if curve.control_points.len() != curve.degree + 1 || curve.degree > 32 {
        return Err(unsupported(
            "Curved contact needs bounded single-span fragments",
        ));
    }
    Ok(curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, &w)| {
            let w = Q::from_float(w).unwrap();
            [
                &w * Q::from_float(p[0]).unwrap(),
                &w * Q::from_float(p[1]).unwrap(),
                w,
            ]
        })
        .collect())
}

pub(crate) type CurveCuts = (Vec<Q>, Vec<([Q; 2], bool)>);

fn select_fragments(
    wires: [&[Vec<Curve>]; 2],
    cuts: [Vec<CurveCuts>; 2],
    operation: PlanarOperation,
) -> Result<Vec<Curve>> {
    let mut retained = Vec::new();
    let half = Q::new(1.into(), 2.into());
    for (side, cuts) in cuts.into_iter().enumerate() {
        let other = wires[1 - side];
        let reverse = operation == PlanarOperation::Difference && side == 1;
        for (curve, (cuts, shared)) in wires[side].iter().flatten().zip(cuts) {
            let literal = exact_bezier_span_partition(curve)?;
            let native = &curve.knots[curve.degree..=curve.control_points.len()];
            let active: Vec<_> = native.windows(2).filter(|k| k[0] < k[1]).collect();
            for (span_index, domain) in active.iter().enumerate() {
                let domain = [domain[0], domain[1]].map(|v| Q::from_float(v).unwrap());
                let mut cuts: Vec<_> = cuts
                    .iter()
                    .filter(|&t| &domain[0] < t && t < &domain[1])
                    .cloned()
                    .collect();
                cuts.extend(domain.clone());
                cuts.sort();
                cuts.dedup();
                let [lo, hi] = &domain;
                let span = hi - lo;
                let poles = if let Some(parts) = &literal {
                    homogeneous(&parts[span_index])?
                } else {
                    exact_curve_bernstein_interval(curve, lo, hi)?
                };
                for interval in cuts.windows(2) {
                    let exact = |value: &Q| (value - lo) / &span;
                    let a = exact(&interval[0]);
                    let b = exact(&interval[1]);
                    let [prefix, _] = split(poles.clone(), &b);
                    let [_, fragment] = split(prefix, &(a / b));
                    let mut common = shared
                        .iter()
                        .filter(|(range, _)| range[0] <= interval[0] && interval[1] <= range[1]);
                    let keep = if let Some((_, keep)) = common.next() {
                        if common.next().is_some() {
                            return Err(unsupported(
                                "Curved overlap has multiple owners of one shared interval",
                            ));
                        }
                        *keep
                    } else {
                        let [left, _] = split(fragment.clone(), &half);
                        let [x, y, w] = left.last().unwrap();
                        let point =
                            [x / w, y / w].map(|x| [x.numer().to_string(), x.denom().to_string()]);
                        let inside = certify_rational_point_winding(other, point)?
                            .windings
                            .iter()
                            .sum::<i64>()
                            != 0;
                        inside != (operation == PlanarOperation::Difference && side == 0)
                    };
                    if !keep {
                        continue;
                    }
                    if interval == domain
                        && let Some(parts) = &literal
                    {
                        let piece = parts[span_index].clone();
                        retained.push(if reverse {
                            reverse_exact(piece)?
                        } else {
                            piece
                        });
                        continue;
                    }
                    let piece = materialize_span(curve, &fragment, interval)?;
                    retained.push(if reverse {
                        reverse_exact(piece)?
                    } else {
                        piece
                    });
                }
            }
        }
    }
    Ok(retained)
}

fn endpoint(curve: &Curve, end: bool) -> [u64; 2] {
    let p = if end {
        curve.control_points.last().unwrap()
    } else {
        &curve.control_points[0]
    };
    [0, 1].map(|axis| if p[axis] == 0. { 0 } else { p[axis].to_bits() })
}

pub(crate) fn trace_cycles(
    wires: [&[Vec<Curve>]; 2],
    cuts: [Vec<CurveCuts>; 2],
    operation: PlanarOperation,
) -> Result<Vec<Vec<Curve>>> {
    let mut outgoing = BTreeMap::new();
    for curve in select_fragments(wires, cuts, operation)? {
        if outgoing.insert(endpoint(&curve, false), curve).is_some() {
            return Err(unsupported(
                "Curved contact boundary has ambiguous outgoing branches",
            ));
        }
    }
    let mut cycles = Vec::new();
    while let Some((start, first)) = outgoing.pop_first() {
        let mut next = endpoint(&first, true);
        let mut cycle = vec![first];
        while next != start {
            let curve = outgoing
                .remove(&next)
                .ok_or_else(|| unsupported("Curved contact boundary is open"))?;
            next = endpoint(&curve, true);
            cycle.push(curve);
        }
        cycles.push(cycle);
    }
    Ok(cycles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::planar_pcurve_regions::representable;
    use num_traits::Zero;

    #[test]
    fn fractional_bezier_weights_use_one_exact_projective_scale() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![3., 0.], vec![3., 3.]],
            weights: vec![1., 1., 4.],
            periodic: false,
        };
        let [prefix, _] = split(homogeneous(&curve).unwrap(), &Q::new(1.into(), 3.into()));
        assert!(representable(&prefix[2][2]).is_err());
        let piece =
            materialize_span(&curve, &prefix, &[Q::zero(), Q::new(1.into(), 3.into())]).unwrap();
        assert_eq!(piece.weights, [3., 3., 4.]);
        let [x, y, w] = &prefix[2];
        assert_eq!(
            [
                representable(&(x / w)).unwrap(),
                representable(&(y / w)).unwrap()
            ],
            [2., 1.]
        );
    }
}
