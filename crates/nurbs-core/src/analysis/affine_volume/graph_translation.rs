//! Exact triangular global map on the ORIGINAL nonplanar carrier.
//! Height is never flattened, sampled or bounded to establish injectivity.
use super::*;
use crate::numerics::compensated::two_sum;
fn subtract(a: f64, b: f64) -> Option<f64> {
    let (v, e) = two_sum(a, -b);
    (v.is_finite() && e == 0.).then_some(v)
}
fn multiply(a: f64, b: f64) -> Option<f64> {
    if a == 0. || b == 0. {
        return Some(0.);
    }
    if a == 1. {
        return Some(b);
    }
    if b == 1. {
        return Some(a);
    }
    let v = a * b;
    (v.is_finite()
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && v.abs() >= 1e-100
        && a.mul_add(b, -v) == 0.)
        .then_some(v)
}
/// true means success or budget exhaustion; false permits the existing atlas
/// with the SAME remaining ledger. No fresh budget is created on refusal.
pub(super) fn certify(
    carrier: &Surface,
    pieces: Motion<'_>,
    closed: bool,
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> Result<bool> {
    if closed || carrier.periodic_u || carrier.periodic_v {
        return Ok(false);
    }
    let pu = carrier.degree_u;
    let pv = carrier.degree_v;
    if pu == 0
        || pv == 0
        || carrier.control_points.len() != pu + 1
        || carrier.control_points[0].len() != pv + 1
    {
        return Ok(false);
    }
    let clamped = |knots: &[f64], p: usize| {
        knots[..=p].iter().all(|x| *x == knots[p])
            && knots[p + 1..].iter().all(|x| *x == knots[p + 1])
    };
    if !clamped(&carrier.knots_u, pu) || !clamped(&carrier.knots_v, pv) {
        return Ok(false);
    }
    macro_rules! charge {
        ($n:expr) => {
            if max_work - out.exact_work < $n {
                out.reason = Some("affine-volume-graph-work-limit");
                return Ok(true);
            }
            out.exact_work += $n;
        };
    }
    // One constant ORIGINAL invertible linear law, not a sampled frame.
    let first = pieces.controls(0)[0];
    for pose in (0..pieces.len()).flat_map(|i| pieces.controls(i)) {
        for j in 0..3 {
            for k in 0..3 {
                charge!(1);
                if pose[j][k] != first[j][k] {
                    return Ok(false);
                }
            }
        }
    }
    charge!(9);
    let determinant = iv::dot_tight(
        first[0].map(I::point),
        iv::cross(first[1].map(I::point), first[2].map(I::point))?,
    )?;
    if determinant.lo <= 0. && determinant.hi >= 0. {
        return Ok(false);
    }
    for &w in carrier.weights.iter().flatten() {
        charge!(1);
        if w != carrier.weights[0][0] {
            return Ok(false);
        }
    }
    // Exact translation-control differences parallel one source matrix column.
    // Thus after the same fixed inverse linear map translation varies only in
    // the original graph-height coordinate. The inverse is never approximated.
    let mut chart = None;
    for height in 0..3 {
        let Some(pivot) = (0..3).find(|&k| first[height][k] != 0.) else {
            continue;
        };
        let mut valid = true;
        let mut varies = false;
        'controls: for pose in (0..pieces.len()).flat_map(|i| pieces.controls(i)) {
            charge!(1);
            let Some(d) = subtract(pose[3][pivot], first[3][pivot]) else {
                valid = false;
                break;
            };
            varies |= d != 0.;
            for k in 0..3 {
                charge!(3);
                let lhs = subtract(pose[3][k], first[3][k])
                    .and_then(|v| multiply(v, first[height][pivot]));
                let rhs = multiply(d, first[height][k]);
                if lhs.is_none() || lhs != rhs {
                    valid = false;
                    break 'controls;
                }
            }
        }
        if valid && varies {
            chart = Some((height, pivot));
            break;
        }
    }
    let Some((height, pivot)) = chart else {
        return Ok(false);
    };
    let axes = (0..3).filter(|&k| k != height).collect::<Vec<_>>();
    let p0 = &carrier.control_points[0][0];
    let mut columns = [[0.; 2]; 2];
    for (j, &k) in axes.iter().enumerate() {
        charge!(2);
        let (Some(du), Some(dv)) = (
            subtract(carrier.control_points[pu][0][k], p0[k]),
            subtract(carrier.control_points[0][pv][k], p0[k]),
        ) else {
            return Ok(false);
        };
        columns[0][j] = du;
        columns[1][j] = dv;
        for i in 0..=pu {
            for l in 0..=pv {
                charge!(7);
                let lhs = subtract(carrier.control_points[i][l][k], p0[k])
                    .and_then(|v| multiply(v, (pu * pv) as f64));
                let rhs = multiply(du, (i * pv) as f64)
                    .zip(multiply(dv, (l * pu) as f64))
                    .and_then(|(a, b)| subtract(a, -b));
                if lhs.is_none() || lhs != rhs {
                    return Ok(false);
                }
            }
        }
    }
    charge!(3);
    let det = I::point(columns[0][0])
        .mul(I::point(columns[1][1]))?
        .sub(I::point(columns[0][1]).mul(I::point(columns[1][0]))?)?;
    if det.lo <= 0. && det.hi >= 0. {
        return Ok(false);
    }
    let mut sign = 0;
    for i in 0..pieces.len() {
        if out.cells == max_cells {
            out.reason = Some("affine-volume-graph-cell-limit");
            return Ok(true);
        }
        out.cells += 1;
        out.boxes += 1;
        let jet = pieces.enclose(i, I::new(0., 1.)?)?;
        let d = jet.origin[1][pivot];
        let s = if d.lo > 0. {
            1
        } else if d.hi < 0. {
            -1
        } else {
            return Ok(false);
        };
        if sign != 0 && sign != s {
            return Ok(false);
        }
        sign = s;
    }
    // Projection is an affine homeomorphism (det!=0), so equal world points
    // force equal UV. At that UV, strict same-sign time translation forces
    // equal time. The Jacobian determinant is det(projection)*translation'.
    // Arbitrary ORIGINAL height and its derivatives cannot alter either fact.
    out.embedding_certified = true;
    out.regularity_certified = true;
    out.parameter_topology_preserved = true;
    out.reason = Some("original-nonplanar-graph-translation-certified");
    Ok(true)
}
