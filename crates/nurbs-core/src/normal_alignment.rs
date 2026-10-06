//! Complete-rectangle normal/line angle bounds, including both sides of knots.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};

#[derive(Debug)]
pub struct Report {
    pub aligned: Option<bool>,
    pub sine_squared_interval: Option<[f64; 2]>,
    pub normal_components: Option<[[f64; 2]; 3]>,
    pub spans: usize,
    pub reason: &'static str,
}
fn square(x: I) -> Result<I> {
    let lo = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        0.
    };
    let hi = x.lo.abs().max(x.hi.abs());
    I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())
}
fn norm_squared(x: [I; 3]) -> Result<I> {
    let mut out = I::point(0.);
    for v in x {
        out = out.add(square(v)?)?;
    }
    I::new(out.lo.max(0.), out.hi)
}
type Net = Vec<Vec<[I; 4]>>;
fn restrict_hull(
    net: &Net,
    degrees: [usize; 2],
    span: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
) -> Result<[I; 4]> {
    let [p, q] = degrees;
    let ku = [vec![span[0][0]; p + 1], vec![span[0][1]; p + 1]].concat();
    let kv = [vec![span[1][0]; q + 1], vec![span[1][1]; q + 1]].concat();
    let weights = [
        net.iter()
            .flatten()
            .map(|h| h[3].lo)
            .fold(f64::INFINITY, f64::min),
        net.iter()
            .flatten()
            .map(|h| h[3].hi)
            .fold(f64::NEG_INFINITY, f64::max),
    ];
    let mut hull = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 4];
    for v in 0..=q {
        let vs = (0..q)
            .map(|k| domain[1][usize::from(k >= q - v)])
            .collect::<Vec<_>>();
        let rows = net
            .iter()
            .map(|row| crate::surface_distance::interpolate(row.clone(), q, &kv, q, &vs, weights))
            .collect::<Result<Vec<_>>>()?;
        for u in 0..=p {
            let us = (0..p)
                .map(|k| domain[0][usize::from(k >= p - u)])
                .collect::<Vec<_>>();
            let h = crate::surface_distance::interpolate(rows.clone(), p, &ku, p, &us, weights)?;
            for k in 0..4 {
                hull[k].lo = hull[k].lo.min(h[k].lo);
                hull[k].hi = hull[k].hi.max(h[k].hi);
            }
        }
    }
    Ok(hull)
}
pub(crate) fn jacobian_on(s: &Surface, indices: [usize; 2], domain: [[f64; 2]; 2]) -> Result<[[I; 2]; 3]> {
    let [u, v] = indices;
    let (p, q) = (s.degree_u, s.degree_v);
    let span = [
        [s.knots_u[u], s.knots_u[u + 1]],
        [s.knots_v[v], s.knots_v[v + 1]],
    ];
    let mut net = crate::curve_surface_composition::surface_net_on(s, indices, span)?;
    let origin = &s.control_points[u - p][v - q];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let values = restrict_hull(&net, [p, q], span, domain)?;
    let denominator = values[3].mul(values[3])?;
    let mut result = [[I::point(0.); 2]; 3];
    for axis in 0..2 {
        let degree = [p, q][axis];
        if degree == 0 {
            continue;
        }
        let derivatives = differentiate_net(&net, [p, q], axis, span)?;
        let d = restrict_hull(
            &derivatives,
            [p - usize::from(axis == 0), q - usize::from(axis == 1)],
            span,
            domain,
        )?;
        for k in 0..3 {
            result[k][axis] = d[k]
                .mul(values[3])?
                .sub(values[k].mul(d[3])?)?
                .div(denominator)?;
        }
    }
    Ok(result)
}
#[derive(Clone, Copy)]
pub(crate) struct JetBounds {
    pub point: [I; 3],
    pub first: [[I; 3]; 2],
    pub second: [[I; 3]; 3],
}
fn differentiate_net(
    net: &Net,
    degrees: [usize; 2],
    axis: usize,
    span: [[f64; 2]; 2],
) -> Result<Net> {
    let degree = degrees[axis];
    let mut out = vec![
        vec![[I::point(0.); 4]; degrees[1] + 1 - usize::from(axis == 1)];
        degrees[0] + 1 - usize::from(axis == 0)
    ];
    let width = I::point(span[axis][1]).sub(I::point(span[axis][0]))?;
    for i in 0..out.len() {
        for j in 0..out[0].len() {
            for k in 0..4 {
                out[i][j][k] = net[i + usize::from(axis == 0)][j + usize::from(axis == 1)][k]
                    .sub(net[i][j][k])?
                    .mul(I::point(degree as f64))?
                    .div(width)?;
            }
        }
    }
    Ok(out)
}
fn homogeneous_jet_hull(
    net: &Net,
    degrees: [usize; 2],
    span: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
    order: [usize; 2],
) -> Result<[I; 4]> {
    if (0..2).any(|axis| order[axis] > degrees[axis]) {
        return Ok([I::point(0.); 4]);
    }
    let mut derivative = net.clone();
    let mut degree = degrees;
    for axis in 0..2 {
        for _ in 0..order[axis] {
            derivative = differentiate_net(&derivative, degree, axis, span)?;
            degree[axis] -= 1;
        }
    }
    restrict_hull(&derivative, degree, span, domain)
}
fn jet_on(s: &Surface, indices: [usize; 2], domain: [[f64; 2]; 2]) -> Result<JetBounds> {
    let [u, v] = indices;
    let degrees = [s.degree_u, s.degree_v];
    let span = [
        [s.knots_u[u], s.knots_u[u + 1]],
        [s.knots_v[v], s.knots_v[v + 1]],
    ];
    let mut net = crate::curve_surface_composition::surface_net_on(s, indices, span)?;
    let origin = &s.control_points[u - degrees[0]][v - degrees[1]];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut h = [[I::point(0.); 4]; 6];
    for (i, order) in [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]]
        .into_iter()
        .enumerate()
    {
        h[i] = homogeneous_jet_hull(&net, degrees, span, domain, order)?;
    }
    let mut out = JetBounds {
        point: [I::point(0.); 3],
        first: [[I::point(0.); 3]; 2],
        second: [[I::point(0.); 3]; 3],
    };
    for k in 0..3 {
        let relative = h[0][k].div(h[0][3])?;
        out.point[k] = relative.add(I::point(origin[k]))?;
        for axis in 0..2 {
            out.first[axis][k] = h[axis + 1][k]
                .sub(relative.mul(h[axis + 1][3])?)?
                .div(h[0][3])?;
        }
        for (slot, a, b) in [(0, 0, 0), (1, 0, 1), (2, 1, 1)] {
            out.second[slot][k] = h[slot + 3][k]
                .sub(out.first[a][k].mul(h[b + 1][3])?)?
                .sub(out.first[b][k].mul(h[a + 1][3])?)?
                .sub(relative.mul(h[slot + 3][3])?)?
                .div(h[0][3])?;
        }
    }
    Ok(out)
}
/// All incident span-side jets over the rectangle. Their union does not prove
/// source continuity across repeated knots; root inclusion must check that separately.
pub(crate) fn jet_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<JetBounds>, usize)> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)?;
    let mut out: Option<JetBounds> = None;
    let mut spans = 0;
    let union = |a: &mut I, b: I| {
        a.lo = a.lo.min(b.lo);
        a.hi = a.hi.max(b.hi);
    };
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let mut section = [[0.; 2]; 2];
            let indices = [u, v];
            let knots = [&s.knots_u, &s.knots_v];
            let mut outside = false;
            for axis in 0..2 {
                let i = indices[axis];
                let lo = knots[axis][i];
                let hi = knots[axis][i + 1];
                if lo == hi || domain[axis][1] < lo || domain[axis][0] > hi {
                    outside = true;
                    break;
                }
                section[axis] = [lo.max(domain[axis][0]), hi.min(domain[axis][1])];
            }
            if outside {
                continue;
            }
            if spans == max_spans {
                return Ok((None, spans));
            }
            spans += 1;
            let next = jet_on(s, indices, section)?;
            if let Some(current) = &mut out {
                for k in 0..3 {
                    union(&mut current.point[k], next.point[k]);
                    for axis in 0..2 {
                        union(&mut current.first[axis][k], next.first[axis][k]);
                    }
                    for slot in 0..3 {
                        union(&mut current.second[slot][k], next.second[slot][k]);
                    }
                }
            } else {
                out = Some(next);
            }
        }
    }
    check(spans > 0, "Jet rectangle has no nonempty knot spans")?;
    Ok((out, spans))
}
fn validate_input(
    s: &Surface,
    domain: [[f64; 2]; 2],
    direction: [f64; 3],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<()> {
    s.validate()?;
    check(
        direction.iter().all(|x| x.is_finite())
            && direction.iter().any(|x| *x != 0.)
            && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && (1..=100000).contains(&max_spans),
        "Normal alignment requires a finite nonzero direction, squared-sine tolerance in [0,1) and bounded positive spans",
    )?;
    let knots = [&s.knots_u, &s.knots_v];
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    for axis in 0..2 {
        check(
            domain[axis].iter().all(|x| x.is_finite())
                && domain[axis][0] <= domain[axis][1]
                && domain[axis][0] >= knots[axis][degrees[axis]]
                && domain[axis][1] <= knots[axis][counts[axis]],
            "Normal rectangle must lie inside the natural surface domain",
        )?;
    }
    Ok(())
}
/// Encloses unnormalized source normals over every intersecting knot span.
/// None means the span budget did not cover the requested rectangle.
pub(crate) fn validate_rectangle(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<()> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)
}
pub fn normal_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)?;
    normal_bounds_validated(s, domain, max_spans)
}
fn normal_bounds_validated(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let mut spans = 0;
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            let indices = [u, v];
            let mut section = [[0.; 2]; 2];
            let mut outside = false;
            for axis in 0..2 {
                let i = indices[axis];
                let lo = knots[axis][i];
                let hi = knots[axis][i + 1];
                if lo == hi || domain[axis][1] < lo || domain[axis][0] > hi {
                    outside = true;
                    break;
                }
                section[axis] = [lo.max(domain[axis][0]), hi.min(domain[axis][1])];
            }
            if outside {
                continue;
            }
            if spans == max_spans {
                return Ok((None, spans));
            }
            spans += 1;
            // Differentiate the original span first, then restrict the derivative
            // polynomial. Narrow/point rectangles never divide by their own width.
            let j = jacobian_on(s, indices, section)?;
            for k in 0..3 {
                let a = (k + 1) % 3;
                let b = (k + 2) % 3;
                let n = j[a][0].mul(j[b][1])?.sub(j[b][0].mul(j[a][1])?)?;
                hull[k][0] = hull[k][0].min(n.lo);
                hull[k][1] = hull[k][1].max(n.hi);
            }
        }
    }
    check(spans > 0, "Normal rectangle has no nonempty knot spans")?;
    Ok((Some(hull), spans))
}
pub fn inspect(
    s: &Surface,
    domain: [[f64; 2]; 2],
    direction: [f64; 3],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report> {
    validate_input(s, domain, direction, max_sine_squared, max_spans)?;
    let (components, spans) = normal_bounds_validated(s, domain, max_spans)?;
    let mut out = Report {
        aligned: None,
        sine_squared_interval: None,
        normal_components: components,
        spans,
        reason: "span-limit",
    };
    let Some(hull) = components else {
        return Ok(out);
    };
    if hull.iter().all(|r| r[0] <= 0. && r[1] >= 0.) {
        out.reason = "normal-unresolved";
        return Ok(out);
    }
    let scale = hull.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
    if scale == 0. {
        out.reason = "normal-unresolved";
        return Ok(out);
    }
    let mut normal = [I::point(0.); 3];
    let mut d = [I::point(0.); 3];
    let ds = direction.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    for k in 0..3 {
        normal[k] = I::new(hull[k][0], hull[k][1])?.div(I::point(scale))?;
        d[k] = I::point(direction[k]).div(I::point(ds))?;
    }
    let denominator = norm_squared(normal)?.mul(norm_squared(d)?)?;
    if denominator.lo <= 0. {
        out.reason = "normal-unresolved";
        return Ok(out);
    }
    let mut cross = [I::point(0.); 3];
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        cross[k] = normal[a].mul(d[b])?.sub(normal[b].mul(d[a])?)?;
    }
    let sine = norm_squared(cross)?.div(denominator)?;
    let bounds = [sine.lo.max(0.), sine.hi.min(1.)];
    I::new(bounds[0], bounds[1])?;
    out.sine_squared_interval = Some(bounds);
    out.aligned = if bounds[1] <= max_sine_squared {
        Some(true)
    } else if bounds[0] > max_sine_squared {
        Some(false)
    } else {
        None
    };
    out.reason = match out.aligned {
        Some(true) => "angular-tolerance",
        Some(false) => "oblique",
        None => "angular-unresolved",
    };
    Ok(out)
}

