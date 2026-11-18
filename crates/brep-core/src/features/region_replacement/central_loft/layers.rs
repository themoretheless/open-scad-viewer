//! Six direct control rows; original multi-span rims are retained, never fitted.
use super::*;
#[path = "layers/weighted_field.rs"]
mod weighted_field;
#[derive(PartialEq)]
pub(super) struct AuthoredRows {
    pub base: [Curve;2],
    pub field: [Curve;2],
    pub gain: f64,
}

fn length(p: &[f64], q: &[f64]) -> f64 {
    (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2])
}
fn normalize_gauges(curves: &mut [Curve], work: &mut Work) -> Result<()> {
    let minimum = curves
        .iter()
        .flat_map(|c| &c.weights)
        .copied()
        .fold(f64::INFINITY, f64::min);
    let maximum = curves
        .iter()
        .flat_map(|c| &c.weights)
        .copied()
        .fold(0., f64::max);
    if maximum / minimum <= 1e12 {
        return Ok(());
    }
    // The old admissible proposals remain bit-for-bit unchanged. A new net
    // with incompatible row gauges needs a new relative weighting proposal.
    // Each individual row image is preserved by an EXACT binary scale, not
    // by independently rounded division of its weights by a floating maximum.
    for c in curves {
        let top = c.weights.iter().copied().fold(0., f64::max);
        let exponent = ((top.to_bits() >> 52) & 0x7ff) as i32 - 1023;
        let factor = 2f64.powi(-exponent);
        for w in &mut c.weights {
            *w *= factor;
            work.charge(1)?;
        }
        c.validate()?;
    }
    Ok(())
}
fn owned_anchors(c: &Curve) -> Result<Vec<(f64, Vec<f64>)>> {
    let p = c.degree;
    let n = c.control_points.len();
    let d = c.domain();
    require(
        !c.periodic
            && c.knots[..=p].iter().all(|&t| t == d[0])
            && c.knots[n..].iter().all(|&t| t == d[1]),
        "Owned central anchors need clamped curves",
    )?;
    let mut anchors = vec![(d[0], c.control_points[0].clone())];
    for span in p..n {
        let t = c.knots[span + 1];
        if c.knots[span] < t && t < d[1] && c.knots.iter().filter(|&&u| u == t).count() == p {
            anchors.push((t, c.control_points[span].clone()));
        }
    }
    anchors.push((d[1], c.control_points[n - 1].clone()));
    Ok(anchors)
}
fn restore_anchors(c: &mut Curve, anchors: &[(f64, Vec<f64>)], work: &mut Work) -> Result<()> {
    let d = c.domain();
    for (t, point) in anchors {
        let index = if *t == d[0] {
            0
        } else {
            (c.degree..c.control_points.len())
                .find(|&i| c.knots[i] < *t && c.knots[i + 1] == *t)
                .ok_or_else(|| refuse("An owned central knot disappeared"))?
        };
        work.charge(3)?;
        c.control_points[index] = point.clone();
    }
    Ok(())
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
                && c.control_points.len() >= degree + 1
                && c.control_points.len() <= 256
                && !c.periodic
                && c.control_points[0].len() == 3
                && c.knots[..=degree].iter().all(|&t| t == d[0])
                && c.knots[c.control_points.len()..].iter().all(|&t| t == d[1]),
            "Central loft requires aligned clamped rational rim pieces",
        )?;
        if i > 0 {
            require(
                pieces[i - 1].control_points.last() == c.control_points.first(),
                "Central half rim lost literal endpoints",
            )?;
            let p = &pieces[i - 1].control_points;
            let last = p.len() - 1;
            let mut ratio = length(&c.control_points[1], &c.control_points[0]) * c.weights[1]
                / c.weights[0]
                / (length(&p[last], &p[last - 1]) * pieces[i - 1].weights[last - 1]
                    / pieces[i - 1].weights[last]);
            if c.control_points.len() != degree + 1 || p.len() != degree + 1 {
                let prev = &pieces[i - 1];
                let pd = prev.domain();
                ratio *= (d[1] - d[0]) / (c.knots[degree + 1] - d[0]);
                ratio /= (pd[1] - pd[0]) / (pd[1] - prev.knots[p.len() - 1]);
            }
            require(
                ratio.is_finite() && ratio > 0.,
                "Central half rim has no regular timing",
            )?;
            spans[i] = spans[i - 1] * ratio;
        }
    }
    if pieces.iter().any(|c| c.control_points.len() != degree + 1)
        && spans.iter().any(|s| s / spans.iter().sum::<f64>() < 1e-6)
    {
        // Micro-runout endpoint speeds can make a whole rational rim occupy
        // 1e-9 of the shared chart. Equal piece clocks avoid that cancellation.
        // Matched endpoint clocks no longer need this fallback. This is a NEW
        // surface proposal, not an image identity: all six
        // boundary, G2, curvature and embedding gates remain mandatory.
        spans = [1.; 3];
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
        for j in 0..pieces[i].control_points.len() {
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
    for i in 0..3 {
        let c = &pieces[i];
        let d = c.domain();
        for &t in &c.knots[degree + 1..c.control_points.len()] {
            let mapped = breaks[i] + (breaks[i + 1] - breaks[i]) * ((t - d[0]) / (d[1] - d[0]));
            require(
                mapped > breaks[i] && mapped < breaks[i + 1],
                "Internal rim clock collapsed",
            )?;
            knots.push(mapped);
        }
        if i < 2 {
            knots.extend(vec![breaks[i + 1]; degree]);
        }
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
) -> Result<(Surface, Vec<Curve>, f64, AuthoredRows)> {
    let mut degree = rim.iter().map(|c| c.degree).max().unwrap().max(5);
    require(
        degree <= 16,
        "Central rim degree exceeds bounded composition",
    )?;
    let mut elevated = Vec::new();
    let mut correction: f64 = 0.;
    for c in rim {
        c.validate()?;
        require(
            !c.periodic
                && c.control_points.len() >= c.degree + 1
                && c.control_points.len() <= 256
                && c.knots[..=c.degree].iter().all(|&t| t == c.domain()[0])
                && c.knots[c.control_points.len()..]
                    .iter()
                    .all(|&t| t == c.domain()[1]),
            "Central rim needs a bounded clamped original curve",
        )?;
        let mut e = c.elevate(degree)?;
        e.control_points[0] = c.control_points[0].clone();
        let last = e.control_points.len() - 1;
        e.control_points[last] = c.control_points.last().unwrap().clone();
        // Exact source constants remain exact under degree elevation. Keep
        // their literal values rather than re-dividing a small rational gauge.
        for k in 0..3 {
            let value = c.control_points[0][k];
            if c.control_points.iter().all(|p| p[k] == value) {
                for point in &mut e.control_points {
                    point[k] = value;
                }
            }
        }
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
        let count = elevated[i].control_points.len();
        let mut f: Vec<[f64; 3]> = (0..count)
            .map(|j| {
                let t = if count == degree + 1 {
                    j as f64 / degree as f64
                } else {
                    let c = &elevated[i];
                    let d = c.domain();
                    let greville =
                        c.knots[j + 1..j + degree + 1].iter().sum::<f64>() / degree as f64;
                    (greville - d[0]) / (d[1] - d[0])
                };
                std::array::from_fn(|k| (1. - t) * a[k] + t * b[k])
            })
            .collect();
        f[..3].fill(a);
        f[count - 3..].fill(b);
        if i == 0 || i == 3 {
            f[0] = [0.; 3];
            f[1] = a;
            f[2] = a.map(|x| 2. * x);
        }
        if i == 2 || i == 5 {
            f[count - 3] = b.map(|x| 2. * x);
            f[count - 2] = b;
            f[count - 1] = [0.; 3];
        }
        fields.push(f);
    }
    let micro_clocks = elevated.iter().any(|c| {
        let d = c.domain();
        c.control_points.len() > degree + 1
            && ((c.knots[degree + 1] - d[0]) / (d[1] - d[0]) < 1e-6
                || (d[1] - c.knots[c.control_points.len() - 1]) / (d[1] - d[0]) < 1e-6)
    });
    if micro_clocks {
        // A pole field must extend over the WHOLE rim piece, not its first
        // microscopic knot cell. Multiply the polynomial displacement by the
        // rational denominator before degree elevation; Greville samples do
        // not preserve the internal two-jets of a rational field.
        let mut weighted = Vec::new();
        let mut smooth_fields = Vec::new();
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
            let mut f = [a, a, a, b, b, b];
            if i == 0 || i == 3 {
                f[0] = [0.; 3];
                f[1] = a;
                f[2] = a.map(|x| 2. * x);
            }
            if i == 2 || i == 5 {
                f[3] = b.map(|x| 2. * x);
                f[4] = b;
                f[5] = [0.; 3];
            }
            let (c, field) = weighted_field::prepare(&elevated[i], f, work)?;
            weighted.push(c);
            smooth_fields.push(field);
        }
        elevated = weighted;
        fields = smooth_fields;
        degree += 5;
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
    // Keep base and displacement separately BEFORE rounding world-space sums.
    // They are proposal authorship, never inherited G2/source certificates.
    let mut components=vec![curves[0].clone(),curves[5].clone()];
    for (base, fields) in [(&curves[0], &fields[..3]),(&curves[5], &other[..])] {
        let points=fields.iter().enumerate().flat_map(|(i,f)|
            f.iter().skip(usize::from(i>0)).map(|p|p.to_vec())).collect::<Vec<_>>();
        require(points.len()==base.control_points.len(),"Authored displacement basis differs")?;
        let mut field=base.clone();field.control_points=points;field.validate()?;
        components.push(field);
    }
    normalize_gauges(&mut curves, work)?;
    let mut internal = vec![timings[0][1], timings[0][2], timings[5][1], timings[5][2]];
    for c in &curves {
        internal.extend(c.knots.iter().copied().filter(|&t| t > 0. && t < 1.));
    }
    internal.sort_by(f64::total_cmp);
    internal.dedup();
    for c in &mut curves {
        let anchors = owned_anchors(c)?;
        for &t in &internal {
            let n = c.knots.iter().filter(|&&k| k == t).count();
            if n < degree {
                *c = c.insert(t, degree - n)?;
            }
            work.charge(c.control_points.len())?;
            require(c.control_points.len() <= 256, "Central loft control limit")?;
        }
        // Knot insertion should retain these original Cartesian values. A
        // rounded homogeneous conversion must not split a shared vertex or
        // a collapsed pole. The actual restored proposal is reverified below.
        restore_anchors(c, &anchors, work)?;
    }
    for c in &mut components {
        let anchors=owned_anchors(c)?;
        for &t in &internal {
            let n=c.knots.iter().filter(|&&k|k==t).count();
            if n<degree {*c=c.insert(t,degree-n)?;}
            work.charge(c.control_points.len())?;
            require(c.control_points.len()<=256,"Authored central field control limit")?;
        }
        restore_anchors(c,&anchors,work)?;
    }
    let authored=AuthoredRows {
        base:[components[0].clone(),components[1].clone()],
        field:[components[2].clone(),components[3].clone()],gain,
    };
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
    Ok((surface, pcurves, correction, authored))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn union_insertion_retains_literal_endpoints_and_owned_c0_vertices() {
        let source = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![0.1, 0.45, 0.],
                vec![0.2, 0.3, 0.],
                vec![0.3, 0.2, 0.],
                vec![0.4, 0.1, 0.],
                vec![0.5, 0., 0.],
            ],
            weights: vec![1e-9, 0.2, 1e-9, 0.2, 1e-9],
            periodic: false,
        };
        let anchors = owned_anchors(&source).unwrap();
        assert_eq!(anchors.len(), 3);
        let mut c = source.insert(0.25, 2).unwrap().insert(0.75, 2).unwrap();
        let weights = c.weights.clone();
        restore_anchors(&mut c, &anchors, &mut Work { used: 0, max: 100 }).unwrap();
        assert_eq!(
            owned_anchors(&c)
                .unwrap()
                .into_iter()
                .filter(|(t, _)| [0., 0.5, 1.].contains(t))
                .collect::<Vec<_>>(),
            anchors
        );
        assert_eq!(c.weights, weights);
        c.validate().unwrap();
    }
    #[test]
    fn only_incompatible_row_gauges_are_rescaled_by_exact_binary_factors() {
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.; 3], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            weights: vec![1e-9, 0.25, 1e-9],
            periodic: false,
        };
        let mut rows = vec![c.clone(), c.clone()];
        for w in &mut rows[1].weights {
            *w *= 2f64.powi(30);
        }
        let before = rows.clone();
        normalize_gauges(&mut rows, &mut Work { used: 0, max: 1000 }).unwrap();
        let low = rows
            .iter()
            .flat_map(|c| &c.weights)
            .copied()
            .fold(f64::INFINITY, f64::min);
        let high = rows
            .iter()
            .flat_map(|c| &c.weights)
            .copied()
            .fold(0., f64::max);
        assert!(high / low < 1e12);
        for (new, old) in rows.iter().zip(&before) {
            assert_eq!(new.control_points, old.control_points);
            let ratio = new.weights[0] / old.weights[0];
            assert!(
                new.weights
                    .iter()
                    .zip(&old.weights)
                    .all(|(a, b)| *a == b * ratio)
            );
            for t in [0., 0.25, 0.5, 0.75, 1.] {
                let a = new.evaluate(t).unwrap();
                let b = old.evaluate(t).unwrap();
                assert!(
                    a.point
                        .iter()
                        .zip(b.point)
                        .all(|(x, y)| (*x - y).abs() < 1e-14)
                );
            }
        }
        let mut unchanged = vec![c.clone()];
        normalize_gauges(&mut unchanged, &mut Work { used: 0, max: 0 }).unwrap();
        assert_eq!(unchanged, vec![c]);
    }
    #[test]
    fn multispan_rims_remain_complete_owned_boundaries_after_composition() {
        let points = [
            [0., 0., 1.],
            [1., 0., 1.],
            [2., 1., 1.],
            [2., 2., 1.],
            [1., 2., 1.],
            [0., 1., 1.],
        ];
        let rim = (0..6)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % 6];
                Curve::from_polyline(vec![
                    a.to_vec(),
                    (0..3).map(|k| (a[k] + b[k]) / 2.).collect(),
                    b.to_vec(),
                ])
                .unwrap()
            })
            .collect::<Vec<_>>();
        let before = rim.clone();
        let mut work = Work {
            used: 0,
            max: 1000000,
        };
        let (s, pcurves, correction, _authored) = build(
            &rim,
            [[0.; 3]; 6],
            0.01,
            [[0., 2.], [0., 2.], [0., 3.]],
            1e-8,
            &mut work,
        )
        .unwrap();
        assert!(s.control_points.len() > 16);
        assert_eq!(correction, 0.);
        assert_eq!(_authored.base[0].knots, s.knots_u);
        for i in 0..6 {
            let r = agreement::verify(&rim[i], &pcurves[i], &s, false, 1e-8, 10000).unwrap();
            assert_eq!(r.status, agreement::Status::WithinTolerance, "rim {i}");
        }
        assert_eq!(rim, before);
        let mut empty = Work { used: 0, max: 0 };
        assert!(
            build(
                &rim,
                [[0.; 3]; 6],
                0.01,
                [[0., 2.], [0., 2.], [0., 3.]],
                1e-8,
                &mut empty
            )
            .is_err()
        );
    }
}
