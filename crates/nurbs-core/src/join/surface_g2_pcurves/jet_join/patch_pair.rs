//! Correlated ORIGINAL boundary jets for two independent rational U patches
//! whose literal weights are constant along each V row. No re-gauged surrogate.
use super::*;
type Polynomial = Vec<[I; 3]>;
#[path = "patch_pair/exact_coefficients.rs"]
mod exact_coefficients;
struct Side {
    u: Polynomial,
    uu: Polynomial,
    uv: Polynomial,
}
fn difference(a: f64, b: f64) -> Result<I> {
    if a == b {
        Ok(I::point(0.))
    } else {
        I::point(a).sub(I::point(b))
    }
}
fn diff(p: &Polynomial, width: I) -> Result<Polynomial> {
    p.windows(2)
        .map(|v| {
            let mut out = [I::point(0.); 3];
            for k in 0..3 {
                out[k] = v[1][k]
                    .sub(v[0][k])?
                    .mul(I::point((p.len() - 1) as f64))?
                    .div(width)?;
            }
            Ok(out)
        })
        .collect()
}
fn evaluate(p: &Polynomial, t: [f64; 2]) -> Result<[I; 3]> {
    let t = I::new(t[0], t[1])?;
    let complement = I::point(1.).sub(t)?.intersect(0., 1.)?;
    let mut rows = p.clone();
    let hull = std::array::from_fn::<_, 3, _>(|k| {
        [
            p.iter().map(|v| v[k].lo).fold(f64::INFINITY, f64::min),
            p.iter().map(|v| v[k].hi).fold(f64::NEG_INFINITY, f64::max),
        ]
    });
    for n in (1..rows.len()).rev() {
        for j in 0..n {
            for k in 0..3 {
                rows[j][k] = rows[j][k].mul(complement)?.add(rows[j + 1][k].mul(t)?)?;
            }
        }
    }
    for k in 0..3 {
        rows[0][k] = rows[0][k].intersect(hull[k][0], hull[k][1])?;
    }
    Ok(rows[0])
}
fn validate(s: &Surface) -> Result<()> {
    s.validate()?;
    let p = s.degree_u;
    let q = s.degree_v;
    check(
        !s.periodic_u
            && !s.periodic_v
            && (2..=16).contains(&p)
            && (2..=16).contains(&q)
            && s.control_points.len() == p + 1
            && s.control_points[0].len() == q + 1
            && s.control_points[0][0].len() == 3
            && s.knots_u[..=p].iter().all(|&u| u == s.knots_u[p])
            && s.knots_u[p + 1..].iter().all(|&u| u == s.knots_u[p + 1])
            && s.knots_v[..=q].iter().all(|&v| v == s.knots_v[q])
            && s.knots_v[q + 1..].iter().all(|&v| v == s.knots_v[q + 1])
            && s.weights.iter().all(|row| row.iter().all(|&w| w == row[0])),
        "Patch-pair jets require original clamped U cells with common V denominators",
    )
}
fn prepare(s: &Surface, end: bool) -> Result<Side> {
    let p = s.degree_u;
    let q = s.degree_v;
    let ids = if end { [p, p - 1, p - 2] } else { [0, 1, 2] };
    let w0 = I::point(s.weights[ids[0]][0]);
    let r1 = I::point(s.weights[ids[1]][0]).div(w0)?;
    let r2 = I::point(s.weights[ids[2]][0]).div(w0)?;
    let width = difference(s.knots_u[p + 1], s.knots_u[p])?;
    let vwidth = difference(s.knots_v[q + 1], s.knots_v[q])?;
    let mut u = Vec::new();
    let mut uu = Vec::new();
    for j in 0..=q {
        let mut first = [I::point(0.); 3];
        let mut second = first;
        for k in 0..3 {
            let p0 = s.control_points[ids[0]][j][k];
            let inward = difference(s.control_points[ids[1]][j][k], p0)?
                .mul(I::point(p as f64))?
                .mul(r1)?;
            second[k] = difference(s.control_points[ids[2]][j][k], p0)?
                .mul(I::point((p * (p - 1)) as f64))?
                .mul(r2)?
                .sub(
                    r1.mul(I::point(p as f64))?
                        .sub(I::point(1.))?
                        .mul(I::point(2.))?
                        .mul(inward)?,
                )?
                .div(width.mul(width)?)?;
            first[k] = inward
                .mul(I::point(if end { -1. } else { 1. }))?
                .div(width)?;
        }
        u.push(first);
        uu.push(second);
    }
    let uv = diff(&u, vwidth)?;
    Ok(Side { u, uu, uv })
}
/// Full original shared polynomial seam, with separate unchanged rational U
/// denominators on either side. Only G2 at this seam is covered, never a solid
/// or an absolute-curvature certificate. Unsupported sources return an error.
pub fn inspect_original_patch_pair(
    left: &Surface,
    right: &Surface,
    normal: f64,
    curvature: f64,
    max_cells: usize,
) -> Result<SpanJoinReport> {
    validate(left)?;
    validate(right)?;
    check(
        normal.is_finite()
            && (0. ..=2.).contains(&normal)
            && curvature.is_finite()
            && curvature >= 0.
            && max_cells <= 100000
            && left.degree_v == right.degree_v
            && left.knots_v == right.knots_v
            && left.control_points.last() == right.control_points.first(),
        "Patch-pair source seam or limits differ",
    )?;
    // Exact preparation has its own bounded allowance derived from the
    // caller's shape-cell budget (never more than one million rational ops,
    // 8192 bits per coefficient). Exhaustion retains the original interval
    // route; no missing exact result can grant acceptance.
    let exact =
        exact_coefficients::prepare_pair(left, right, max_cells.saturating_mul(128).min(1000000));
    let (a, b, v, vv, delta) = if let Some(p) = exact {
        (p.a, p.b, p.v, p.vv, p.delta)
    } else {
        let q = left.degree_v;
        let width = difference(left.knots_v[q + 1], left.knots_v[q])?;
        let origin = &left.control_points.last().unwrap()[0];
        let seam: Polynomial = left
            .control_points
            .last()
            .unwrap()
            .iter()
            .map(|p| {
                Ok([
                    difference(p[0], origin[0])?,
                    difference(p[1], origin[1])?,
                    difference(p[2], origin[2])?,
                ])
            })
            .collect::<Result<_>>()?;
        let v = diff(&seam, width)?;
        let vv = diff(&v, width)?;
        let a = prepare(left, true)?;
        let b = prepare(right, false)?;
        let subtract = |x: &Polynomial, y: &Polynomial| -> Result<Polynomial> {
            x.iter()
                .zip(y)
                .map(|(x, y)| Ok([y[0].sub(x[0])?, y[1].sub(x[1])?, y[2].sub(x[2])?]))
                .collect()
        };
        let delta = [
            subtract(&a.u, &b.u)?,
            subtract(&a.uu, &b.uu)?,
            subtract(&a.uv, &b.uv)?,
        ];
        (a, b, v, vv, delta)
    };
    let query = |t: [f64; 2]| -> Result<Option<[[f64; 2]; 2]>> {
        let mut ja = [[[I::point(0.); 3]; 4]; 4];
        let mut jb = ja;
        ja[0][1] = evaluate(&v, t)?;
        ja[0][2] = evaluate(&vv, t)?;
        jb[0][1] = ja[0][1];
        jb[0][2] = ja[0][2];
        for (s, j) in [(&a, &mut ja), (&b, &mut jb)] {
            j[1][0] = evaluate(&s.u, t)?;
            j[2][0] = evaluate(&s.uu, t)?;
            j[1][1] = evaluate(&s.uv, t)?;
        }
        paired::bounds(
            &ja,
            &jb,
            [
                evaluate(&delta[0], t)?,
                evaluate(&delta[1], t)?,
                evaluate(&delta[2], t)?,
            ],
        )
    };
    let mut work = 0;
    let mut lower = [0f64; 2];
    let mut pending = vec![[0., 1.]];
    let mut upper = [0f64; 2];
    let mut unresolved = 0;
    while let Some(t) = pending.pop() {
        if max_cells - work < 2 {
            unresolved += 1 + pending.len();
            break;
        }
        work += 2;
        let bound = query(t)?;
        if let Some(b) = bound.filter(|b| b[0][1] <= normal && b[1][1] <= curvature) {
            for k in 0..2 {
                upper[k] = upper[k].max(b[k][1]);
            }
            continue;
        }
        // A failed whole cover may hide an endpoint-only defect. Test the
        // ORIGINAL seam endpoints before spending the remaining allowance on
        // interior subdivisions. Point bounds may reject, never accept.
        if t == [0., 1.] {
            for endpoint in [0., 1.] {
                if max_cells - work < 2 {
                    break;
                }
                work += 2;
                if let Some(b) = query([endpoint, endpoint])? {
                    for k in 0..2 {
                        lower[k] = lower[k].max(b[k][0]);
                    }
                }
                if lower[0] > normal || lower[1] > curvature {
                    break;
                }
            }
        }
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if lower[0] <= normal && lower[1] <= curvature && max_cells - work >= 2 {
            work += 2;
            if let Some(b) = query([mid, mid])? {
                for k in 0..2 {
                    lower[k] = lower[k].max(b[k][0]);
                }
            }
        }
        if lower[0] > normal || lower[1] > curvature {
            return Ok(SpanJoinReport {
                decision: Decision::CorrespondenceExceedsTolerance,
                normal: [lower[0], f64::INFINITY],
                tensor: [lower[1], f64::INFINITY],
                regularity_proven: false,
                cells: work,
                pending: pending.len() + 1,
                cross_scales: [1.; 2],
            });
        }
        if !(t[0] < mid && mid < t[1]) {
            unresolved += 1;
            continue;
        }
        pending.push([mid, t[1]]);
        pending.push([t[0], mid]);
    }
    Ok(SpanJoinReport {
        decision: if unresolved == 0 {
            Decision::WithinTolerance
        } else {
            Decision::Unresolved
        },
        normal: [
            lower[0],
            if unresolved == 0 {
                upper[0]
            } else {
                f64::INFINITY
            },
        ],
        tensor: [
            lower[1],
            if unresolved == 0 {
                upper[1]
            } else {
                f64::INFINITY
            },
        ],
        regularity_proven: unresolved == 0,
        cells: work,
        pending: unresolved,
        cross_scales: [1.; 2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> [Surface; 2] {
        [0., 1.].map(|start| Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![start, start, start, start + 1., start + 1., start + 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![start + i as f64 / 2., j as f64 / 2., 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![if start == 0. { 0.25 } else { 8. }; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        })
    }
    #[test]
    fn independent_original_gauges_and_full_seam_are_checked() {
        let [a, b] = pair();
        let before = [a.clone(), b.clone()];
        let r = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 200).unwrap();
        assert_eq!(r.decision, Decision::WithinTolerance);
        assert!(r.regularity_proven);
        assert!(r.cells <= 200);
        assert_eq!([a, b], before);
    }
    #[test]
    fn curved_original_join_and_curvature_only_defect_are_distinguished() {
        let [mut a, mut b] = pair();
        for (i, z) in [0., 0., 1.].into_iter().enumerate() {
            for point in &mut a.control_points[i] {
                point[2] = z;
            }
        }
        for (i, z) in [1., 2., 4.].into_iter().enumerate() {
            for point in &mut b.control_points[i] {
                point[2] = z;
            }
        }
        let r = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 1000).unwrap();
        assert_eq!(r.decision, Decision::WithinTolerance);
        assert!(r.regularity_proven && r.pending == 0);
        // The seam position and first derivative are unchanged. Only the
        // second U derivative changes on the actual original right surface.
        for point in &mut b.control_points[2] {
            point[2] += 0.1;
        }
        let r = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 1000).unwrap();
        assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
        assert!(r.tensor[0] > 1e-4);
    }
    #[test]
    fn a_localized_original_endpoint_defect_is_not_hidden_by_a_good_midpoint() {
        let [mut a, mut b] = pair();
        for s in [&mut a, &mut b] {
            s.degree_v = 16;
            s.knots_v = [vec![0.; 17], vec![1.; 17]].concat();
            for i in 0..3 {
                let x = s.control_points[i][0][0];
                let w = s.weights[i][0];
                s.control_points[i] = (0..17).map(|j| vec![x, j as f64 / 16., 0.]).collect();
                s.weights[i] = vec![w; 17];
            }
        }
        // At V=1/2 the tensor jump is 0.002/2^16, below 1e-4.
        // At the ACTUAL original V=0 endpoint the jump is 0.002.
        b.control_points[2][0][2] = 0.001;
        let r = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 6).unwrap();
        assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
        assert!(r.tensor[0] > 1e-4 && r.cells <= 6);
        let tiny = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 2).unwrap();
        assert_eq!(tiny.decision, Decision::Unresolved);
        assert_eq!(tiny.cells, 2);
    }
    #[test]
    fn real_normal_jump_and_missing_budget_cannot_be_admitted() {
        let [a, mut b] = pair();
        let r = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 0).unwrap();
        assert_eq!(r.decision, Decision::Unresolved);
        assert_eq!(r.cells, 0);
        for p in &mut b.control_points[1] {
            p[2] = 0.1;
        }
        let r = inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 200).unwrap();
        assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
        b.weights[1][1] += 1e-12;
        assert!(inspect_original_patch_pair(&a, &b, 1e-5, 1e-4, 200).is_err());
    }
}
