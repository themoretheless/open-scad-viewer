//! Whole-piece polynomial displacement over a rational boundary proposal.
//! All resulting stored geometry requires fresh G0/G2 and body admission.
use super::*;
type Vector = [f64; 3];

fn split(field: &[Vector], t: f64) -> (Vec<Vector>, Vec<Vector>) {
    let mut row = field.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    for n in (1..row.len()).rev() {
        for i in 0..n {
            row[i] = std::array::from_fn(|k| (1. - t) * row[i][k] + t * row[i + 1][k]);
        }
        left.push(row[0]);
        right.push(row[n - 1]);
    }
    right.reverse();
    (left, right)
}
fn restrict(field: &[Vector], a: f64, b: f64) -> Vec<Vector> {
    let mut out = if a == 0. {
        field.to_vec()
    } else {
        split(field, a).1
    };
    if b < 1. {
        out = split(&out, (b - a) / (1. - a)).0;
    }
    // Shared original parameter values have one literal authored field value.
    out[0] = split(field, a).0[field.len() - 1];
    let last = out.len() - 1;
    out[last] = split(field, b).0[field.len() - 1];
    out
}
fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1u64, |v, i| v * (n - i) as u64 / (i + 1) as u64) as f64
}

pub(super) fn prepare(
    original: &Curve,
    field: [Vector; 6],
    work: &mut Work,
) -> Result<(Curve, Vec<Vector>)> {
    original.validate()?;
    let p = original.degree;
    let q = p + 5;
    require(q <= 16, "Bound weighted central field degree")?;
    let mut source = original.clone();
    let d = source.domain();
    let mut internal: Vec<_> = source
        .knots
        .iter()
        .copied()
        .filter(|&t| d[0] < t && t < d[1])
        .collect();
    internal.dedup();
    for &t in &internal {
        let count = source.knots.iter().filter(|&&v| v == t).count();
        require(
            count <= p,
            "Weighted field needs continuous original rim pieces",
        )?;
        if count < p {
            source = source.insert(t, p - count)?;
        }
        work.charge(source.control_points.len())?;
    }
    let mut points = Vec::new();
    let mut weights = Vec::new();
    let mut offsets = Vec::new();
    for span in p..source.control_points.len() {
        let a = source.knots[span];
        let b = source.knots[span + 1];
        if a == b {
            continue;
        }
        let cell = Curve {
            degree: p,
            knots: [vec![a; p + 1], vec![b; p + 1]].concat(),
            control_points: source.control_points[span - p..=span].to_vec(),
            weights: source.weights[span - p..=span].to_vec(),
            periodic: false,
        };
        let elevated = cell.elevate(q)?;
        let restricted = restrict(
            &field,
            (a - d[0]) / (d[1] - d[0]),
            (b - d[0]) / (d[1] - d[0]),
        );
        if !points.is_empty() {
            require(
                points.last() == elevated.control_points.first()
                    && weights.last() == elevated.weights.first()
                    && offsets.last() == Some(&restricted[0]),
                "Weighted field lost its shared original endpoint",
            )?;
        }
        for k in 0..=q {
            if !points.is_empty() && k == 0 {
                continue;
            }
            let mut value = [0.; 3];
            for i in 0..=p {
                if k < i || k - i > 5 {
                    continue;
                }
                let j = k - i;
                let factor = choose(p, i) * choose(5, j) / choose(q, k) * cell.weights[i];
                for axis in 0..3 {
                    value[axis] += factor * restricted[j][axis];
                }
                work.charge(4)?;
            }
            for v in &mut value {
                *v /= elevated.weights[k];
            }
            if k == 0 {
                value = restricted[0];
            }
            if k == q {
                value = restricted[5];
            }
            points.push(elevated.control_points[k].clone());
            weights.push(elevated.weights[k]);
            offsets.push(value);
        }
    }
    require(points.len() <= 256, "Weighted central field control limit")?;
    // Preserve exact zero endpoint field jets when the whole-piece polynomial
    // has them; rounded Bernstein product summation is not an identity proof.
    if field[0] == field[1] && field[1] == field[2] {
        offsets[..3].fill(field[0]);
    }
    let n = offsets.len();
    if field[3] == field[4] && field[4] == field[5] {
        offsets[n - 3..].fill(field[5]);
    }
    let mut knots = vec![d[0]; q + 1];
    for t in internal {
        knots.extend(vec![t; q]);
    }
    knots.extend(vec![d[1]; q + 1]);
    let curve = Curve {
        degree: q,
        knots,
        weights,
        control_points: points,
        periodic: false,
    };
    curve.validate()?;
    Ok((curve, offsets))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_denominator_product_preserves_the_whole_smooth_field() {
        let source = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        }
        .insert(0.5, 2)
        .unwrap();
        let before = source.clone();
        let field = [0., 1., 2., 1., 1., 1.].map(|z| [0., 0., z]);
        let mut work = Work {
            used: 0,
            max: 1000000,
        };
        let (base, offsets) = prepare(&source, field, &mut work).unwrap();
        let mut displaced = base.clone();
        for (p, f) in displaced.control_points.iter_mut().zip(offsets) {
            for k in 0..3 {
                p[k] += f[k];
            }
        }
        // Independent evaluation regression; actual continuity is separately
        // proved below over the original complete V interval.
        for t in [0., 0.2, 0.5, 0.8, 1.] {
            let p = source.evaluate(t).unwrap();
            let actual = displaced.evaluate(t).unwrap();
            let f = split(&field, t).0[5];
            for k in 0..3 {
                assert!((actual.point[k] - p.point[k] - f[k]).abs() < 1e-12);
            }
        }
        let s = Surface {
            degree_u: displaced.degree,
            degree_v: 1,
            knots_u: displaced.knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: displaced
                .control_points
                .iter()
                .map(|p| vec![p.clone(), vec![p[0], 1., p[2]]])
                .collect(),
            weights: displaced.weights.iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let r =
            nurbs_core::surface_g2_pcurves::inspect_original_span_join(&s, 0.5, 1e-5, 1e-4, 6000)
                .unwrap();
        assert_eq!(
            r.decision,
            nurbs_core::surface_join::Decision::WithinTolerance,
            "normal={:?} tensor={:?}",
            r.normal,
            r.tensor
        );
        assert!(r.regularity_proven && r.pending == 0);
        assert_eq!(source, before);
        assert!(prepare(&source, field, &mut Work { used: 0, max: 0 }).is_err());
    }
}
