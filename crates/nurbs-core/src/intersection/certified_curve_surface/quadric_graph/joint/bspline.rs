//! One global boundary and positive original-cell Jacobians, including C0 joins.
use super::super::homogeneous_affine::bspline::{restrict, Work as RestrictionWork};
use super::*;
fn continuous(knots: &[f64], p: usize, n: usize) -> bool {
    let mut i = p + 1;
    while i < n {
        let mut j = i + 1;
        while j < n && knots[j] == knots[i] {
            j += 1;
        }
        if j - i > p {
            return false;
        }
        i = j;
    }
    true
}
pub(super) fn prove(
    s: &Surface,
    axes: [usize; 2],
    frame: &AffineFrame,
    limit: u64,
) -> Option<Joint> {
    let (p, r) = (s.degree_u, s.degree_v);
    let (n, m) = (s.control_points.len(), s.control_points[0].len());
    if !continuous(&s.knots_u, p, n) || !continuous(&s.knots_v, r, m) {
        return None;
    }
    let h: Vec<Vec<[Q; 3]>> = (0..n)
        .map(|i| {
            (0..m)
                .map(|j| super::super::homogeneous_separable::pole(s, i, j, axes, frame))
                .collect()
        })
        .collect();
    if h.iter().flatten().any(|a| a[2] <= q(0.)) {
        return None;
    }
    // These are the ORIGINAL outer edges. Local cell boundaries do not
    // replace or weaken this single global rectangle boundary certificate.
    if !(0..m).all(|j| h[0][j][0] == q(0.) && h[n - 1][j][0] == h[n - 1][j][2])
        || !(0..n).all(|i| h[i][0][1] == q(0.) && h[i][m - 1][1] == h[i][m - 1][2])
    {
        return None;
    }
    for (degree, knots, poles) in [
        (
            p,
            &s.knots_u,
            (0..n)
                .map(|i| [h[i][0][0].clone(), h[i][0][2].clone()])
                .collect::<Vec<_>>(),
        ),
        (
            p,
            &s.knots_u,
            (0..n)
                .map(|i| [h[i][m - 1][0].clone(), h[i][m - 1][2].clone()])
                .collect(),
        ),
        (
            r,
            &s.knots_v,
            (0..m)
                .map(|j| [h[0][j][1].clone(), h[0][j][2].clone()])
                .collect(),
        ),
        (
            r,
            &s.knots_v,
            (0..m)
                .map(|j| [h[n - 1][j][1].clone(), h[n - 1][j][2].clone()])
                .collect(),
        ),
    ] {
        super::super::clock::Clock::bspline(
            degree,
            knots,
            poles.iter().map(|a| a[1].clone()).collect(),
            &poles.iter().map(|a| &a[0] / &a[1]).collect::<Vec<_>>(),
        )
        .ok()?;
    }
    let ku: Vec<_> = s.knots_u.iter().map(|x| q(*x)).collect();
    let kv: Vec<_> = s.knots_v.iter().map(|x| q(*x)).collect();
    let mut restriction = RestrictionWork::new(limit);
    let mut jacobian = Work::new(limit);
    let mut patches = Vec::new();
    for iu in p..n {
        if ku[iu] == ku[iu + 1] {
            continue;
        }
        for iv in r..m {
            if kv[iv] == kv[iv + 1] {
                continue;
            }
            jacobian.charge(((p + 1) * (r + 1) * 8) as u64)?;
            let mut cell = vec![vec![[q(0.), q(0.), q(0.)]; r + 1]; p + 1];
            for j in 0..=r {
                let column = restrict(
                    (iu - p..=iu).map(|i| h[i][iv - r + j].clone()).collect(),
                    &ku,
                    p,
                    iu,
                    &mut restriction,
                )?;
                for i in 0..=p {
                    cell[i][j] = column[i].clone();
                }
            }
            for row in &mut cell {
                *row = restrict(row.clone(), &kv, r, iv, &mut restriction)?;
            }
            if cell.iter().flatten().any(|a| a[2] <= q(0.))
                || !positive_jacobian(&cell, &mut jacobian)?
            {
                return None;
            }
            let uv = [
                [
                    (ku[iu].clone() - &ku[p]) / (&ku[n] - &ku[p]),
                    (ku[iu + 1].clone() - &ku[p]) / (&ku[n] - &ku[p]),
                ],
                [
                    (kv[iv].clone() - &kv[r]) / (&kv[m] - &kv[r]),
                    (kv[iv + 1].clone() - &kv[r]) / (&kv[m] - &kv[r]),
                ],
            ];
            patches.push(Patch { h: cell, uv });
        }
    }
    if patches.is_empty() {
        return None;
    }
    // At a C0 seam both adjacent cells have the same nonzero tangent.
    // Positive Jacobians place their interiors on opposite sides of it.
    // At a grid vertex four positive corner angles are in (0,pi); their
    // closed cycle therefore winds exactly once. Thus seams/vertices are
    // locally open too. The global oriented boundary has degree one, and
    // every regular cell-interior preimage contributes +1. This proves a
    // single inverse globally, also on knot seams and grid vertices.
    Some(Joint { patches })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c0_source_derivative_jump_and_shared_work_are_exact() {
        let f:Vec<value_codec::Value>=value_codec::from_str(include_str!("../../../../../../../tools/browser-qualification/joint-multispan-quadric-graph-fixtures.json")).unwrap();
        let s: Surface = value_codec::from_value(f[4]["query"]["patch"].clone()).unwrap();
        let l = f[4]["scale"].as_f64().unwrap();
        let (o, u, v) = ([q(0.), q(0.)], [q(l), q(0.)], [q(0.), q(l)]);
        let det = q(l) * q(l);
        let frame = AffineFrame { origin: o, u, v, det };
        assert!(prove(&s, [0, 1], &frame, 0).is_none());
        let chart = prove(&s, [0, 1], &frame, LIMIT).unwrap();
        assert_eq!(chart.patches.len(), 2);
        let at = |row: &Vec<[Q; 3]>| {
            let mut d = row.clone();
            for level in 1..d.len() {
                for j in 0..d.len() - level {
                    d[j] = std::array::from_fn(|k| (&d[j][k] + &d[j + 1][k]) / q(2.));
                }
            }
            d[0][0].clone()
        };
        let left = &chart.patches[0].h;
        let right = &chart.patches[1].h;
        assert_eq!(q(8.) * (at(&left[4]) - at(&left[3])), q(7.) / q(8.));
        assert_eq!(q(8.) * (at(&right[1]) - at(&right[0])), q(9.) / q(8.));
    }
}
