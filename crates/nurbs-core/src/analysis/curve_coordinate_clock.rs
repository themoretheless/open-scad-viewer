//! Complete strict coordinate monotonicity on the original rational curve.
//! No refit, degree elevation, knot insertion or sampled derivative admission.
use crate::{Result, check, curve::Curve, curve_jets};
pub struct Report {
    pub proven: bool,
    pub increasing: bool,
    pub cells: usize,
    pub covered_cells: usize,
    pub spans: usize,
    pub unresolved: Option<[f64; 2]>,
}
/// Every nonempty original knot span must have a strictly signed derivative.
/// Interior multiplicity >degree (possible jumps) and periodic clocks refuse.
/// One-sided derivative signs suffice at C0 joins. The proof is conservative
/// for stationary but one-to-one clocks; those may remain unresolved.
pub fn inspect(curve: &Curve, axis: usize, max_cells: usize) -> Result<Report> {
    curve.validate()?;
    check(
        !curve.periodic
            && (1..=32).contains(&curve.degree)
            && curve.control_points.len() <= 4096
            && axis < curve.control_points[0].len()
            && max_cells <= 100000
            && curve.weights.iter().all(|w| *w > 0.),
        "Coordinate clock needs a bounded nonperiodic positive rational curve",
    )?;
    let n = curve.control_points.len();
    let p = curve.degree;
    check(
        curve.knots[..=p].iter().all(|k| *k == curve.knots[p])
            && curve.knots[n..].iter().all(|k| *k == curve.knots[n]),
        "Coordinate clock needs clamped endpoints",
    )?;
    let increasing = curve.control_points[n - 1][axis] > curve.control_points[0][axis];
    let mut out = Report {
        proven: false,
        increasing,
        cells: 0,
        covered_cells: 0,
        spans: 0,
        unresolved: None,
    };
    if curve.control_points[n - 1][axis] == curve.control_points[0][axis] {
        return Ok(out);
    }
    let mut i = p + 1;
    while i < n {
        let mut j = i + 1;
        while j < n && curve.knots[j] == curve.knots[i] {
            j += 1;
        }
        if j - i > p {
            out.unresolved = Some([curve.knots[i]; 2]);
            return Ok(out);
        }
        i = j;
    }
    for span in p..n {
        let domain = [curve.knots[span], curve.knots[span + 1]];
        if domain[0] == domain[1] {
            continue;
        }
        out.spans += 1;
        let mut stack = vec![domain];
        while let Some(range) = stack.pop() {
            if out.cells == max_cells {
                out.unresolved = Some(range);
                return Ok(out);
            }
            out.cells += 1;
            let jets = curve_jets::enclose(curve, span, range)?;
            let derivative = jets[1][axis];
            if (increasing && derivative.lo > 0.) || (!increasing && derivative.hi < 0.) {
                out.covered_cells += 1;
                continue;
            }
            if (increasing && derivative.hi < 0.) || (!increasing && derivative.lo > 0.) {
                out.unresolved = Some(range);
                return Ok(out);
            }
            let middle = range[0] * 0.5 + range[1] * 0.5;
            if middle == range[0] || middle == range[1] {
                out.unresolved = Some(range);
                return Ok(out);
            }
            stack.push([middle, range[1]]);
            stack.push([range[0], middle]);
        }
    }
    out.proven = out.spans > 0;
    Ok(out)
}
