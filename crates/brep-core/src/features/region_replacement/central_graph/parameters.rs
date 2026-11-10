//! Proposal-only sampling at projected knot crossings. Original interval
//! inspection, never this numerical root scan, decides geometric admission.
use super::*;
pub(super) fn stations(
    c: &Curve,
    chart: &Chart,
    knots: [&[f64]; 2],
    degree: usize,
    base: usize,
) -> Result<Vec<f64>> {
    let d = c.domain();
    let evaluate = |t: f64| -> Result<[f64; 2]> {
        Ok(chart.uv(point(
            &c.evaluate(if t == 0. {
                d[0]
            } else if t == 1. {
                d[1]
            } else {
                d[0] + (d[1] - d[0]) * t
            })?
            .point,
        )))
    };
    let scan = (0..=256)
        .map(|i| evaluate(i as f64 / 256.))
        .collect::<Result<Vec<_>>>()?;
    let mut roots = vec![0., 1.];
    for axis in 0..2 {
        let mut unique = knots[axis].to_vec();
        unique.dedup();
        for knot in unique {
            for i in 0..256 {
                let a = scan[i][axis] - knot;
                let b = scan[i + 1][axis] - knot;
                if a.abs() < 1e-12 && b.abs() < 1e-12 {
                    continue;
                }
                if a.abs() < 1e-12 {
                    roots.push(i as f64 / 256.);
                }
                if b.abs() < 1e-12 {
                    roots.push((i + 1) as f64 / 256.);
                }
                if a * b >= 0. {
                    continue;
                }
                let (mut lo, mut hi) = (i as f64 / 256., (i + 1) as f64 / 256.);
                for _ in 0..40 {
                    let mid = (lo + hi) * 0.5;
                    let v = evaluate(mid)?[axis] - knot;
                    if v == 0. {
                        lo = mid;
                        hi = mid;
                        break;
                    }
                    if v.is_sign_positive() == a.is_sign_positive() {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                roots.push((lo + hi) * 0.5);
            }
        }
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let mut sites = (0..base)
        .map(|i| i as f64 / (base - 1) as f64)
        .collect::<Vec<_>>();
    sites.extend(roots.iter().copied());
    for pair in roots.windows(2) {
        for j in 1..=2 * degree - 1 {
            sites.push(pair[0] + (pair[1] - pair[0]) * j as f64 / (2 * degree) as f64);
        }
    }
    sites.sort_by(f64::total_cmp);
    sites.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
    if sites.len() > 4096 {
        return Err(refuse(
            "Knot-aware fitting exceeds 4096 stations per boundary",
        ));
    }
    Ok(sites)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stations_cover_a_thin_projected_cell_missed_by_uniform_sampling() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let chart = Chart {
            frame: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            origin: [0.; 3],
            low: [0.; 2],
            span: [1.; 2],
        };
        let sites = stations(&c, &chart, [&[0., 0.499, 0.5, 1.], &[0., 1.]], 5, 129).unwrap();
        let a = 1. - (1_f64 - 0.499).sqrt();
        let b = 1. - (1_f64 - 0.5).sqrt();
        assert!(!(0..129).any(|i| a < i as f64 / 128. && (i as f64 / 128.) < b));
        assert!(sites.iter().filter(|&&t| a < t && t < b).count() >= 5);
    }
}
