//! report algorithm: certify_polar_source_frame.
use super::*;
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
