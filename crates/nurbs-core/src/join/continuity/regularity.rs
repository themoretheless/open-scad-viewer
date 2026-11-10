//! Outward-rounded Bernstein enclosure of the rational boundary normal.
//! A nonzero component over every subinterval proves regularity of the seam.
use super::Boundary;
use super::SeamRegularity;
use crate::{Result, surface::Surface};
pub(super) use crate::numerics::outward_interval::{I, choose, insert};
type Pair = [I; 8];
fn product(a: &[I], b: &[I]) -> Vec<I> {
    let m = a.len() - 1;
    let n = b.len() - 1;
    (0..=m + n)
        .map(|k| {
            let mut x = I::point(0.);
            for i in k.saturating_sub(n)..=k.min(m) {
                let factor = choose(m, i)
                    .mul(choose(n, k - i))
                    .div_positive(choose(m + n, k))
                    .unwrap();
                x = x.add(factor.mul(a[i]).mul(b[k - i]));
            }
            x
        })
        .collect()
}
fn difference(a: Vec<I>, b: Vec<I>) -> Vec<I> {
    a.into_iter().zip(b).map(|(x, y)| x.sub(y)).collect()
}
fn normal_net(points: &[Pair]) -> [Vec<I>; 3] {
    let p = points.len() - 1;
    let component = |index: usize| points.iter().map(|h| h[index]).collect::<Vec<_>>();
    let derivative = |index: usize| {
        points
            .windows(2)
            .map(|h| h[1][index].sub(h[0][index]).mul(I::point(p as f64)))
            .collect::<Vec<_>>()
    };
    let w = component(3);
    let dw = derivative(3);
    let inward_w = component(7);
    let along: Vec<Vec<I>> = (0..3)
        .map(|a| difference(product(&derivative(a), &w), product(&component(a), &dw)))
        .collect();
    let cross: Vec<Vec<I>> = (0..3)
        .map(|a| {
            difference(
                product(&component(4 + a), &w),
                product(&component(a), &inward_w),
            )
        })
        .collect();
    std::array::from_fn(|a| {
        let b = (a + 1) % 3;
        let c = (a + 2) % 3;
        difference(product(&cross[b], &along[c]), product(&cross[c], &along[b]))
    })
}
fn split(a: &[I]) -> (Vec<I>, Vec<I>) {
    let mut row = a.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| p[0].add(p[1]).mul(I::point(0.5)))
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
pub(super) fn certify(surface: &Surface, b: Boundary) -> Result<SeamRegularity> {
    certify_impl(surface, b, false)
}
/// Only after an independent exact proof establishes smoothness at every
/// internal along knot. Span normals are still independently enclosed.
pub(super) fn certify_after_exact_along_jets(
    surface: &Surface,
    b: Boundary,
) -> Result<SeamRegularity> {
    certify_impl(surface, b, true)
}
fn certify_impl(surface: &Surface, b: Boundary, exact_along_jets: bool) -> Result<SeamRegularity> {
    let (p, n, k, _) = b.along(surface);
    let a = k[p];
    let z = k[n];
    let mut breaks = k[p..=n].to_vec();
    breaks.dedup();
    for &u in &breaks {
        if !exact_along_jets && u > a && u < z && k.iter().filter(|&&x| x == u).count() >= p {
            return Ok(SeamRegularity::NonC1Basis {
                unresolved_intervals: vec![[a, z]],
            });
        }
    }
    let mut controls: Vec<Pair> = (0..n)
        .map(|i| {
            std::array::from_fn(|c| {
                let (u, v) = b.index(surface, i, c / 4);
                let axis = c % 4;
                let w = I::point(surface.weights[u][v]);
                if axis == 3 {
                    w
                } else {
                    I::point(surface.control_points[u][v][axis]).mul(w)
                }
            })
        })
        .collect();
    let mut knots = k.to_vec();
    for &u in &breaks {
        let target = if u == a || u == z { p + 1 } else { p };
        while knots.iter().filter(|&&x| x == u).count() < target {
            insert(&mut knots, &mut controls, p, u)?;
        }
    }
    let start = knots.iter().position(|&x| x == a).unwrap();
    let end = knots.iter().rposition(|&x| x == z).unwrap();
    knots = knots[start..=end].to_vec();
    let count = knots.len() - p - 1;
    controls = controls[start..start + count].to_vec();
    let mut pending = Vec::new();
    let mut spans = 0;
    for i in p..count {
        if knots[i] < knots[i + 1] {
            pending.push((
                normal_net(&controls[i - p..=i]),
                [knots[i], knots[i + 1]],
                0,
            ));
            spans += 1;
        }
    }
    let mut inspected = 0;
    let mut accepted = 0;
    let mut unresolved = Vec::new();
    let mut resource = false;
    while let Some((net, domain, depth)) = pending.pop() {
        inspected += 1;
        if inspected > 4096 {
            resource = true;
            unresolved.push(domain);
            break;
        }
        let separated = net.iter().any(|v| {
            v.iter().all(|x| x.finite() && x.lo > 0.) || v.iter().all(|x| x.finite() && x.hi < 0.)
        });
        if separated {
            accepted += 1;
            continue;
        }
        let mid = domain[0] + (domain[1] - domain[0]) * 0.5;
        if depth >= 12
            || mid == domain[0]
            || mid == domain[1]
            || net.iter().flatten().any(|x| !x.finite())
        {
            unresolved.push(domain);
            continue;
        }
        let split: Vec<_> = net.iter().map(|v| split(v)).collect();
        pending.push((
            std::array::from_fn(|i| split[i].1.clone()),
            [mid, domain[1]],
            depth + 1,
        ));
        pending.push((
            std::array::from_fn(|i| split[i].0.clone()),
            [domain[0], mid],
            depth + 1,
        ));
    }
    let unresolved_count = unresolved.len() + pending.len();
    unresolved.extend(pending.into_iter().map(|(_, domain, _)| domain));
    unresolved.truncate(32);
    Ok(SeamRegularity::Bernstein {
        domain: [a, z],
        spans,
        inspected_cells: inspected,
        accepted_cells: accepted,
        unresolved_count,
        unresolved_intervals: unresolved,
        resource_limit_reached: resource,
    })
}
