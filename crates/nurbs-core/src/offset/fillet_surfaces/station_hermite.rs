//! Bounded quintic station proposals. Finite differences propose derivatives;
//! they never certify source contact, smoothness or a radius law.
use super::*;

/// Proposal routing only. Affine polynomial charts already have straight
/// contact fields for a linear law; preserve their simpler cubic storage.
/// All original-contact/radius/regularity admission gates remain mandatory.
pub(super) fn affine_chart(s: &Surface) -> bool {
    s.degree_u == 1
        && s.degree_v == 1
        && s.control_points.len() == 2
        && s.control_points[0].len() == 2
        && s.weights.iter().flatten().all(|&w| w == s.weights[0][0])
        && (0..3).all(|k| {
            s.control_points[1][1][k] - s.control_points[1][0][k]
                == s.control_points[0][1][k] - s.control_points[0][0][k]
        })
}

fn interpolate(
    values: &[Vec<f64>],
    first: &[Vec<f64>],
    second: &[Vec<f64>],
    t: &[f64],
) -> Result<Curve> {
    let mut controls = vec![values[0].clone()];
    let mut knots = vec![0.; 6];
    for i in 0..t.len() - 1 {
        let h = t[i + 1] - t[i];
        for j in 1..=5 {
            controls.push(
                (0..values[0].len())
                    .map(|k| match j {
                        1 => values[i][k] + h * first[i][k] / 5.,
                        2 => values[i][k] + 2. * h * first[i][k] / 5. + h * h * second[i][k] / 20.,
                        3 => {
                            values[i + 1][k] - 2. * h * first[i + 1][k] / 5.
                                + h * h * second[i + 1][k] / 20.
                        }
                        4 => values[i + 1][k] - h * first[i + 1][k] / 5.,
                        _ => values[i + 1][k],
                    })
                    .collect(),
            );
        }
        knots.extend(std::iter::repeat_n(t[i + 1], 5));
    }
    knots.push(1.);
    let curve = Curve {
        degree: 5,
        knots,
        weights: vec![1.; controls.len()],
        control_points: controls,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

pub(super) fn build(
    m: &Marcher<'_>,
    raw: &[RawPoint],
    law: &RadiusLaw,
    n: usize,
) -> Result<(Surface, [Curve; 2], Vec<SpineSample>)> {
    // At most 6*n root corrections, each with the existing 32 iteration cap.
    // Tighter solve accuracy improves derivative proposals; proof tolerance and
    // the 100k/1M downstream qualification budgets are unchanged.
    let strict = Marcher::new(m.a, m.b, m.sa, m.sb, m.tol.min(1e-13));
    let t = (0..n)
        .map(|i| i as f64 / (n - 1) as f64)
        .collect::<Vec<_>>();
    let mut spine = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);
    let mut first = Vec::with_capacity(n);
    let mut second = Vec::with_capacity(n);
    let flatten = |sample: &SpineSample| -> Result<Vec<f64>> {
        let section = g2_section(&strict, sample)?.map_err(|_| {
            crate::Error::new("FILLET_STATION_SECTION", "Quintic station section failed")
        })?;
        let mut v = section
            .control_points
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        v.extend(sample.uv_a);
        v.extend(sample.uv_b);
        Ok(v)
    };
    for &q in &t {
        let sample = station_at(&strict, raw, q, law)?;
        let base = flatten(&sample)?;
        let h = 0.001;
        let (offset, w1, w2): ([f64; 5], [f64; 5], [f64; 5]) = if q == 0. {
            (
                [0., 1., 2., 3., 4.],
                [-25., 48., -36., 16., -3.],
                [35., -104., 114., -56., 11.],
            )
        } else if q == 1. {
            (
                [0., -1., -2., -3., -4.],
                [25., -48., 36., -16., 3.],
                [35., -104., 114., -56., 11.],
            )
        } else {
            (
                [-2., -1., 0., 1., 2.],
                [1., -8., 0., 8., -1.],
                [-1., 16., -30., 16., -1.],
            )
        };
        let samples = offset
            .iter()
            .map(|&d| flatten(&station_at(&strict, raw, q + d * h, law)?))
            .collect::<Result<Vec<_>>>()?;
        first.push(
            (0..base.len())
                .map(|k| {
                    (0..5)
                        .map(|j| w1[j] * (samples[j][k] - base[k]))
                        .sum::<f64>()
                        / (12. * h)
                })
                .collect::<Vec<_>>(),
        );
        second.push(
            (0..base.len())
                .map(|k| {
                    (0..5)
                        .map(|j| w2[j] * (samples[j][k] - base[k]))
                        .sum::<f64>()
                        / (12. * h * h)
                })
                .collect::<Vec<_>>(),
        );
        values.push(base);
        spine.push(sample);
    }
    let field = |start: usize, count: usize| -> Result<Curve> {
        let slice = |v: &[Vec<f64>]| {
            v.iter()
                .map(|p| p[start..start + count].to_vec())
                .collect::<Vec<_>>()
        };
        interpolate(&slice(&values), &slice(&first), &slice(&second), &t)
    };
    let rows = (0..6)
        .map(|i| field(3 * i, 3))
        .collect::<Result<Vec<_>>>()?;
    let skin = Surface {
        degree_u: 5,
        degree_v: 5,
        knots_u: vec![0.; 6].into_iter().chain(vec![1.; 6]).collect(),
        knots_v: rows[0].knots.clone(),
        control_points: rows.iter().map(|c| c.control_points.clone()).collect(),
        weights: rows.iter().map(|c| c.weights.clone()).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    skin.validate()?;
    Ok((skin, [field(18, 2)?, field(20, 2)?], spine))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quintic_matches_independent_polynomial_on_nonuniform_stations() {
        let t = [0., 0.3, 1.];
        let v = t.map(|q: f64| vec![q.powi(5), q * q]);
        let d = t.map(|q: f64| vec![5. * q.powi(4), 2. * q]);
        let dd = t.map(|q: f64| vec![20. * q.powi(3), 2.]);
        let c = interpolate(&v, &d, &dd, &t).unwrap();
        for q in [0., 0.13, 0.3, 0.51, 0.87, 1.] {
            let p = c.evaluate(q).unwrap().point;
            assert!((p[0] - q.powi(5)).abs() < 1e-12 && (p[1] - q * q).abs() < 1e-12);
        }
    }
}
