//! Exact ordered traversal of an axis-aligned chart segment.
use num_rational::BigRational;
use num_traits::ToPrimitive;
use nurbs_core::curve::Curve;

fn charge(cells: &mut usize, limit: usize, cost: usize) -> Option<()> {
    if cost > limit - *cells {
        return None;
    }
    *cells += cost;
    Some(())
}

fn exact_endpoint(value: &BigRational) -> Option<f64> {
    let stored = value.to_f64()?;
    (BigRational::from_float(stored).as_ref() == Some(value)).then_some(stored)
}

pub(crate) fn natural_segment(
    c: &crate::Coedge,
    cells: &mut usize,
    limit: usize,
) -> Option<[[f64; 2]; 2]> {
    let p = &c.pcurve;
    let degree = p.degree;
    let count = p.control_points.len();
    if p.periodic || !(1..=5).contains(&degree) {
        return None;
    }
    let domain = p.domain();
    if p.knots[..=degree].iter().any(|&t| t != domain[0])
        || p.knots[count..].iter().any(|&t| t != domain[1])
    {
        return None;
    }
    let point = |i: usize| [p.control_points[i][0], p.control_points[i][1]];
    let [a, b] = [point(0), point(count - 1)];
    if count > 2 {
        charge(cells, limit, count - 1)?;
        if p.knots[degree + 1..count]
            .chunk_by(|a, b| a == b)
            .any(|run| run.len() > degree || run[0] <= domain[0] || run[0] >= domain[1])
        {
            return None;
        }
        let axis = usize::from(a[0] == b[0]);
        if a[axis] == b[axis] || a[1 - axis] != b[1 - axis] {
            return None;
        }
        // Knot insertion to Bézier spans preserves pole order: each
        // new Cartesian pole lies between its two positive-weight
        // neighbors. The rational derivative on each resulting span
        // has the sign of its ordered pole differences.
        if !p.control_points.windows(2).all(|pair| {
            pair[1][1 - axis] == a[1 - axis]
                && (if a[axis] < b[axis] {
                    pair[0][axis] < pair[1][axis]
                } else {
                    pair[0][axis] > pair[1][axis]
                })
        }) {
            return None;
        }
    }
    let Some(trim) = c.trim() else {
        return Some([a, b]);
    };
    charge(cells, limit, degree * degree)?;
    let [a, b] = trim.pcurve_parameter;
    Some([exact_side_point(p, a)?, exact_side_point(p, b)?])
}

fn exact_side_point(p: &Curve, t: f64) -> Option<[f64; 2]> {
    let degree = p.degree;
    let count = p.control_points.len();
    let [lo, hi] = p.domain();
    if !(lo..=hi).contains(&t) {
        return None;
    }
    if t == lo || t == hi {
        let end = if t == lo { 0 } else { count - 1 };
        return Some([p.control_points[end][0], p.control_points[end][1]]);
    }
    let span = p.knots.partition_point(|&k| k <= t) - 1;
    let first = span - degree;
    let q = BigRational::from_float;
    let mut net = (first..=span)
        .map(|i| {
            let w = q(p.weights[i])?;
            Some([
                q(p.control_points[i][0])? * &w,
                q(p.control_points[i][1])? * &w,
                w,
            ])
        })
        .collect::<Option<Vec<_>>>()?;
    let t = q(t)?;
    for level in 1..=degree {
        for j in (level..=degree).rev() {
            let i = first + j;
            let lower = q(p.knots[i])?;
            let fraction = (&t - &lower) / (q(p.knots[i + degree + 1 - level])? - lower);
            net[j] = std::array::from_fn(|axis| {
                &net[j - 1][axis] + &fraction * (&net[j][axis] - &net[j - 1][axis])
            });
        }
    }
    let [x, y, w] = &net[degree];
    Some([exact_endpoint(&(x / w))?, exact_endpoint(&(y / w))?])
}
