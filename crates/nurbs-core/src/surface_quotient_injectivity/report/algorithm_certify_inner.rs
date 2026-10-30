//! report algorithm: certify_inner.
use super::*;
pub fn certify_inner(
    s: &Surface,
    collapsed_end: usize,
    projection: Projection,
    subdivisions: usize,
    max_cells: usize,
    profile:Option<&crate::normal_alignment::WeightProfile<'_>>,
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
            let Some(coefficients)=source_coefficients(frame)? else {report.reason="frame-not-separated";return Ok(report);};
            coefficients
        }
    };
    let mut coordinates: [Poly; 3] =
        std::array::from_fn(|_| vec![vec![I::point(0.); q + 1]; p + 1]);
    for i in 0..=p {
        for j in 0..=q {
            let point = &s.control_points[index(i)][j];
            let weight = if let Some(profile)=profile {
                let (row,column,pivot)=profile.factors();
                I::point(row[index(i)]).mul(I::point(column[j]))?.div(I::point(pivot))?
            }else{I::point(s.weights[index(i)][j])};
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