/// Angular compatibility over both complete original UV rectangles. This
/// checks normal directions, not positional coincidence or a shared seam map.
#[derive(Debug)]
pub struct PairReport {
    pub aligned: Option<bool>,
    pub sine_squared_interval: Option<[f64; 2]>,
    pub normal_components: [Option<[[f64; 2]; 3]>; 2],
    pub spans: usize,
    pub reason: &'static str,
}
pub fn inspect_pair(
    surfaces: [&Surface; 2],
    domains: [[[f64; 2]; 2]; 2],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<PairReport> {
    // Validate both inputs before a budget refusal can hide invalid geometry.
    for side in 0..2 {
        validate_input(
            surfaces[side],
            domains[side],
            [1., 0., 0.],
            max_sine_squared,
            max_spans,
        )?;
    }
    let first = inspect(
        surfaces[0],
        domains[0],
        [1., 0., 0.],
        max_sine_squared,
        max_spans,
    )?;
    let mut out = PairReport {
        aligned: None,
        sine_squared_interval: None,
        normal_components: [first.normal_components, None],
        spans: first.spans,
        reason: "normal-unresolved",
    };
    if first.spans == max_spans {
        out.reason = "span-limit";
        return Ok(out);
    }
    let second = inspect(
        surfaces[1],
        domains[1],
        [1., 0., 0.],
        max_sine_squared,
        max_spans - first.spans,
    )?;
    out.spans += second.spans;
    out.normal_components[1] = second.normal_components;
    if first.reason == "span-limit" || second.reason == "span-limit" {
        out.reason = "span-limit";
        return Ok(out);
    }
    let mut normals = [[I::point(0.); 3]; 2];
    for side in 0..2 {
        let Some(hull) = out.normal_components[side] else {
            return Ok(out);
        };
        if hull.iter().all(|r| r[0] <= 0. && r[1] >= 0.) {
            return Ok(out);
        }
        let scale = hull.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
        for k in 0..3 {
            normals[side][k] = I::new(hull[k][0], hull[k][1])?.div(I::point(scale))?;
        }
    }
    let denominator = norm_squared(normals[0])?.mul(norm_squared(normals[1])?)?;
    if denominator.lo <= 0. {
        return Ok(out);
    }
    let mut cross = [I::point(0.); 3];
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        cross[k] = normals[0][a]
            .mul(normals[1][b])?
            .sub(normals[0][b].mul(normals[1][a])?)?;
    }
    let sine = norm_squared(cross)?.div(denominator)?;
    let bounds = [sine.lo.max(0.), sine.hi.min(1.)];
    I::new(bounds[0], bounds[1])?;
    out.sine_squared_interval = Some(bounds);
    out.aligned = if bounds[1] <= max_sine_squared {
        Some(true)
    } else if bounds[0] > max_sine_squared {
        Some(false)
    } else {
        None
    };
    out.reason = match out.aligned {
        Some(true) => "angular-tolerance",
        Some(false) => "oblique",
        None => "angular-unresolved",
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
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
        }
    }
    #[test]
    fn pair_normals_distinguish_tangency_fold_and_singular_transition() {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[0] += 100.;
                p[1] -= 20.;
            }
        }
        let domains = [[[0., 1.], [0., 1.]]; 2];
        let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
        assert_eq!(r.aligned, Some(true));
        assert_eq!(r.spans, 2);
        // Position is deliberately different: angular agreement is not G0/G1.
        b.control_points.reverse();
        assert_eq!(
            inspect_pair([&a, &b], domains, 1e-6, 2).unwrap().aligned,
            Some(true)
        );
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[0] - 100.;
            }
        }
        let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
        assert_eq!(r.aligned, Some(false));
        let d = r.sine_squared_interval.unwrap();
        assert!(d[0] <= 0.5 && d[1] >= 0.5);
        for row in &mut b.control_points {
            for p in row {
                p[1] = 0.;
                p[2] = 0.;
            }
        }
        let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.reason, "normal-unresolved");
        let r = inspect_pair([&a, &a], domains, 1e-6, 1).unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.reason, "span-limit");
        assert_eq!(r.spans, 1);
        let invalid = [domains[0], [[0., 2.], [0., 1.]]];
        assert!(inspect_pair([&a, &a], invalid, 1e-6, 1).is_err());
    }
    #[test]
    fn parallel_antiparallel_oblique_and_collapsed_normals_are_distinct() {
        let s = plane();
        let before = format!("{s:?}");
        for d in [[0., 0., 1.], [0., 0., -1.], [0., 0., f64::MAX]] {
            let r = inspect(&s, [[0.2, 0.8]; 2], d, 1e-10, 1).unwrap();
            assert_eq!(r.aligned, Some(true));
        }
        let r = inspect(&s, [[0.2, 0.8]; 2], [1., 0., 1.], 1e-10, 1).unwrap();
        assert_eq!(r.aligned, Some(false));
        let b = r.sine_squared_interval.unwrap();
        assert!(b[0] <= 0.5 && b[1] >= 0.5);
        let mut collapsed = s.clone();
        collapsed.control_points[1] = collapsed.control_points[0].clone();
        assert_eq!(
            inspect(&collapsed, [[0.2, 0.8]; 2], [0., 0., 1.], 1e-10, 1)
                .unwrap()
                .aligned,
            None
        );
        assert_eq!(format!("{s:?}"), before);
    }
    #[test]
    fn knot_sides_and_span_exhaustion_cannot_be_hidden() {
        let mut s = plane();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .push(vec![vec![2., 0., 1.], vec![2., 1., 1.]]);
        s.weights.push(vec![1.; 2]);
        let r = inspect(&s, [[0.5, 0.5], [0.3, 0.7]], [0., 0., 1.], 1e-6, 1).unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.reason, "span-limit");
        assert!(r.normal_components.is_none());
        let r = inspect(&s, [[0.5, 0.5], [0.3, 0.7]], [0., 0., 1.], 1e-6, 2).unwrap();
        assert_ne!(r.aligned, Some(true));
        assert_eq!(r.spans, 2);
        for (d, t, n) in [
            ([0.; 3], 1e-6, 1),
            ([0., 0., 1.], 1., 1),
            ([0., 0., 1.], 1e-6, 0),
        ] {
            assert!(inspect(&s, [[0., 1.]; 2], d, t, n).is_err());
        }
    }
    #[test]
    fn rational_original_derivatives_stay_enclosed_on_narrow_and_point_rectangles() {
        let s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17],
            knots_v: vec![-0.71, -0.71, -0.71, 0.29, 0.29, 0.29],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64, j as f64, (i * i + 2 * j * j + i * j) as f64 / 10.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1., 0.7, 1.2], vec![0.9, 1.1, 0.8], vec![1.3, 0.6, 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = format!("{s:?}");
        for domain in [
            [[0.43, 0.53], [-0.23, -0.13]],
            [[0.43, 0.43_f64.next_up()], [-0.23, (-0.23_f64).next_up()]],
            [[0.43, 0.43], [-0.23, -0.23]],
        ] {
            let r = inspect(&s, domain, [0.3, -0.2, 1.], 1e-6, 1).unwrap();
            let bounds = r.normal_components.unwrap();
            let sine = r.sine_squared_interval.unwrap();
            for i in 0..=4 {
                for j in 0..=4 {
                    let u = domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / 4.;
                    let v = domain[1][0] + (domain[1][1] - domain[1][0]) * j as f64 / 4.;
                    let (a, b) = s.evaluate(u, v).unwrap().first_derivatives().unwrap();
                    let n = std::array::from_fn::<_, 3, _>(|k| {
                        a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3]
                    });
                    for k in 0..3 {
                        assert!(n[k] >= bounds[k][0] - 1e-11 && n[k] <= bounds[k][1] + 1e-11);
                    }
                    let d = [0.3, -0.2, 1.];
                    let cross = std::array::from_fn::<_, 3, _>(|k| {
                        n[(k + 1) % 3] * d[(k + 2) % 3] - n[(k + 2) % 3] * d[(k + 1) % 3]
                    });
                    let ratio = cross.iter().map(|x| x * x).sum::<f64>()
                        / (n.iter().map(|x| x * x).sum::<f64>()
                            * d.iter().map(|x| x * x).sum::<f64>());
                    assert!(ratio >= sine[0] - 1e-11 && ratio <= sine[1] + 1e-11);
                }
            }
        }
        let direction = s.evaluate(0.43, -0.23).unwrap().unit_normal().unwrap();
        let r = inspect(&s, [[0.43, 0.43], [-0.23, -0.23]], direction, 1e-10, 1).unwrap();
        assert_eq!(r.aligned, Some(true));
        assert_eq!(format!("{s:?}"), before);
    }
    #[test]
    fn general_spatial_plane_does_not_depend_on_a_cartesian_axis() {
        let mut s = plane();
        for row in &mut s.control_points {
            for p in row {
                let (u, v) = (p[0], p[1]);
                *p = vec![1024. + u, -2048. + 2. * u + v, 512. + 3. * v];
            }
        }
        let r = inspect(
            &s,
            [[0.31, 0.31_f64.next_up()], [0.67, 0.67_f64.next_up()]],
            [6., -3., 1.],
            1e-10,
            1,
        )
        .unwrap();
        assert_eq!(r.aligned, Some(true));
        let b = r.normal_components.unwrap();
        for (i, n) in [6., -3., 1.].into_iter().enumerate() {
            assert!(b[i][0] <= n && b[i][1] >= n);
        }
    }
}
