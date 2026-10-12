use super::*;

/// Join blend v=1 to a ruled wall v=1, traversing the wall toward v=0.
/// Both charts use normalized u, reversed together at collapsed_end=1.
/// For the concatenated rectangle the bounds are f_u>=a, g_t>=b*u²,
/// |f_t|<=e*u and |g_u|<=c*u. Integrating along a segment gives determinant
/// >= (a*b-c*e)*integral(u²), by integral(u)²<=integral(u²).
/// Thus equal projections are excluded except along the blend's u=0 pole.
/// Exact matching of original seam controls/weights makes the map continuous.
/// This proves the chart union only; ownership and trimmed topology are separate.
pub fn certify_ruled_join(
    blend: &Surface,
    wall: &Surface,
    collapsed_end: usize,
    projection: [[f64; 3]; 2],
    subdivisions: usize,
    max_cells: usize,
) -> Result<JoinedReport> {
    blend.validate()?;
    wall.validate()?;
    check(
        collapsed_end <= 1
            && subdivisions > 0
            && subdivisions <= 64
            && max_cells > 0
            && max_cells <= 100000
            && projection.iter().flatten().all(|x| x.is_finite()),
        "Joined projection requires finite rows and bounded work",
    )?;
    let mut out = JoinedReport {
        proven: false,
        reason: "joined-projection-not-proven",
        cells: 0,
        weighted_bounds: None,
        dominance_margin_lower: None,
    };
    if 2 * subdivisions * subdivisions > max_cells {
        out.reason = "work-limit";
        return Ok(out);
    }
    let p = blend.degree_u;
    let q = blend.degree_v;
    let clamped = |k: &[f64], d: usize| {
        k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|v| *v == k[d])
            && k[d + 1..].iter().all(|v| *v == k[d + 1])
    };
    if p == 0
        || p > 8
        || wall.degree_u != p
        || wall.degree_v != 1
        || wall.periodic_u
        || wall.periodic_v
        || wall.control_points.len() != p + 1
        || wall.control_points[0].len() != 2
        || !clamped(&wall.knots_u, p)
        || !clamped(&wall.knots_v, 1)
    {
        out.reason = "unsupported-wall";
        return Ok(out);
    }
    if blend.control_points.len() != p + 1 || blend.control_points[0].len() != q + 1 {
        out.reason = "unsupported-blend";
        return Ok(out);
    }
    for i in 0..=p {
        if wall.control_points[i][1] != blend.control_points[i][q]
            || wall.weights[i][1] != blend.weights[i][q]
            || wall.weights[i][0] != wall.weights[i][1]
        {
            out.reason = "source-seam-not-identical";
            return Ok(out);
        }
        if (0..3).any(|k| {
            projection[0][k] != 0. && wall.control_points[i][0][k] != wall.control_points[i][1][k]
        }) {
            out.reason = "wall-first-projection-not-constant";
            return Ok(out);
        }
    }
    let index = |i| if collapsed_end == 0 { i } else { p - i };
    // These exact source equalities establish g_u(0,t)=0 regardless of
    // differing u weights. Preserve that known factor rather than accepting
    // independently rounded polynomial cancellations as an exact zero.
    if (0..2).any(|j| {
        (0..3).any(|k| {
            projection[1][k] != 0.
                && wall.control_points[index(0)][j][k] != wall.control_points[index(1)][j][k]
        })
    }) {
        out.reason = "wall-weighted-order-not-proven";
        return Ok(out);
    }
    let first = certify(
        blend,
        collapsed_end,
        projection,
        subdivisions,
        subdivisions * subdivisions,
    )?;
    out.cells = first.cells;
    let Some(bounds) = first.weighted_bounds else {
        out.reason = first.reason;
        return Ok(out);
    };
    if first.cells != subdivisions * subdivisions {
        out.reason = "incomplete-blend-bounds";
        return Ok(out);
    }
    let pole = &blend.control_points[index(0)][0];
    let mut coordinates: [Poly; 3] = std::array::from_fn(|_| vec![vec![I::point(0.); 2]; p + 1]);
    for i in 0..=p {
        for j in 0..2 {
            let point = &wall.control_points[index(i)][1 - j];
            let weight = I::point(wall.weights[index(i)][1 - j]);
            coordinates[2][i][j] = weight;
            for row in 0..2 {
                let mut value = I::point(0.);
                for k in 0..3 {
                    if point[k] != pole[k] && projection[row][k] != 0. {
                        value = add(
                            value,
                            I::point(point[k])
                                .sub(I::point(pole[k]))?
                                .mul(I::point(projection[row][k]))?,
                        )?;
                    }
                }
                coordinates[row][i][j] = mul(value, weight)?;
            }
        }
    }
    let [f, g, w] = coordinates;
    let numerator = |n: &Poly, axis| -> Result<Poly> {
        combine(
            &product(&derivative(n, axis)?, &w)?,
            &product(n, &derivative(&w, axis)?)?,
            -1.,
        )
    };
    let fu = numerator(&f, 0)?;
    let gv = numerator(&g, 1)?;
    let mut gu = numerator(&g, 0)?;
    for v in &mut gu[0] {
        *v = I::point(0.);
    } // Exact source identity checked above.
    let Some(gu) = factor_u(&gu, 1)? else {
        out.reason = "wall-weighted-order-not-proven";
        return Ok(out);
    };
    let mut a = f64::INFINITY;
    let mut b = f64::INFINITY;
    let mut c = 0_f64;
    for i in 0..subdivisions {
        for j in 0..subdivisions {
            let domain = [
                [
                    i as f64 / subdivisions as f64,
                    (i + 1) as f64 / subdivisions as f64,
                ],
                [
                    j as f64 / subdivisions as f64,
                    (j + 1) as f64 / subdivisions as f64,
                ],
            ];
            out.cells += 1;
            let weight = bound(&restrict(&w, domain)?);
            if weight.lo <= 0. {
                out.reason = "wall-weight-not-separated";
                return Ok(out);
            }
            let denominator = weight.mul(weight)?;
            a = a.min(bound(&restrict(&fu, domain)?).div(denominator)?.lo);
            // u²<=1, so a positive unweighted wall bound also bounds b*u².
            b = b.min(bound(&restrict(&gv, domain)?).div(denominator)?.lo);
            let value = bound(&restrict(&gu, domain)?).div(denominator)?;
            c = c.max(value.lo.abs().max(value.hi.abs()));
        }
    }
    let [a, b, e, c] = [
        a.min(bounds[0]),
        b.min(bounds[1]),
        bounds[2],
        c.max(bounds[3]),
    ];
    out.weighted_bounds = Some([a, b, e, c]);
    let margin = I::point(a)
        .mul(I::point(b))?
        .sub(I::point(c).mul(I::point(e))?)?
        .lo;
    out.dominance_margin_lower = Some(margin);
    if a > 0. && b > 0. && margin > 0. {
        out.proven = true;
        out.reason = "global-joined-weighted-dominance";
    }
    Ok(out)
}

