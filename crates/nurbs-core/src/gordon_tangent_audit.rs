//! Whole-boundary tangent comparison against the original tensor controls.
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface};
use value_codec::{Value, json};
fn choose(n: usize, k: usize) -> Result<I> {
    (0..k.min(n - k)).try_fold(I::point(1.), |v, i| {
        v.mul(I::point((n - i) as f64))?
            .div(I::point((i + 1) as f64))
    })
}
fn product(a: &[I], b: &[I]) -> Result<Vec<I>> {
    let (n, m) = (a.len() - 1, b.len() - 1);
    let mut result = vec![I::point(0.); n + m + 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            let factor = choose(n, i)?
                .mul(choose(m, j)?)?
                .div(choose(n + m, i + j)?)?;
            result[i + j] = result[i + j].add(x.mul(*y)?.mul(factor)?)?;
        }
    }
    Ok(result)
}
fn restrict(
    net: &[Vec<I>],
    knots: &[f64],
    degree: usize,
    span: usize,
    domain: [f64; 2],
) -> Result<Vec<Vec<I>>> {
    let mut result = Vec::new();
    for k in 0..=degree {
        let mut q = net[span - degree..=span].to_vec();
        for r in 1..=degree {
            let parameter = if r - 1 < degree - k {
                domain[0]
            } else {
                domain[1]
            };
            for j in (r..=degree).rev() {
                let i = span - degree + j;
                let alpha = I::point(parameter)
                    .sub(I::point(knots[i]))?
                    .div(I::point(knots[i + degree - r + 1]).sub(I::point(knots[i]))?)?
                    .intersect(0., 1.)?;
                let beta = I::point(1.).sub(alpha)?.intersect(0., 1.)?;
                for axis in 0..q[j].len() {
                    let a = q[j - 1][axis];
                    let b = q[j][axis];
                    q[j][axis] = a
                        .mul(beta)?
                        .add(b.mul(alpha)?)?
                        .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
                }
            }
        }
        result.push(q[degree].clone());
    }
    Ok(result)
}
fn owning_span(knots: &[f64], degree: usize, count: usize, domain: [f64; 2]) -> usize {
    (degree..count)
        .find(|&i| knots[i] <= domain[0] && knots[i + 1] >= domain[1] && knots[i] < knots[i + 1])
        .unwrap()
}
fn bound(surface: &Surface, target: &Curve, end: bool, domain: [f64; 2]) -> Result<f64> {
    let nv = surface.control_points[0].len();
    let p = surface.degree_u;
    let (owner, neighbor, extent, sign) = if end {
        (
            nv - 1,
            nv - 2,
            I::point(surface.knots_v[nv]).sub(I::point(surface.knots_v[nv - 1]))?,
            -1.,
        )
    } else {
        (
            0,
            1,
            I::point(surface.knots_v[surface.degree_v + 1])
                .sub(I::point(surface.knots_v[surface.degree_v]))?,
            1.,
        )
    };
    let factor = I::point(sign * surface.degree_v as f64).div(extent)?;
    let scale = surface.weights.iter().flatten().copied().fold(0., f64::max);
    let origin = &surface.control_points[0][owner];
    let mut net = Vec::new();
    for i in 0..surface.control_points.len() {
        let w = I::point(surface.weights[i][owner]).div(I::point(scale))?;
        let other = I::point(surface.weights[i][neighbor]).div(I::point(scale))?;
        let mut row = Vec::new();
        for k in 0..3 {
            row.push(
                I::point(surface.control_points[i][owner][k])
                    .sub(I::point(origin[k]))?
                    .mul(w)?,
            );
        }
        row.push(w);
        for k in 0..3 {
            row.push(
                I::point(surface.control_points[i][neighbor][k])
                    .sub(I::point(origin[k]))?
                    .mul(other)?
                    .sub(row[k])?
                    .mul(factor)?,
            );
        }
        row.push(other.sub(w)?.mul(factor)?);
        net.push(row);
    }
    let span = owning_span(&surface.knots_u, p, surface.control_points.len(), domain);
    let net = restrict(&net, &surface.knots_u, p, span, domain)?;
    let column = |k: usize| net.iter().map(|p| p[k]).collect::<Vec<_>>();
    let w = column(3);
    let dw = column(7);
    let denominator = product(&w, &w)?;
    let ts = target.weights.iter().copied().fold(0., f64::max);
    let target_net = target
        .control_points
        .iter()
        .zip(&target.weights)
        .map(|(point, &weight)| {
            let w = I::point(weight).div(I::point(ts))?;
            let mut row = point
                .iter()
                .map(|&x| I::point(x).mul(w))
                .collect::<Result<Vec<_>>>()?;
            row.push(w);
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    let span = owning_span(
        &target.knots,
        target.degree,
        target.control_points.len(),
        domain,
    );
    let target_net = restrict(&target_net, &target.knots, target.degree, span, domain)?;
    let tw = target_net.iter().map(|p| p[3]).collect::<Vec<_>>();
    let divisor = product(&denominator, &tw)?;
    let minimum = divisor.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
    crate::numeric(minimum > 0., "Tangent audit denominator is unresolved")?;
    let mut squared = I::point(0.);
    for k in 0..3 {
        let a = product(&column(k + 4), &w)?;
        let b = product(&column(k), &dw)?;
        let numerator = a
            .iter()
            .zip(b)
            .map(|(a, b)| a.sub(b))
            .collect::<Result<Vec<_>>>()?;
        let a = product(&numerator, &tw)?;
        let b = product(
            &target_net.iter().map(|p| p[k]).collect::<Vec<_>>(),
            &denominator,
        )?;
        let upper = a
            .iter()
            .zip(b)
            .map(|(a, b)| a.sub(b).map(|d| d.lo.abs().max(d.hi.abs())))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .fold(0., f64::max);
        let bound = I::point(upper).div(I::point(minimum))?;
        squared = squared.add(bound.mul(bound)?)?;
    }
    Ok(squared.hi.sqrt().next_up())
}
/// Entire U cover, including each original knot span. Exhaustion is not acceptance.
pub(super) fn verify(
    surface: &Surface,
    targets: &[Curve],
    end: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Value> {
    surface.validate()?;
    check(
        !surface.periodic_u
            && !surface.periodic_v
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=1_000_000).contains(&max_cells)
            && !targets.is_empty(),
        "Invalid tangent audit inputs",
    )?;
    let nv = surface.control_points[0].len();
    let va = surface.knots_v[surface.degree_v];
    let vb = surface.knots_v[nv];
    check(
        surface
            .knots_v
            .iter()
            .take(surface.degree_v + 1)
            .all(|v| *v == va)
            && surface
                .knots_v
                .iter()
                .rev()
                .take(surface.degree_v + 1)
                .all(|v| *v == vb),
        "Tangent audit needs clamped V boundaries",
    )?;
    let domain = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    for target in targets {
        target.validate()?;
        check(
            !target.periodic && target.control_points[0].len() == 3,
            "Invalid tangent target",
        )?;
    }
    check(
        targets[0].domain()[0] == domain[0]
            && targets.last().unwrap().domain()[1] == domain[1]
            && targets
                .windows(2)
                .all(|p| p[0].domain()[1] == p[1].domain()[0]),
        "Tangent targets must cover the complete U domain",
    )?;
    let mut pending = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        let [a, b] = target.domain();
        let mut breaks = vec![a, b];
        breaks.extend(
            surface
                .knots_u
                .iter()
                .chain(&target.knots)
                .copied()
                .filter(|u| *u > a && *u < b),
        );
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        pending.extend(breaks.windows(2).map(|p| (index, [p[0], p[1]])));
    }
    let mut cells = 0;
    let mut upper: f64 = 0.;
    let mut reason = None;
    while let Some((index, [a, b])) = pending.pop() {
        if cells == max_cells {
            reason = Some("cell_budget");
            break;
        }
        cells += 1;
        match bound(surface, &targets[index], end, [a, b]) {
            Ok(error) if error <= tolerance => {
                upper = upper.max(error);
            }
            Ok(_) => {
                let mid = a + (b - a) * 0.5;
                if mid == a || mid == b {
                    reason = Some("parameter_precision");
                    break;
                }
                pending.push((index, [a, mid]));
                pending.push((index, [mid, b]));
            }
            Err(_) => {
                reason = Some("numerical_enclosure");
                break;
            }
        }
    }
    Ok(
        json!({"operation":"cartesian-boundary-tangent-retention","exact":false,
        "accepted":reason.is_none(),"errorUpper":if reason.is_none(){Some(upper)}else{None},
        "tolerance":tolerance,"cells":cells,"maxCells":max_cells,"domain":domain,"end":end,
        "unresolvedReason":reason,"method":"outward-original-tensor-quotient-rule-Bernstein-residual"}),
    )
}
