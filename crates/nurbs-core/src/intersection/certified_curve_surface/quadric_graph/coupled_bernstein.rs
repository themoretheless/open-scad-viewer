//! Coupled positive tensor Bernstein mean maps and exact inverse enclosures.
use super::*;
/// In logit coordinates the projection is (E[i]/p,E[j]/q) for the
/// full-support distribution proportional to binom(p,i)binom(q,j)w_ij exp(i*x+j*y).
/// Its log-partition Hessian is positive definite (the rectangular support
/// spans R²). Subtracting any interior prescribed mean gives a coercive,
/// strictly convex function, hence exactly one inverse. On the four edges
/// the same variance proof reduces to the positive univariate clock.
/// Thus the closed original patch covers the full projected rectangle once.
///
/// a/(1-a) = u/(1-u) * N/D, where N,D have positive Bernstein
/// coefficients w_(i+1,j), w_(i,j) in degree (p-1,q). After exact restriction
/// to any parameter rectangle, N/D is inside the hull of coefficient ratios.
/// Solving that positive odds inequality gives an enclosing u interval.
/// The analogous v inequality needs no separability or cross-sign assumption.
pub(super) fn enclosure(w: &[Vec<Q>], r: &[[Q; 2]; 2]) -> Result<[[Q; 2]; 2]> {
    let mut uv = [[q(0.), q(1.)], [q(0.), q(1.)]];
    for _ in 0..8 {
        let before = uv.clone();
        for axis in 0..2 {
            let (n, m) = (
                w.len() - usize::from(axis == 0),
                w[0].len() - usize::from(axis == 1),
            );
            let mut h: Vec<Vec<[Q; 2]>> = (0..n)
                .map(|i| {
                    (0..m)
                        .map(|j| {
                            [
                                w[i + usize::from(axis == 0)][j + usize::from(axis == 1)].clone(),
                                w[i][j].clone(),
                            ]
                        })
                        .collect()
                })
                .collect();
            for j in 0..m {
                let column = restrict((0..n).map(|i| h[i][j].clone()).collect(), &uv[0]);
                for i in 0..n {
                    h[i][j] = column[i].clone();
                }
            }
            for row in &mut h {
                *row = restrict(row.clone(), &uv[1]);
            }
            let ratios: Vec<_> = h.iter().flatten().map(|c| &c[0] / &c[1]).collect();
            let lo = ratios.iter().min().unwrap();
            let hi = ratios.iter().max().unwrap();
            let solve = |a: &Q, k: &Q| a / (k * (q(1.) - a) + a);
            let low = solve(&r[axis][0], hi).max(uv[axis][0].clone());
            let high = solve(&r[axis][1], lo).min(uv[axis][1].clone());
            // Binary64 proposals only cap arithmetic growth. Outward exact
            // enclosures retain every solution at every refinement step.
            let bound = super::super::patch_exact_membership::bound;
            uv[axis] = [
                q(bound(&low)?[0]).max(uv[axis][0].clone()),
                q(bound(&high)?[1]).min(uv[axis][1].clone()),
            ];
        }
        if uv == before {
            break;
        }
    }
    Ok(uv)
}
fn split(mut d: Vec<[Q; 2]>, t: &Q) -> (Vec<[Q; 2]>, Vec<[Q; 2]>) {
    let p = d.len() - 1;
    let mut left = vec![d[0].clone()];
    let mut right = vec![d[p].clone()];
    for r in 1..=p {
        for i in 0..=p - r {
            d[i] = std::array::from_fn(|k| (q(1.) - t) * &d[i][k] + t * &d[i + 1][k]);
        }
        left.push(d[0].clone());
        right.push(d[p - r].clone());
    }
    right.reverse();
    (left, right)
}
fn restrict(d: Vec<[Q; 2]>, r: &[Q; 2]) -> Vec<[Q; 2]> {
    if r[0] == q(1.) {
        return vec![d.last().unwrap().clone(); d.len()];
    }
    let (_, right) = split(d, &r[0]);
    split(right, &((&r[1] - &r[0]) / (q(1.) - &r[0]))).0
}

