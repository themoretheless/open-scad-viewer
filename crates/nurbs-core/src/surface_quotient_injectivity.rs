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
#[derive(Clone, Debug)]
pub struct RuledReport {
    pub proven: bool,
    pub cells: usize,
    pub reason: &'static str,
    pub direction_denominator_lower: Option<f64>,
    pub angular_derivative_numerator: Option<[f64; 2]>,
}
/// A linear U chart with one constant row is a rational family of rays.
/// A positive linear direction functional and a strictly monotone projected
/// direction ratio prove distinct rays on the whole V interval. Along each
/// ray u/(w0(v)*(1-u)+w1(v)*u) is strictly increasing for positive weights.
/// Only the complete constant row is identified; no samples admit a chart.
pub fn certify_ruled_source_frame(
    s: &Surface,
    collapsed_end: usize,
    subdivisions: usize,
    max_cells: usize,
) -> Result<RuledReport> {
    s.validate()?;
    check(
        collapsed_end <= 1 && (1..=64).contains(&subdivisions) && (1..=100000).contains(&max_cells),
        "Choose bounded ruled pole work",
    )?;
    let mut out = RuledReport {
        proven: false,
        cells: 0,
        reason: "unsupported-ruled-chart",
        direction_denominator_lower: None,
        angular_derivative_numerator: None,
    };
    let q = s.degree_v;
    let clamped = |k: &[f64], d: usize| {
        k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|x| *x == k[d])
            && k[d + 1..].iter().all(|x| *x == k[d + 1])
    };
    if s.periodic_u
        || s.periodic_v
        || s.degree_u != 1
        || s.control_points.len() != 2
        || q == 0
        || q > 8
        || s.control_points[0].len() != q + 1
        || !clamped(&s.knots_u, 1)
        || !clamped(&s.knots_v, q)
    {
        return Ok(out);
    }
    let pole = &s.control_points[collapsed_end][0];
    if s.control_points[collapsed_end].iter().any(|p| p != pole) {
        out.reason = "boundary-not-collapsed";
        return Ok(out);
    }
    let row = 1 - collapsed_end;
    let difference = |p: &[f64]| -> Result<[I; 3]> {
        let mut a = [I::point(0.); 3];
        for k in 0..3 {
            if p[k] != pole[k] {
                a[k] = I::point(p[k]).sub(I::point(pole[k]))?;
            }
        }
        Ok(a)
    };
    let first = difference(&s.control_points[row][0])?;
    let last = difference(&s.control_points[row][q])?;
    let mut h = [I::point(0.); 3];
    let mut g = h;
    for k in 0..3 {
        h[k] = add(first[k], last[k])?;
        g[k] = sub(last[k], first[k])?;
    }
    let mut coordinates = [
        vec![vec![I::point(0.); q + 1]],
        vec![vec![I::point(0.); q + 1]],
    ];
    for j in 0..=q {
        let point = difference(&s.control_points[row][j])?;
        for (axis, projection) in [h, g].iter().enumerate() {
            let mut value = I::point(0.);
            for k in 0..3 {
                value = add(value, mul(projection[k], point[k])?)?;
            }
            coordinates[axis][0][j] = mul(value, I::point(s.weights[row][j]))?;
        }
    }
    let [h, g] = coordinates;
    let derivative = combine(
        &product(&derivative(&g, 1)?, &h)?,
        &product(&g, &derivative(&h, 1)?)?,
        -1.,
    )?;
    if subdivisions > max_cells {
        out.reason = "work-limit";
        return Ok(out);
    }
    let mut denominator = f64::INFINITY;
    let mut angular = [f64::INFINITY, f64::NEG_INFINITY];
    for j in 0..subdivisions {
        let domain = [
            [0., 1.],
            [
                j as f64 / subdivisions as f64,
                (j + 1) as f64 / subdivisions as f64,
            ],
        ];
        out.cells += 1;
        denominator = denominator.min(bound(&restrict(&h, domain)?).lo);
        let b = bound(&restrict(&derivative, domain)?);
        angular = [angular[0].min(b.lo), angular[1].max(b.hi)];
    }
    out.direction_denominator_lower = Some(denominator);
    out.angular_derivative_numerator = Some(angular);
    if denominator > 0. && (angular[0] > 0. || angular[1] < 0.) {
        out.proven = true;
        out.reason = "global-ruled-ray-quotient";
    } else {
        out.reason = "ray-direction-order-unproven";
    }
    Ok(out)
}
#[derive(Clone, Debug)]
pub struct PolarReport {
    pub proven: bool,
    pub cells: usize,
    pub reason: &'static str,
    pub derivative_bounds: Option<[f64; 4]>,
    pub dominance_margin_lower: Option<f64>,
}
/// Blow up only a complete constant U row. With S-P=u D/W, use
/// F=f.(S-P) and G=(g.D)/(h.D), with independent fixed functionals.
/// The exact common u factor is removed
/// from G before bounding derivatives. Global diagonal dominance proves the
/// extended (F,G) map injective on the rectangle; positive F_u and F(0,v)=0
/// prove that the original image is injective modulo that boundary alone.
pub fn certify_polar_source_frame(
    s: &Surface,
    end: usize,
    subdivisions: usize,
    max_cells: usize,
) -> Result<PolarReport> {
    s.validate()?;
    check(
        end <= 1 && (1..=64).contains(&subdivisions) && (1..=100000).contains(&max_cells),
        "Choose bounded polar chart work",
    )?;
    let mut out = PolarReport {
        proven: false,
        cells: 0,
        reason: "unsupported-polar-chart",
        derivative_bounds: None,
        dominance_margin_lower: None,
    };
    let (p, q) = (s.degree_u, s.degree_v);
    let clamped = |k: &[f64], d: usize| {
        k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|x| *x == k[d])
            && k[d + 1..].iter().all(|x| *x == k[d + 1])
    };
    if s.periodic_u
        || s.periodic_v
        || p < 2
        || p > 8
        || q == 0
        || q > 8
        || s.control_points.len() != p + 1
        || s.control_points[0].len() != q + 1
        || !clamped(&s.knots_u, p)
        || !clamped(&s.knots_v, q)
    {
        return Ok(out);
    }
    let index = |i| if end == 0 { i } else { p - i };
    let pole = &s.control_points[index(0)][0];
    if s.control_points[index(0)].iter().any(|x| x != pole) {
        out.reason = "boundary-not-collapsed";
        return Ok(out);
    }
    let delta = |point: &[f64]| -> Result<[I; 3]> {
        let mut a = [I::point(0.); 3];
        for k in 0..3 {
            if point[k] != pole[k] {
                a[k] = I::point(point[k]).sub(I::point(pole[k]))?;
            }
        }
        Ok(a)
    };
    let a = delta(&s.control_points[index(p)][0])?;
    let b = delta(&s.control_points[index(p)][q])?;
    let first = delta(&s.control_points[index(1)][0])?;
    let last = delta(&s.control_points[index(1)][q])?;
    let mut h = [I::point(0.); 3];
    let mut direction = h;
    let mut g = h;
    for k in 0..3 {
        h[k] = add(a[k], b[k])?;
        direction[k] = add(first[k], last[k])?;
        g[k] = sub(last[k], first[k])?;
    }
    let mut coordinates: [Poly; 4] =
        std::array::from_fn(|_| vec![vec![I::point(0.); q + 1]; p + 1]);
    for i in 0..=p {
        for j in 0..=q {
            let d = delta(&s.control_points[index(i)][j])?;
            let w = I::point(s.weights[index(i)][j]);
            coordinates[3][i][j] = w;
            for (axis, row) in [h, direction, g].iter().enumerate() {
                let mut value = I::point(0.);
                for k in 0..3 {
                    value = add(value, mul(row[k], d[k])?)?;
                }
                coordinates[axis][i][j] = mul(value, w)?;
            }
        }
    }
    let [f, direction, g, w] = coordinates;
    let Some(h) = factor_u(&direction, 1)? else {
        out.reason = "pole-factor-unproven";
        return Ok(out);
    };
    let Some(k) = factor_u(&g, 1)? else {
        out.reason = "pole-factor-unproven";
        return Ok(out);
    };
    let numerator = |n: &Poly, d: &Poly, axis| {
        combine(
            &product(&derivative(n, axis)?, d)?,
            &product(n, &derivative(d, axis)?)?,
            -1.,
        )
    };
    let fu = numerator(&f, &w, 0)?;
    let fv = numerator(&f, &w, 1)?;
    let gu = numerator(&k, &h, 0)?;
    let gv = numerator(&k, &h, 1)?;
    if subdivisions * subdivisions > max_cells {
        out.reason = "work-limit";
        return Ok(out);
    }
    let (mut a, mut b, mut e, mut c) = (f64::INFINITY, f64::INFINITY, 0_f64, 0_f64);
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
            let direction = bound(&restrict(&h, domain)?);
            if weight.lo <= 0. || direction.lo <= 0. {
                out.reason = "polar-denominator-unproven";
                return Ok(out);
            }
            let wd = weight.mul(weight)?;
            let hd = direction.mul(direction)?;
            a = a.min(bound(&restrict(&fu, domain)?).div(wd)?.lo);
            b = b.min(bound(&restrict(&gv, domain)?).div(hd)?.lo);
            let x = bound(&restrict(&fv, domain)?).div(wd)?;
            e = e.max(x.lo.abs().max(x.hi.abs()));
            let x = bound(&restrict(&gu, domain)?).div(hd)?;
            c = c.max(x.lo.abs().max(x.hi.abs()));
        }
    }
    let margin = I::point(a)
        .mul(I::point(b))?
        .sub(I::point(e).mul(I::point(c))?)?
        .lo;
    out.derivative_bounds = Some([a, b, e, c]);
    out.dominance_margin_lower = Some(margin);
    if a > 0. && b > 0. && margin > 0. {
        out.proven = true;
        out.reason = "global-polar-blowup-dominance";
    } else {
        out.reason = "polar-dominance-unproven";
    }
    Ok(out)
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
        || q == 0
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
#[derive(Clone, Debug)]
pub struct JoinedReport {
    pub proven: bool,
    pub reason: &'static str,
    pub cells: usize,
    pub weighted_bounds: Option<[f64; 4]>,
    pub dominance_margin_lower: Option<f64>,
}
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
    fn polar_surface() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![2., 2., 2., 5., 5., 5.],
            knots_v: vec![-4., -4., 8., 8.],
            control_points: vec![
                vec![vec![3., -7., 5.]; 2],
                vec![vec![3.5, -7., 5.], vec![3.5, -6.5, 5.]],
                vec![vec![4., -7., 5.], vec![4., -6., 5.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn polar_blowup_preserves_original_nonunit_charts_and_both_pole_ends() {
        let mut s = polar_surface();
        for end in [0, 1] {
            if end == 1 {
                s.control_points.reverse();
                s.weights.reverse();
            }
            let original = s.clone();
            let r = certify_polar_source_frame(&s, end, 8, 64).unwrap();
            assert!(r.proven, "{r:?}");
            assert_eq!(r.cells, 64);
            assert!(r.dominance_margin_lower.unwrap() > 0.);
            assert_eq!(s, original);
        }
    }
    #[test]
    fn polar_blowup_refuses_fold_repeated_direction_nonpole_and_work_limit() {
        let mut s = polar_surface();
        s.control_points[1][0][0] = 2.;
        s.control_points[1][1][0] = 2.;
        assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
        let mut s = polar_surface();
        for row in &mut s.control_points {
            row[1] = row[0].clone();
        }
        assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
        let mut s = polar_surface();
        s.control_points[0][1][0] += 1e-12;
        assert!(!certify_polar_source_frame(&s, 0, 8, 64).unwrap().proven);
        let r = certify_polar_source_frame(&polar_surface(), 0, 8, 63).unwrap();
        assert!(!r.proven && r.cells == 0 && r.derivative_bounds.is_none());
        assert!(certify_polar_source_frame(&polar_surface(), 2, 8, 64).is_err());
    }
    fn ruled_surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![2., 2., 5., 5.],
            knots_v: vec![-4., -4., -4., 8., 8., 8.],
            control_points: vec![
                vec![vec![3., -7., 5.]; 3],
                vec![vec![4., -7., 6.], vec![4., -6., 6.], vec![3., -6., 6.]],
            ],
            weights: vec![vec![1., 0.25, 2.], vec![0.3, 1., 0.75]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn ruled_ray_ratio_covers_nonunit_domains_and_independent_weight_rows() {
        let mut s = ruled_surface();
        for end in [0, 1] {
            if end == 1 {
                s.control_points.reverse();
                s.weights.reverse();
            }
            let original = s.clone();
            let r = certify_ruled_source_frame(&s, end, 16, 16).unwrap();
            assert!(r.proven, "{r:?}");
            assert_eq!(r.cells, 16);
            assert!(r.direction_denominator_lower.unwrap() > 0.);
            assert!(r.angular_derivative_numerator.unwrap()[0] > 0.);
            assert_eq!(s, original);
        }
    }
    #[test]
    fn ruled_fold_repeated_ray_and_nonconstant_pole_remain_unproven() {
        let mut folded = ruled_surface();
        folded.control_points[1][1] = vec![8., -12., 6.];
        assert!(
            !certify_ruled_source_frame(&folded, 0, 16, 16)
                .unwrap()
                .proven
        );
        let mut repeated = ruled_surface();
        repeated.control_points[1][2] = repeated.control_points[1][0].clone();
        assert!(
            !certify_ruled_source_frame(&repeated, 0, 16, 16)
                .unwrap()
                .proven
        );
        let mut broken = ruled_surface();
        broken.control_points[0][1][0] += 1e-12;
        let r = certify_ruled_source_frame(&broken, 0, 16, 16).unwrap();
        assert!(!r.proven && r.reason == "boundary-not-collapsed");
    }
    #[test]
    fn ruled_budget_exhaustion_has_no_direction_certificate() {
        let r = certify_ruled_source_frame(&ruled_surface(), 0, 16, 15).unwrap();
        assert!(!r.proven && r.cells == 0 && r.angular_derivative_numerator.is_none());
        assert!(certify_ruled_source_frame(&ruled_surface(), 0, 0, 16).is_err());
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
