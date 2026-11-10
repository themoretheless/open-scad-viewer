//! Whole-cell rational jets for a restricted original knot span.
//! Derivatives use the local parameter x in [0,1], not the source parameter.
//! Interval blossom extraction retains rounding from the original definition.
use crate::{
    Result, check, curve::Curve, curve_distance::restricted_controls, distance_bounds::Interval,
};

pub(crate) fn enclose(curve: &Curve, span: usize, domain: [f64; 2]) -> Result<Vec<Vec<Interval>>> {
    calculate(curve, span, domain, None)
}

pub(crate) fn endpoint(
    curve: &Curve,
    span: usize,
    domain: [f64; 2],
    end: bool,
) -> Result<Vec<Vec<Interval>>> {
    calculate(curve, span, domain, Some(end))
}

fn calculate(
    curve: &Curve,
    span: usize,
    domain: [f64; 2],
    endpoint: Option<bool>,
) -> Result<Vec<Vec<Interval>>> {
    // The owning operation validates the original curve once before traversal.
    check(
        span >= curve.degree && span < curve.control_points.len(),
        "Jet span outside curve",
    )?;
    check(
        domain[0].is_finite()
            && domain[1].is_finite()
            && domain[0] < domain[1]
            && domain[0] >= curve.knots[span]
            && domain[1] <= curve.knots[span + 1],
        "Jet cell must lie inside one nonempty knot span",
    )?;
    let mut net = restricted_controls(curve, span, Interval::new(domain[0], domain[1])?)?;
    let dimension = curve.control_points[0].len();
    let mut homogeneous = Vec::with_capacity(4);
    for order in 0..4 {
        homogeneous.push(
            (0..=dimension)
                .map(|axis| {
                    if let Some(end) = endpoint {
                        Ok(net[if end { net.len() - 1 } else { 0 }][axis])
                    } else {
                        Interval::new(
                            net.iter().map(|p| p[axis].lo).fold(f64::INFINITY, f64::min),
                            net.iter()
                                .map(|p| p[axis].hi)
                                .fold(f64::NEG_INFINITY, f64::max),
                        )
                    }
                })
                .collect::<Result<Vec<_>>>()?,
        );
        if order < 3 {
            let degree = net.len() - 1;
            net = if degree == 0 {
                vec![vec![Interval::point(0.); dimension + 1]]
            } else {
                net.windows(2)
                    .map(|pair| {
                        (0..=dimension)
                            .map(|axis| {
                                pair[1][axis]
                                    .sub(pair[0][axis])?
                                    .mul(Interval::point(degree as f64))
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?
            };
        }
    }
    let mut jets: Vec<Vec<Interval>> = Vec::with_capacity(4);
    for order in 0..4 {
        let mut jet = Vec::with_capacity(dimension);
        for axis in 0..dimension {
            let mut numerator = homogeneous[order][axis];
            for j in 1..=order {
                let coefficient = match (order, j) {
                    (2, 1) => 2.,
                    (3, 1 | 2) => 3.,
                    _ => 1.,
                };
                numerator = numerator.sub(
                    homogeneous[j][dimension]
                        .mul(jets[order - j][axis])?
                        .mul(Interval::point(coefficient))?,
                )?;
            }
            jet.push(numerator.div(homogeneous[0][dimension])?);
        }
        jets.push(jet);
    }
    // Extraction translated homogeneous coordinates by this exact source origin.
    for axis in 0..dimension {
        jets[0][axis] = jets[0][axis].add(Interval::point(
            curve.control_points[span - curve.degree][axis],
        ))?;
    }
    Ok(jets)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cubic_third_jet_and_invalid_cells() {
        let curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1. / 3., 0.],
                vec![2. / 3., 0.],
                vec![1., 1.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let jets = enclose(&curve, 3, [0.2, 0.7]).unwrap();
        let third = 6_f64 * 0.5_f64.powi(3);
        assert!(jets[3][1].lo <= third && third <= jets[3][1].hi);
        assert!(jets[3][0].lo <= 0. && 0. <= jets[3][0].hi);
        assert!(enclose(&curve, 2, [0., 1.]).is_err());
        assert!(enclose(&curve, 3, [0.2, 1.1]).is_err());
        assert!(enclose(&curve, 3, [0.5, 0.5]).is_err());
    }
    #[test]
    fn rational_jets_enclose_original_curve_on_restricted_cells() {
        let curve = Curve {
            degree: 3,
            knots: vec![-3., -3., -3., -3., 7., 7., 7., 7.],
            control_points: vec![vec![0.1, 0.3], vec![2., 4.], vec![5., -1.], vec![7., 2.]],
            weights: vec![0.1, 0.7, 0.3, 0.9],
            periodic: false,
        };
        curve.validate().unwrap();
        for domain in [[-3., 7.], [-2., 0.], [0.2, 0.9], [6.9, 7.]] {
            let jets = enclose(&curve, 3, domain).unwrap();
            for step in 0..=100 {
                let t = domain[0] + (domain[1] - domain[0]) * step as f64 / 100.;
                let evaluated = curve.evaluate(t).unwrap();
                for axis in 0..2 {
                    assert!(
                        jets[0][axis].lo <= evaluated.point[axis]
                            && evaluated.point[axis] <= jets[0][axis].hi
                    );
                    for (order, derivative) in
                        [(1, evaluated.d1.as_ref()), (2, evaluated.d2.as_ref())]
                    {
                        let value =
                            derivative.unwrap()[axis] * (domain[1] - domain[0]).powi(order as i32);
                        assert!(
                            jets[order][axis].lo <= value && value <= jets[order][axis].hi,
                            "order {order}, axis {axis}, parameter {t}, value {value}, bound {:?}",
                            jets[order][axis]
                        );
                    }
                }
            }
        }
    }
}
