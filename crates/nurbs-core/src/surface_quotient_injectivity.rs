//! Sufficient global injectivity on a rectangular chart with one collapsed
//! parameter boundary. Only that entire boundary is identified to one point.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
type Poly = Vec<Vec<I>>;

#[derive(Clone, Debug)]
pub struct Report {
    pub proven: bool,
    pub cells: usize,
    pub reason: &'static str,
    /// Bounds a,b,e,c for f_u>=a, g_v>=b*u², |f_v|<=e*u, |g_u|<=c*u.
    pub weighted_bounds: Option<[f64; 4]>,
    pub dominance_margin_lower: Option<f64>,
    /// Conditional dominance margins when the first projection restricts
    /// where a second, potentially coincident parameter point could lie.
    pub band_margins_lower: Option<Vec<f64>>,
    /// Original controls defining an exact orthogonal source frame: pole,
    /// adjacent-row point, and the two meridian controls in the next row.
    pub source_frame: Option<[[f64; 3]; 4]>,
}
enum Projection {
    Fixed([[f64; 3]; 2]),
    Source,
}

/// Work in normalized u,v and reverse u for collapsed_end=1. The projection
/// uses two linear rows on positions relative to the collapsed point. All
/// signs and exact factors come from original controls, never sampled jets.
pub fn certify(
    s: &Surface,
    collapsed_end: usize,
    projection: [[f64; 3]; 2],
    subdivisions: usize,
    max_cells: usize,
) -> Result<Report> {
    certify_inner(
        s,
        collapsed_end,
        Projection::Fixed(projection),
        subdivisions,
        max_cells,
    )
}
/// The source frame is defined without rounding its orthogonality: t=A-P,
/// h=B1-B0, f=t.(S-P), g=(h-(h.t)/(t.t)*t).(S-P). Interval coefficients
/// enclose this exact rational expression. Its known value at A is exactly
/// zero for g, regardless of rigid placement or binary64 rounding of controls.
pub fn certify_source_frame(
    s: &Surface,
    collapsed_end: usize,
    subdivisions: usize,
    max_cells: usize,
) -> Result<Report> {
    certify_inner(
        s,
        collapsed_end,
        Projection::Source,
        subdivisions,
        max_cells,
    )
}
fn certify_inner(
    s: &Surface,
    collapsed_end: usize,
    projection: Projection,
    subdivisions: usize,
    max_cells: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        collapsed_end <= 1
            && subdivisions > 0
            && subdivisions <= 64
            && max_cells > 0
            && max_cells <= 100_000
            && match &projection {
                Projection::Fixed(p) => p.iter().flatten().all(|x| x.is_finite()),
                Projection::Source => true,
            },
        "Quotient injectivity requires endpoint 0/1, finite projection, 1..64 subdivisions and 1..100000 cells",
    )?;
    let mut report = Report {
        proven: false,
        cells: 0,
        reason: "weighted-projection-not-proven",
        weighted_bounds: None,
        dominance_margin_lower: None,
        band_margins_lower: None,
        source_frame: None,
    };
    let (p, q) = (s.degree_u, s.degree_v);
    let clamped = |k: &[f64], d: usize| {
        k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|v| *v == k[d])
            && k[d + 1..].iter().all(|v| *v == k[d + 1])
    };
    if s.periodic_u
        || s.periodic_v
        || p > 8
        || q > 8
        || s.control_points.len() != p + 1
        || s.control_points[0].len() != q + 1
        || !clamped(&s.knots_u, p)
        || !clamped(&s.knots_v, q)
    {
        report.reason = "unsupported-chart";
        return Ok(report);
    }
    let index = |i| if collapsed_end == 0 { i } else { p - i };
    let pole = &s.control_points[index(0)][0];
    if !s.control_points[index(0)].iter().all(|point| point == pole) {
        report.reason = "boundary-not-collapsed";
        return Ok(report);
    }
    let coefficients = match projection {
        Projection::Fixed(p) => p.map(|row| row.map(I::point)),
        Projection::Source => {
            if p < 2 {
                report.reason = "unsupported-chart";
                return Ok(report);
            }
            let frame: [[f64; 3]; 4] = [
                pole.as_slice().try_into().unwrap(),
                s.control_points[index(1)][0].as_slice().try_into().unwrap(),
                s.control_points[index(2)][0].as_slice().try_into().unwrap(),
                s.control_points[index(2)][q].as_slice().try_into().unwrap(),
            ];
            report.source_frame = Some(frame);
            let difference = |a: [f64; 3], b: [f64; 3]| -> Result<[I; 3]> {
                let mut result = [I::point(0.); 3];
                for k in 0..3 {
                    if a[k] != b[k] {
                        result[k] = I::point(a[k]).sub(I::point(b[k]))?;
                    }
                }
                Ok(result)
            };
            let dot = |a: [I; 3], b: [I; 3]| -> Result<I> {
                let mut result = I::point(0.);
                for k in 0..3 {
                    result = add(result, mul(a[k], b[k])?)?;
                }
                Ok(result)
            };
            let tangent = difference(frame[1], frame[0])?;
            let meridian = difference(frame[3], frame[2])?;
            let length_squared = dot(tangent, tangent)?;
            if length_squared.lo <= 0. {
                report.reason = "frame-not-separated";
                return Ok(report);
            }
            let alpha = dot(meridian, tangent)?.div(length_squared)?;
            let mut normal = [I::point(0.); 3];
            for k in 0..3 {
                normal[k] = sub(meridian[k], mul(alpha, tangent[k])?)?;
            }
            [tangent, normal]
        }
    };
    let mut coordinates: [Poly; 3] =
        std::array::from_fn(|_| vec![vec![I::point(0.); q + 1]; p + 1]);
    for i in 0..=p {
        for j in 0..=q {
            let point = &s.control_points[index(i)][j];
            let weight = I::point(s.weights[index(i)][j]);
            coordinates[2][i][j] = weight;
            for row in 0..2 {
                if row == 1
                    && report
                        .source_frame
                        .as_ref()
                        .is_some_and(|frame| point.as_slice() == frame[1].as_slice())
                {
                    // Exact orthogonality of the constructed expression, not
                    // a tolerance test on independently rounded coefficients.
                    coordinates[row][i][j] = I::point(0.);
                    continue;
                }
                let mut value = I::point(0.);
                for k in 0..3 {
                    // Equality of binary64 input coordinates establishes exact
                    // zero differences. Preserve them without tolerance snapping.
                    if point[k] == pole[k] || zero(coefficients[row][k]) {
                        continue;
                    }
                    value = add(
                        value,
                        I::point(point[k])
                            .sub(I::point(pole[k]))?
                            .mul(coefficients[row][k])?,
                    )?;
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
    let Some(fv) = factor_u(&numerator(&f, 1)?, 1)? else {
        report.reason = "weighted-order-not-proven";
        return Ok(report);
    };
    let Some(gu) = factor_u(&numerator(&g, 0)?, 1)? else {
        report.reason = "weighted-order-not-proven";
        return Ok(report);
    };
    let Some(gv) = factor_u(&numerator(&g, 1)?, 2)? else {
        report.reason = "weighted-order-not-proven";
        return Ok(report);
    };
    if subdivisions * subdivisions > max_cells {
        report.reason = "work-limit";
        return Ok(report);
    }
    let mut a = f64::INFINITY;
    let mut b = f64::INFINITY;
    let mut e = 0_f64;
    let mut c = 0_f64;
    let mut bands = vec![[f64::INFINITY, 0., 0.]; subdivisions];
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
            report.cells += 1;
            let weight = bound(&restrict(&w, domain)?);
            if weight.lo <= 0. {
                report.reason = "weight-not-separated";
                return Ok(report);
            }
            let denominator = weight.mul(weight)?;
            let fa = bound(&restrict(&fu, domain)?).div(denominator)?;
            let gb = bound(&restrict(&gv, domain)?).div(denominator)?;
            let fe = bound(&restrict(&fv, domain)?).div(denominator)?;
            let gc = bound(&restrict(&gu, domain)?).div(denominator)?;
            a = a.min(fa.lo);
            b = b.min(gb.lo);
            e = e.max(fe.lo.abs().max(fe.hi.abs()));
            c = c.max(gc.lo.abs().max(gc.hi.abs()));
            bands[i][0] = bands[i][0].min(gb.lo);
            bands[i][1] = bands[i][1].max(fe.lo.abs().max(fe.hi.abs()));
            bands[i][2] = bands[i][2].max(gc.lo.abs().max(gc.hi.abs()));
        }
    }
    report.weighted_bounds = Some([a, b, e, c]);
    let margin = I::point(a)
        .mul(I::point(b))?
        .sub(I::point(c).mul(I::point(e))?)?
        .lo;
    report.dominance_margin_lower = Some(margin);
    if a > 0. && b > 0. && margin > 0. {
        report.proven = true;
        report.reason = "global-weighted-quotient-dominance";
    } else if a > 0. && b > 0. {
        let mut margins = Vec::new();
        for i in 0..subdivisions {
            // If f agrees and u2>=u1, a*(u2-u1)<=E*u2*|v2-v1|.
            // Since |v2-v1|<=1, u1>=u2*(1-E/a). Bound g_u only
            // over this necessary range, with outward endpoints.
            let [band_b, band_e, _] = bands[i];
            let ratio = I::point(band_e).div(I::point(a))?.hi;
            let lower = if ratio >= 1. {
                0.
            } else {
                I::point(i as f64 / subdivisions as f64)
                    .mul(I::point(1.).sub(I::point(ratio))?)?
                    .lo
                    .max(0.)
            };
            let upper = (i + 1) as f64 / subdivisions as f64;
            let band_c = bands
                .iter()
                .enumerate()
                .filter(|(k, _)| {
                    (*k + 1) as f64 / subdivisions as f64 >= lower
                        && *k as f64 / subdivisions as f64 <= upper
                })
                .map(|(_, bounds)| bounds[2])
                .fold(0., f64::max);
            margins.push(
                I::point(a)
                    .mul(I::point(band_b))?
                    .sub(I::point(band_c).mul(I::point(band_e))?)?
                    .lo,
            );
        }
        let minimum = margins.iter().copied().fold(f64::INFINITY, f64::min);
        report.band_margins_lower = Some(margins);
        report.dominance_margin_lower = Some(minimum);
        if minimum > 0. {
            report.proven = true;
            report.reason = "global-localized-weighted-quotient-dominance";
        }
    }
    Ok(report)
}
fn zero(x: I) -> bool {
    x.lo == 0. && x.hi == 0.
}
fn add(a: I, b: I) -> Result<I> {
    if zero(a) {
        Ok(b)
    } else if zero(b) {
        Ok(a)
    } else {
        a.add(b)
    }
}
fn mul(a: I, b: I) -> Result<I> {
    if zero(a) || zero(b) {
        Ok(I::point(0.))
    } else {
        a.mul(b)
    }
}
fn sub(a: I, b: I) -> Result<I> {
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
fn product(a: &Poly, b: &Poly) -> Result<Poly> {
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
fn combine(a: &Poly, b: &Poly, scale: f64) -> Result<Poly> {
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
fn derivative(a: &Poly, axis: usize) -> Result<Poly> {
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
fn factor_u(a: &Poly, power: usize) -> Result<Option<Poly>> {
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
fn bound(a: &Poly) -> I {
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
fn restrict(a: &Poly, domain: [[f64; 2]; 2]) -> Result<Poly> {
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

#[cfg(test)]
mod tests {
    use super::*;
    fn triangular() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 2],
                vec![vec![0.5, 0., 0.]; 2],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn collapsed_triangle_is_injective_only_after_identifying_its_pole_boundary() {
        let mut s = triangular();
        let projection = [[1., 0., 0.], [0., 1., 0.]];
        let before = format!("{s:?}");
        let r = certify(&s, 0, projection, 4, 16).unwrap();
        assert!(r.proven, "{r:?}");
        assert_eq!(r.cells, 16);
        assert!(
            !crate::surface_injectivity::certify(&s, 1000)
                .unwrap()
                .proven
        );
        assert_eq!(format!("{s:?}"), before);
        assert!(!certify(&s, 0, projection, 4, 15).unwrap().proven);
        assert!(certify(&s, 0, projection, 0, 16).is_err());
        assert!(certify(&s, 2, projection, 4, 16).is_err());
        s.control_points.reverse();
        s.weights.reverse();
        assert!(certify(&s, 1, projection, 4, 16).unwrap().proven);
        s.control_points.reverse();
        s.weights = vec![vec![1., 3.], vec![2., 6.], vec![4., 12.]];
        assert!(certify(&s, 0, projection, 8, 64).unwrap().proven);
        s.control_points[1][1][1] = 1e-12;
        let r = certify(&s, 0, projection, 8, 64).unwrap();
        assert!(!r.proven);
        assert_eq!(r.reason, "weighted-order-not-proven");
    }
    #[test]
    fn folding_the_transverse_parameter_is_not_hidden_by_a_collapsed_boundary() {
        let mut s = triangular();
        s.degree_v = 2;
        s.knots_v = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = vec![
            vec![vec![0., 0., 0.]; 3],
            vec![vec![0.5, 0., 0.]; 3],
            vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![1., 0., 0.]],
        ];
        s.weights = vec![vec![1.; 3]; 3];
        assert!(
            !certify(&s, 0, [[1., 0., 0.], [0., 1., 0.]], 8, 64)
                .unwrap()
                .proven
        );
    }
}
