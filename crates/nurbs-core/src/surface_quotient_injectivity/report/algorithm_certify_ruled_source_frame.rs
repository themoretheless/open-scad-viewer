//! report algorithm: certify_ruled_source_frame.
use super::*;
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