#[cfg(test)]
mod tests {
    use super::*;
    fn basis(p: usize, i: usize, t: &Q) -> Q {
        let c = (0..i).fold(1u64, |c, j| c * (p - j) as u64 / (j + 1) as u64);
        (0..i).fold(q(c as f64), |a, _| a * t) * (0..p - i).fold(q(1.), |a, _| a * (q(1.) - t))
    }
    fn forward(w: &[Vec<Q>], u: &Q, v: &Q) -> [Q; 2] {
        let (p, r) = (w.len() - 1, w[0].len() - 1);
        let mut d = q(0.);
        let mut a = q(0.);
        let mut b = q(0.);
        for i in 0..=p {
            for j in 0..=r {
                let h = &w[i][j] * basis(p, i, u) * basis(r, j, v);
                d += &h;
                a += &h * q(i as f64) / q(p as f64);
                b += h * q(j as f64) / q(r as f64);
            }
        }
        [a / &d, b / d]
    }
    #[test]
    fn all_degree_pairs_retain_independent_original_bernstein_preimages() {
        for p in 1..=8 {
            for r in 1..=8 {
                let w: Vec<Vec<Q>> = (0..=p)
                    .map(|i| {
                        (0..=r)
                            .map(|j| q((1 + (i + 2 * j) * (i + j + 3) % 7) as f64))
                            .collect()
                    })
                    .collect();
                let (u, v) = (q(0.5), q(0.25));
                let a = forward(&w, &u, &v);
                let input = a.map(|a| [a.clone(), a]);
                let e = enclosure(&w, &input).unwrap();
                assert!(e[0][0] <= u && u <= e[0][1], "degrees {p},{r}");
                assert!(e[1][0] <= v && v <= e[1][1], "degrees {p},{r}");
            }
        }
    }
    #[test]
    fn coupled_degree_two_refines_and_preserves_edges_and_rectangles() {
        let mut w = vec![vec![q(1.); 3]; 3];
        w[2][2] = q(17.);
        for (u, v) in [
            (0., 0.),
            (1., 1.),
            (0., 0.5),
            (1., 0.5),
            (0.5, 0.),
            (0.5, 1.),
            (0.5, 0.5),
        ] {
            let (u, v) = (q(u), q(v));
            let a = forward(&w, &u, &v);
            let input = a.map(|a| [a.clone(), a]);
            let e = enclosure(&w, &input).unwrap();
            assert!(e[0][0] <= u && u <= e[0][1]);
            assert!(e[1][0] <= v && v <= e[1][1]);
            assert!(&e[0][1] - &e[0][0] < q(0.5) && &e[1][1] - &e[1][0] < q(0.5));
        }
    }
}

#[cfg(test)]
#[test]
fn coupled_rectangles_retain_independent_interior_preimages_and_weight_gauge() {
    let mut w = vec![vec![q(1.); 3]; 3];
    w[2][2] = q(17.);
    let r = [[q(0.5), q(0.875)], [q(0.25), q(0.875)]];
    let e = enclosure(&w, &r).unwrap();
    let gauge: Vec<Vec<_>> = w
        .iter()
        .map(|row| row.iter().map(|x| x * q(7.)).collect())
        .collect();
    assert_eq!(e, enclosure(&gauge, &r).unwrap());
    // Original degree-two tensor basis evaluated independently at a grid.
    for i in 0..=8 {
        for j in 0..=8 {
            let (u, v) = (q(i as f64) / q(8.), q(j as f64) / q(8.));
            let bu = [
                (q(1.) - &u) * (q(1.) - &u),
                q(2.) * &u * (q(1.) - &u),
                &u * &u,
            ];
            let bv = [
                (q(1.) - &v) * (q(1.) - &v),
                q(2.) * &v * (q(1.) - &v),
                &v * &v,
            ];
            let mut d = q(0.);
            let mut a = q(0.);
            let mut b = q(0.);
            for x in 0..3 {
                for y in 0..3 {
                    let h = &w[x][y] * &bu[x] * &bv[y];
                    d += &h;
                    a += &h * q(x as f64) / q(2.);
                    b += h * q(y as f64) / q(2.);
                }
            }
            let a = a / &d;
            let b = b / d;
            if r[0][0] <= a && a <= r[0][1] && r[1][0] <= b && b <= r[1][1] {
                assert!(e[0][0] <= u && u <= e[0][1]);
                assert!(e[1][0] <= v && v <= e[1][1]);
            }
        }
    }
}
