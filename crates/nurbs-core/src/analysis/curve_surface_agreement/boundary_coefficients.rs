//! Original Bernstein coefficient comparison at a literal clamped boundary.
//! Restriction clocks are enclosed affine preimages, never rounded roots.
use super::*;
type Net = Vec<[Interval; 4]>;
fn difference(a: f64, b: f64) -> Result<Interval> {
    if a == b {
        Ok(Interval::point(0.))
    } else {
        Interval::point(a).sub(Interval::point(b))
    }
}
fn mix(a: Interval, b: Interval, t: Interval) -> Result<Interval> {
    if t.lo == 0. && t.hi == 0. {
        return Ok(a);
    }
    if t.lo == 1. && t.hi == 1. {
        return Ok(b);
    }
    if a.lo == a.hi && a.lo == b.lo && b.lo == b.hi {
        return Ok(a);
    }
    a.mul(Interval::point(1.).sub(t)?)?.add(b.mul(t)?)
}
fn restrict(net: &Net, a: Interval, b: Interval) -> Result<Net> {
    let degree = net.len() - 1;
    let mut out = Vec::new();
    for k in 0..=degree {
        let mut row = net.clone();
        for stage in 0..degree {
            let t = if stage < degree - k { a } else { b };
            for i in 0..row.len() - 1 {
                for axis in 0..4 {
                    row[i][axis] = mix(row[i][axis], row[i + 1][axis], t)?;
                }
            }
            row.pop();
        }
        out.push(row[0]);
    }
    Ok(out)
}
fn elevate(net: &mut Net, degree: usize) -> Result<()> {
    while net.len() < degree + 1 {
        let q = net.len();
        let mut out = vec![net[0]];
        for i in 1..q {
            let a = Interval::point(i as f64).div(Interval::point(q as f64))?;
            let b = Interval::point((q - i) as f64).div(Interval::point(q as f64))?;
            let mut value = [Interval::point(0.); 4];
            for k in 0..4 {
                value[k] = if net[i - 1][k].lo == 0.
                    && net[i - 1][k].hi == 0.
                    && net[i][k].lo == 0.
                    && net[i][k].hi == 0.
                {
                    Interval::point(0.)
                } else {
                    net[i - 1][k].mul(a)?.add(net[i][k].mul(b)?)?
                };
            }
            out.push(value);
        }
        out.push(*net.last().unwrap());
        *net = out;
    }
    Ok(())
}
fn net(points: &[Vec<f64>], weights: &[f64], origin: [f64; 3]) -> Result<Net> {
    points
        .iter()
        .zip(weights)
        .map(|(p, &w)| {
            Ok([
                difference(p[0], origin[0])?.mul(Interval::point(w))?,
                difference(p[1], origin[1])?.mul(Interval::point(w))?,
                difference(p[2], origin[2])?.mul(Interval::point(w))?,
                Interval::point(w),
            ])
        })
        .collect()
}
fn relative_bound(a: &Net, b: &Net, diameter: f64) -> Result<f64> {
    let mut point_error = 0f64;
    let mut low = f64::INFINITY;
    let mut high = 0f64;
    for (a, b) in a.iter().zip(b) {
        let ratio = b[3].div(a[3])?;
        low = low.min(ratio.lo);
        high = high.max(ratio.hi);
        let aa = (0..3).map(|k| a[k].div(a[3])).collect::<Result<Vec<_>>>()?;
        let bb = (0..3).map(|k| b[k].div(b[3])).collect::<Result<Vec<_>>>()?;
        point_error = point_error.max(box_distance(&aa, &bb)?.1);
    }
    // With positive common Bernstein basis, normalizing weights whose ratios
    // lie in [low,high] moves the convex combination by at most
    // diameter*(high-low)/low. This avoids division by the minimum endpoint
    // weight. Both nets still come from the actual original definitions.
    let drift = Interval::point(high)
        .sub(Interval::point(low))?
        .div(Interval::point(low))?;
    Ok(Interval::point(point_error)
        .add(Interval::point(diameter).mul(drift)?)?
        .hi)
}
pub(super) fn inspect(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Option<Report>> {
    if c.periodic
        || c.control_points.len() != c.degree + 1
        || p.periodic
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || s.periodic_u
        || s.periodic_v
        || s.control_points[0][0].len() != 3
    {
        return Ok(None);
    }
    let cd = c.domain();
    if !c.knots[..=c.degree].iter().all(|&t| t == cd[0])
        || !c.knots[c.control_points.len()..]
            .iter()
            .all(|&t| t == cd[1])
    {
        return Ok(None);
    }
    let m = s.control_points[0].len();
    let vd = [s.knots_v[s.degree_v], s.knots_v[m]];
    let value = p.control_points[0][1];
    if p.control_points[1][1] != value {
        return Ok(None);
    }
    let row = if value == vd[0] && s.knots_v[..=s.degree_v].iter().all(|&v| v == value) {
        0
    } else if value == vd[1] && s.knots_v[m..].iter().all(|&v| v == value) {
        m - 1
    } else {
        return Ok(None);
    };
    let first = usize::from(reversed);
    let a = p.control_points[first][0];
    let b = p.control_points[1 - first][0];
    if a == b {
        return Ok(None);
    }
    let ends = [a.min(b), a.max(b)];
    let degree = s.degree_u;
    let n = s.control_points.len();
    if ends[0] < s.knots_u[degree] || ends[1] > s.knots_u[n] {
        return Ok(None);
    }
    let spans = (degree..n)
        .filter(|&i| {
            s.knots_u[i] < s.knots_u[i + 1] && s.knots_u[i + 1] > ends[0] && s.knots_u[i] < ends[1]
        })
        .collect::<Vec<_>>();
    if spans.is_empty()
        || spans.len() > max_cells
        || s.knots_u[spans[0]] > ends[0]
        || s.knots_u[spans.last().unwrap() + 1] < ends[1]
    {
        return Ok(None);
    }
    for &i in &spans {
        for t in [s.knots_u[i], s.knots_u[i + 1]] {
            let count = s.knots_u.iter().filter(|&&u| u == t).count();
            let outside = t == s.knots_u[degree] || t == s.knots_u[n];
            if count != degree + usize::from(outside) {
                return Ok(None);
            }
        }
    }
    let zero = [Interval::point(0.); 3];
    let widths = (0..3)
        .map(|k| {
            let low = c
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min);
            let high = c
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            difference(high, low)
        })
        .collect::<Result<Vec<_>>>()?;
    let diameter = box_distance(&widths, &zero)?.1;
    let clock = |u: f64| -> Result<Interval> {
        if u == a {
            return Ok(Interval::point(0.));
        }
        if u == b {
            return Ok(Interval::point(1.));
        }
        Interval::point(u)
            .sub(Interval::point(a))?
            .div(Interval::point(b).sub(Interval::point(a))?)?
            .intersect(0., 1.)
    };
    let mut report = Report {
        status: Status::WithinTolerance,
        cells: 0,
        witness: None,
        witness_distance: None,
    };
    let mut rational_work = rational_boundary_residual::Work::new(max_cells);
    for i in spans {
        report.cells += 1;
        let bound = (|| -> Result<f64> {
            let lo = s.knots_u[i].max(ends[0]);
            let hi = s.knots_u[i + 1].min(ends[1]);
            let left = clock(lo)?;
            let right = clock(hi)?;
            let center = ((left.lo + left.hi + right.lo + right.hi) * 0.25 * c.degree as f64)
                .round()
                .clamp(0., c.degree as f64) as usize;
            let origin = std::array::from_fn(|k| c.control_points[center][k]);
            let mut ca = restrict(&net(&c.control_points, &c.weights, origin)?, left, right)?;
            let points = (i - degree..=i)
                .map(|u| s.control_points[u][row].clone())
                .collect::<Vec<_>>();
            let weights = (i - degree..=i)
                .map(|u| s.weights[u][row])
                .collect::<Vec<_>>();
            let full = net(&points, &weights, origin)?;
            let local = |u: f64| -> Result<Interval> {
                if u == s.knots_u[i] {
                    return Ok(Interval::point(0.));
                }
                if u == s.knots_u[i + 1] {
                    return Ok(Interval::point(1.));
                }
                Interval::point(u)
                    .sub(Interval::point(s.knots_u[i]))?
                    .div(Interval::point(s.knots_u[i + 1]).sub(Interval::point(s.knots_u[i]))?)?
                    .intersect(0., 1.)
            };
            let mut cb = restrict(&full, local(lo)?, local(hi)?)?;
            let q = c.degree.max(degree);
            elevate(&mut ca, q)?;
            elevate(&mut cb, q)?;
            relative_bound(&ca, &cb, diameter)
        })();
        if !matches!(bound,Ok(upper) if upper<=tolerance)
            && rational_boundary_residual::within(
                c,
                s,
                row,
                i,
                a,
                b,
                s.knots_u[i].max(ends[0]),
                s.knots_u[i + 1].min(ends[1]),
                tolerance,
                &mut rational_work,
            ) != Some(true)
        {
            report.status = Status::Unresolved;
        }
    }
    Ok(Some(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(multiplicity: usize) -> (Curve, Curve, Surface) {
        let c = crate::construct::curves::bounded_corner::propose(0.1, 10., 2.5e-8)
            .unwrap()
            .quartic_g2()
            .unwrap();
        let mut boundary = c.elevate(5).unwrap().insert(0.37, multiplicity).unwrap();
        for t in &mut boundary.knots {
            *t = -2. + 5. * *t;
        }
        let s = Surface {
            degree_u: 5,
            degree_v: 1,
            knots_u: boundary.knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: boundary
                .control_points
                .iter()
                .map(|p| vec![p.clone(), vec![p[0], p[1], 1.]])
                .collect(),
            weights: boundary.weights.iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![-2., 0.], vec![3., 0.]]).unwrap();
        (c, p, s)
    }
    #[test]
    fn partial_original_cell_uses_outward_restriction_on_both_clocks() {
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let c = c.trim(0.17, 0.83).unwrap();
        let p = Curve::from_polyline(vec![vec![0.17, 0.], vec![0.83, 0.]]).unwrap();
        assert_eq!(
            inspect(&c, &p, &s, false, 1e-12, 1)
                .unwrap()
                .unwrap()
                .status,
            Status::WithinTolerance
        );
        let mut wrong = c.clone();
        wrong.control_points[0][1] = 0.01;
        assert_ne!(
            inspect(&wrong, &p, &s, false, 1e-12, 1)
                .unwrap()
                .unwrap()
                .status,
            Status::WithinTolerance
        );
    }
    #[test]
    fn degree_elevation_nondyadic_preimages_and_original_knot_cells_keep_full_coverage() {
        let (c, p, mut s) = fixture(5);
        let before = c.clone();
        for row in &mut s.weights {
            for w in row {
                *w *= 2f64.powi(20);
            }
        }
        let report = verify(&c, &p, &s, false, 1e-7, 2).unwrap();
        assert_eq!(report.status, Status::WithinTolerance);
        assert_eq!(report.cells, 2);
        assert!(inspect(&c, &p, &s, false, 1e-7, 1).unwrap().is_none());
        assert_eq!(c, before);
    }
    #[test]
    fn reversed_full_clocks_do_not_grant_partial_interval_or_wrong_orientation_admission() {
        let (c, p, s) = fixture(5);
        let reverse = p.reverse().unwrap();
        assert_eq!(
            verify(&c, &reverse, &s, true, 1e-7, 2).unwrap().status,
            Status::WithinTolerance
        );
        assert_ne!(
            verify(&c, &reverse, &s, false, 1e-7, 2).unwrap().status,
            Status::WithinTolerance
        );
        assert_ne!(
            verify_on(&c, &p, &s, false, [0., 0.5], 1e-7, 128)
                .unwrap()
                .status,
            Status::WithinTolerance
        );
    }
    #[test]
    fn altered_original_coefficients_or_unsupported_bases_cannot_inherit_a_bound() {
        let (c, mut p, mut s) = fixture(5);
        s.control_points[3][0][0] += 1e-3;
        let report = verify(&c, &p, &s, false, 1e-7, 2).unwrap();
        assert_eq!(report.status, Status::Unresolved);
        assert_eq!(report.cells, 2);
        p.control_points.iter_mut().for_each(|p| p[1] = 0.25);
        assert!(inspect(&c, &p, &s, false, 1e-7, 2).unwrap().is_none());
        let (c, p, s) = fixture(1);
        assert!(inspect(&c, &p, &s, false, 1e-7, 100).unwrap().is_none());
    }
}
