//! Source-owned ray projections with exact dual-frame fallback.
use super::*;
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
    for projection in 0..3 {
        let coordinates = if projection != 0 {
            let Some(c) = dual_coordinates(s, collapsed_end, projection == 2)? else {
                continue;
            };
            c
        } else {
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
            coordinates
        };
        let [h, g] = coordinates;
        let derivative = combine(
            &product(&derivative(&g, 1)?, &h)?,
            &product(&g, &derivative(&h, 1)?)?,
            -1.,
        )?;
        if subdivisions > max_cells - out.cells {
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
            out.reason = match projection {
                0 => "global-ruled-ray-quotient",
                1 => "global-ruled-dual-ray-quotient",
                _ => "global-ruled-three-ray-quotient",
            };
            return Ok(out);
        } else {
            out.reason = "ray-direction-order-unproven";
        }
    }
    Ok(out)
}

// Exact dual rows alpha(A)=1,alpha(B)=0; beta(A)=0,beta(B)=1.
// H=alpha+beta and G=beta-alpha are fixed linear functionals on the
// ORIGINAL ray numerators. No transformed/rounded Surface is substituted.
fn dual_coordinates(s: &Surface, end: usize, three: bool) -> Result<Option<[Poly; 2]>> {
    use num_rational::BigRational as Q;
    use num_traits::{ToPrimitive, Zero};
    let q = |v| Q::from_float(v).unwrap();
    let n = s.control_points[0].len();
    let origin = &s.control_points[end][0];
    let delta = |j: usize| {
        std::array::from_fn::<_, 3, _>(|k| q(s.control_points[1 - end][j][k]) - q(origin[k]))
    };
    let a = delta(0);
    let b = delta(n - 1);
    let dot = |x: &[Q; 3], y: &[Q; 3]| (0..3).map(|k| &x[k] * &y[k]).sum::<Q>();
    let rows = if three {
        // A third independent literal ray supplies dual coordinates invariant
        // under invertible affine changes of the entire source cone. Only
        // these fixed functionals are proposed; whole polynomial bounds below
        // still decide positivity and monotone direction for every original ray.
        let cross = |x: &[Q; 3], y: &[Q; 3]| {
            std::array::from_fn::<_, 3, _>(|k| {
                &x[(k + 1) % 3] * &y[(k + 2) % 3] - &x[(k + 2) % 3] * &y[(k + 1) % 3]
            })
        };
        let Some((middle, determinant)) = (1..n - 1).find_map(|j| {
            let middle = delta(j);
            let determinant = dot(&a, &cross(&middle, &b));
            (!determinant.is_zero()).then_some((middle, determinant))
        }) else {
            return Ok(None);
        };
        let alpha = cross(&middle, &b);
        let beta = cross(&b, &a);
        let gamma = cross(&a, &middle);
        [
            std::array::from_fn::<_, 3, _>(|k| (&alpha[k] + &beta[k] + &gamma[k]) / &determinant),
            std::array::from_fn::<_, 3, _>(|k| (&gamma[k] - &alpha[k]) / &determinant),
        ]
    } else {
        let aa = dot(&a, &a);
        let ab = dot(&a, &b);
        let bb = dot(&b, &b);
        let det = &aa * &bb - &ab * &ab;
        if det <= Q::zero() {
            return Ok(None);
        }
        let alpha = std::array::from_fn::<_, 3, _>(|k| (&bb * &a[k] - &ab * &b[k]) / &det);
        let beta = std::array::from_fn::<_, 3, _>(|k| (&aa * &b[k] - &ab * &a[k]) / &det);
        [
            std::array::from_fn::<_, 3, _>(|k| &alpha[k] + &beta[k]),
            std::array::from_fn::<_, 3, _>(|k| &beta[k] - &alpha[k]),
        ]
    };
    let mut coordinates = std::array::from_fn(|_| vec![vec![I::point(0.); n]]);
    for j in 0..n {
        let d = delta(j);
        for axis in 0..2 {
            let exact = dot(&rows[axis], &d) * q(s.weights[1 - end][j]);
            let Some(v) = exact.to_f64().filter(|v| v.is_finite()) else {
                return Ok(None);
            };
            let represented = q(v);
            let lo = if represented > exact {
                v.next_down()
            } else {
                v
            };
            let hi = if represented < exact { v.next_up() } else { v };
            if !lo.is_finite() || !hi.is_finite() {
                return Ok(None);
            }
            coordinates[axis][0][j] = I { lo, hi };
        }
    }
    Ok(Some(coordinates))
}
