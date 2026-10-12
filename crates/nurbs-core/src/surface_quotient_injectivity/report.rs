use super::*;

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