pub(super) fn zero(x: I) -> bool {
    x.lo == 0. && x.hi == 0.
}
pub(super) fn add(a: I, b: I) -> Result<I> {
    if zero(a) {
        Ok(b)
    } else if zero(b) {
        Ok(a)
    } else {
        a.add(b)
    }
}
pub(super) fn mul(a: I, b: I) -> Result<I> {
    if zero(a) || zero(b) {
        Ok(I::point(0.))
    } else {
        a.mul(b)
    }
}
pub(super) fn sub(a: I, b: I) -> Result<I> {
    if zero(a) && zero(b) {
        Ok(I::point(0.))
    } else {
        a.sub(b)
    }
}
fn choose(n: usize, k: usize) -> f64 {
    let mut v = 1u64;
    for i in 0..k.min(n - k) {
        v = v * (n - i) as u64 / (i + 1) as u64;
    }
    v as f64
}
pub(super) fn product(a: &Poly, b: &Poly) -> Result<Poly> {
    let (p, q, r, t) = (a.len() - 1, a[0].len() - 1, b.len() - 1, b[0].len() - 1);
    let mut out = vec![vec![I::point(0.); q + t + 1]; p + r + 1];
    for i in 0..=p {
        for j in 0..=q {
            for k in 0..=r {
                for l in 0..=t {
                    if zero(a[i][j]) || zero(b[k][l]) {
                        continue;
                    }
                    let u = I::point(choose(p, i))
                        .mul(I::point(choose(r, k)))?
                        .div(I::point(choose(p + r, i + k)))?;
                    let v = I::point(choose(q, j))
                        .mul(I::point(choose(t, l)))?
                        .div(I::point(choose(q + t, j + l)))?;
                    out[i + k][j + l] =
                        add(out[i + k][j + l], a[i][j].mul(b[k][l])?.mul(u)?.mul(v)?)?;
                }
            }
        }
    }
    Ok(out)
}
pub(super) fn combine(a: &Poly, b: &Poly, scale: f64) -> Result<Poly> {
    a.iter()
        .zip(b)
        .map(|(a, b)| {
            a.iter()
                .zip(b)
                .map(|(&a, &b)| add(a, mul(b, I::point(scale))?))
                .collect()
        })
        .collect()
}
pub(super) fn derivative(a: &Poly, axis: usize) -> Result<Poly> {
    let (p, q) = (a.len() - 1, a[0].len() - 1);
    (0..=p - usize::from(axis == 0))
        .map(|i| {
            (0..=q - usize::from(axis == 1))
                .map(|j| {
                    mul(
                        sub(
                            a[i + usize::from(axis == 0)][j + usize::from(axis == 1)],
                            a[i][j],
                        )?,
                        I::point([p, q][axis] as f64),
                    )
                })
                .collect()
        })
        .collect()
}
pub(super) fn factor_u(a: &Poly, power: usize) -> Result<Option<Poly>> {
    let n = a.len() - 1;
    if n < power || a[..power].iter().flatten().any(|&x| !zero(x)) {
        return Ok(None);
    }
    Ok(Some(
        (power..=n)
            .map(|i| {
                let scale = I::point(choose(n, i)).div(I::point(choose(n - power, i - power)))?;
                a[i].iter().map(|&x| mul(x, scale)).collect()
            })
            .collect::<Result<Poly>>()?,
    ))
}
pub(super) fn bound(a: &Poly) -> I {
    a.iter().flatten().fold(
        I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        },
        |a, &b| I {
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
        },
    )
}
fn split(a: &[I], t: I) -> Result<(Vec<I>, Vec<I>)> {
    let mut row = a.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    let one = I::point(1.).sub(t)?;
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| add(mul(p[0], one)?, mul(p[1], t)?))
            .collect::<Result<_>>()?;
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    Ok((left, right))
}
fn restrict_curve(a: &[I], lo: f64, hi: f64) -> Result<Vec<I>> {
    let right = if lo == 0. {
        a.to_vec()
    } else {
        split(a, I::point(lo))?.1
    };
    if hi == 1. {
        return Ok(right);
    }
    let t = I::point(hi)
        .sub(I::point(lo))?
        .div(I::point(1.).sub(I::point(lo))?)?;
    Ok(split(&right, t)?.0)
}
pub(super) fn restrict(a: &Poly, domain: [[f64; 2]; 2]) -> Result<Poly> {
    let mut out = vec![vec![I::point(0.); a[0].len()]; a.len()];
    for j in 0..a[0].len() {
        let column = a.iter().map(|r| r[j]).collect::<Vec<_>>();
        let restricted = restrict_curve(&column, domain[0][0], domain[0][1])?;
        for (i, x) in restricted.into_iter().enumerate() {
            out[i][j] = x;
        }
    }
    out.iter()
        .map(|row| restrict_curve(row, domain[1][0], domain[1][1]))
        .collect()
}
