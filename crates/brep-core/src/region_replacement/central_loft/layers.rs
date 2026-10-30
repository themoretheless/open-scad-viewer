//! Six direct Bernstein rows, not sampled graph interpolation.
use super::*;
fn length(p: &[f64], q: &[f64]) -> f64 {
    (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2])
}
fn composite(
    pieces: &[Curve],
    offsets: &[Vec<[f64; 3]>],
    gain: f64,
    work: &mut Work,
) -> Result<(Curve, [f64; 4])> {
    let degree = pieces[0].degree;
    let mut spans = [1.; 3];
    for i in 0..3 {
        let c = &pieces[i];
        c.validate()?;
        let d = c.domain();
        require(
            c.degree == degree
                && (5..=16).contains(&degree)
                && c.control_points.len() == degree + 1
                && !c.periodic
                && c.control_points[0].len() == 3
                && c.knots[..=degree].iter().all(|&t| t == d[0])
                && c.knots[degree + 1..].iter().all(|&t| t == d[1]),
            "Central loft requires complete aligned rational Bernstein rim pieces",
        )?;
        if i > 0 {
            require(
                pieces[i - 1].control_points.last() == c.control_points.first(),
                "Central half rim lost literal endpoints",
            )?;
            let p = &pieces[i - 1].control_points;
            let ratio = length(&c.control_points[1], &c.control_points[0]) * c.weights[1]
                / c.weights[0]
                / (length(&p[degree], &p[degree - 1]) * pieces[i - 1].weights[degree - 1]
                    / pieces[i - 1].weights[degree]);
            require(
                ratio.is_finite() && ratio > 0.,
                "Central half rim has no regular timing",
            )?;
            spans[i] = spans[i - 1] * ratio;
        }
    }
    let total = spans.iter().sum::<f64>();
    let breaks = [0., spans[0] / total, (spans[0] + spans[1]) / total, 1.];
    require(
        breaks.windows(2).all(|w| w[0].is_finite() && w[0] < w[1]),
        "Central timing is unresolved",
    )?;
    let mut points = Vec::new();
    let mut weights = Vec::new();
    let mut carry = None;
    for i in 0..3 {
        let factor = carry.map_or(1., |last: f64| last / pieces[i].weights[0]);
        require(
            factor.is_finite() && factor > 0.,
            "Central rational gauge is unresolved",
        )?;
        for j in 0..=degree {
            let p: Vec<_> = (0..3)
                .map(|k| pieces[i].control_points[j][k] + gain * offsets[i][j][k])
                .collect();
            work.charge(3)?;
            if i > 0 && j == 0 {
                require(
                    points.last() == Some(&p),
                    "Central layer junction is not source-owned",
                )?;
            } else {
                points.push(p);
                weights.push(pieces[i].weights[j] * factor);
            }
        }
        carry = weights.last().copied();
    }
    let mut knots = vec![0.; degree + 1];
    for t in [breaks[1], breaks[2]] {
        knots.extend(vec![t; degree]);
    }
    knots.extend(vec![1.; degree + 1]);
    let curve = Curve {
        degree,
        knots,
        weights,
        control_points: points,
        periodic: false,
    };
    curve.validate()?;
    Ok((curve, breaks))
}
pub(super) fn build(
    rim: &[Curve],
    directions: [[f64; 3]; 6],
    gain: f64,
    stock: [[f64; 2]; 3],
    boundary_margin: f64,
    work: &mut Work,
) -> Result<(Surface, Vec<Curve>, f64)> {
    let degree = rim.iter().map(|c| c.degree).max().unwrap().max(5);
    require(
        degree <= 16,
        "Central rim degree exceeds bounded composition",
    )?;
    let mut elevated = Vec::new();
    let mut correction: f64 = 0.;
    for c in rim {
        c.validate()?;
        require(
            !c.periodic && c.control_points.len() == c.degree + 1,
            "Central rim piece needs one complete Bernstein span",
        )?;
        let mut e = c.elevate(degree)?;
        e.control_points[0] = c.control_points[0].clone();
        e.control_points[degree] = c.control_points.last().unwrap().clone();
        for p in &mut e.control_points {
            for k in 0..3 {
                let q = p[k].clamp(stock[k][0], stock[k][1]);
                let change = (q - p[k]).abs();
                require(
                    change <= boundary_margin,
                    "Original rim control hull exceeds the bounded material correction",
                )?;
                correction = correction.max(change);
                p[k] = q;
                work.charge(1)?;
            }
        }
        elevated.push(e);
    }
    let mut fields = Vec::new();
    for i in 0..6 {
        let a = if i % 2 == 0 {
            directions[i]
        } else {
            directions[(i + 5) % 6]
        };
        let b = if i % 2 == 0 {
            directions[i]
        } else {
            directions[(i + 1) % 6]
        };
        let mut f: Vec<[f64; 3]> = (0..=degree)
            .map(|j| {
                let t = j as f64 / degree as f64;
                std::array::from_fn(|k| (1. - t) * a[k] + t * b[k])
            })
            .collect();
        f[..3].fill(a);
        f[degree - 2..].fill(b);
        if i == 0 || i == 3 {
            f[0] = [0.; 3];
            f[1] = a;
            f[2] = a.map(|x| 2. * x);
        }
        if i == 2 || i == 5 {
            f[degree - 2] = b.map(|x| 2. * x);
            f[degree - 1] = b;
            f[degree] = [0.; 3];
        }
        fields.push(f);
    }
    let first = elevated[..3].to_vec();
    let second: Vec<_> = (3..6)
        .rev()
        .map(|i| elevated[i].reverse())
        .collect::<Result<_>>()?;
    let other: Vec<_> = (3..6)
        .rev()
        .map(|i| {
            let mut f = fields[i].clone();
            f.reverse();
            f
        })
        .collect();
    let mut curves = Vec::new();
    let mut timings = Vec::new();
    for (pieces, field, g) in [
        (&first, &fields[..3], 0.),
        (&first, &fields[..3], gain),
        (&first, &fields[..3], 2. * gain),
        (&second, &other[..], 2. * gain),
        (&second, &other[..], gain),
        (&second, &other[..], 0.),
    ] {
        let (c, t) = composite(pieces, field, g, work)?;
        curves.push(c);
        timings.push(t);
    }
    let mut internal = vec![timings[0][1], timings[0][2], timings[5][1], timings[5][2]];
    internal.sort_by(f64::total_cmp);
    internal.dedup();
    for c in &mut curves {
        for &t in &internal {
            let n = c.knots.iter().filter(|&&k| k == t).count();
            if n < degree {
                *c = c.insert(t, degree - n)?;
            }
            work.charge(c.control_points.len())?;
            require(c.control_points.len() <= 256, "Central loft control limit")?;
        }
    }
    let n = curves[0].control_points.len();
    require(
        curves
            .iter()
            .all(|c| c.knots == curves[0].knots && c.control_points.len() == n),
        "Central layer bases differ",
    )?;
    let surface = Surface {
        degree_u: degree,
        degree_v: 5,
        knots_u: curves[0].knots.clone(),
        knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
        control_points: (0..n)
            .map(|i| curves.iter().map(|c| c.control_points[i].clone()).collect())
            .collect(),
        weights: (0..n)
            .map(|i| curves.iter().map(|c| c.weights[i]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let mut pcurves = Vec::new();
    for i in 0..6 {
        let (t, a, b, v) = if i < 3 {
            (&timings[0], i, i + 1, 0.)
        } else {
            (&timings[5], 6 - i, 5 - i, 1.)
        };
        let mut p = Curve::from_polyline(vec![vec![t[a], v], vec![t[b], v]])?;
        let d = rim[i].domain();
        p.knots = vec![d[0], d[0], d[1], d[1]];
        pcurves.push(p);
    }
    Ok((surface, pcurves, correction))
}
