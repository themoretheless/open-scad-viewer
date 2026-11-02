//! Conservative whole-support boxes and oriented normal cones of exact patches.
//! These are geometric enclosures, not material/thickness admission by themselves.
use super::*;
use crate::distance_bounds::Interval as I;
type Net = Vec<Vec<[Q; 3]>>;
pub struct SupportFrame {
    bounds: [[Q; 2]; 3],
    normal: Option<[[f64; 2]; 3]>,
}
impl SupportFrame {
    pub fn bounds(&self) -> &[[Q; 2]; 3] {
        &self.bounds
    }
    pub fn normal(&self) -> Option<[[f64; 2]; 3]> {
        self.normal
    }
}
fn bounds(net: &Net, b: &mut Budget) -> Result<[[Q; 2]; 3]> {
    let mut out = std::array::from_fn(|k| [net[0][0][k].clone(), net[0][0][k].clone()]);
    for p in net.iter().flatten() {
        for k in 0..3 {
            let x = b.keep(p[k].clone())?;
            out[k][0] = out[k][0].clone().min(x.clone());
            out[k][1] = out[k][1].clone().max(x);
        }
    }
    Ok(out)
}
fn square(x: I) -> Result<I> {
    I::new(
        if x.lo <= 0. && x.hi >= 0. {
            0.
        } else {
            (x.lo * x.lo).min(x.hi * x.hi).next_down().max(0.)
        },
        (x.lo * x.lo).max(x.hi * x.hi).next_up(),
    )
}
fn normalize(n: [I; 3], reversed: bool) -> Result<Option<[[f64; 2]; 3]>> {
    let mut length = I::point(0.);
    for x in n {
        length = length.add(square(x)?)?;
    }
    if length.lo <= 0. {
        return Ok(None);
    }
    let length = I::new(
        length.lo.sqrt().next_down().max(0.),
        length.hi.sqrt().next_up(),
    )?;
    if length.lo <= 0. {
        return Ok(None);
    }
    let mut out = [[0.; 2]; 3];
    for k in 0..3 {
        let v = n[k].div(length)?;
        let v = if reversed { I::new(-v.hi, -v.lo)? } else { v };
        let lo = v.lo.max(-1.);
        let hi = v.hi.min(1.);
        if lo > hi {
            return Ok(None);
        }
        out[k] = [lo, hi];
    }
    // Unit length couples the component intervals. A nonzero guaranteed Z,
    // for example, excludes an X component arbitrarily close to +/-1.
    for k in 0..3 {
        let mut others = I::point(0.);
        for j in 0..3 {
            if j != k {
                let q = square(I::new(out[j][0], out[j][1])?)?;
                others = others.add(I::point(q.lo))?;
            }
        }
        let remaining = I::point(1.).sub(others)?.hi;
        if remaining < 0. {
            return Ok(None);
        }
        let bound = remaining.sqrt().next_up().min(1.);
        out[k] = [out[k][0].max(-bound), out[k][1].min(bound)];
        if out[k][0] > out[k][1] {
            return Ok(None);
        }
    }
    Ok(Some(out))
}
fn hull(net: &Net, b: &mut Budget) -> Result<[I; 3]> {
    let mut out = [I::point(0.); 3];
    for k in 0..3 {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for p in net.iter().flatten() {
            let x = curvature::enclose(&p[k], b)?;
            lo = lo.min(x.lo);
            hi = hi.max(x.hi);
        }
        out[k] = I::new(lo, hi)?;
    }
    Ok(out)
}
fn raw_normal(net: &Net, reversed: bool, b: &mut Budget) -> Result<Option<[[f64; 2]; 3]>> {
    let u = hull(&curvature::derivative(net, 0, b)?, b)?;
    let v = hull(&curvature::derivative(net, 1, b)?, b)?;
    let mut cross = [I::point(0.); 3];
    for k in 0..3 {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        cross[k] = u[i].mul(v[j])?.sub(u[j].mul(v[i])?)?;
    }
    normalize(cross, reversed)
}
impl Patch {
    pub fn support_frame(&self, reversed: bool, b: &mut Budget) -> Result<SupportFrame> {
        let box_ = bounds(&self.coefficients, b)?;
        let normal = if let Some(f) = self.graph_curvature_factors([[0., 1.]; 2], false, b)? {
            normalize(
                [f.normal_slopes[0], f.normal_slopes[1], I::point(1.)],
                reversed,
            )?
        } else {
            raw_normal(&self.coefficients, reversed, b)?
        };
        Ok(SupportFrame {
            bounds: box_,
            normal,
        })
    }
}
pub fn polynomial_support_frame(
    s: &Surface,
    reversed: bool,
    b: &mut Budget,
) -> Result<SupportFrame> {
    s.validate()?;
    let (n, m) = (s.degree_u, s.degree_v);
    check(
        !s.periodic_u
            && !s.periodic_v
            && (1..=32).contains(&n)
            && (1..=32).contains(&m)
            && s.control_points.len() == n + 1
            && s.control_points[0].len() == m + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Support frame requires a polynomial Bezier tensor",
    )?;
    for (knots, d) in [(&s.knots_u, n), (&s.knots_v, m)] {
        check(
            knots[..=d].iter().all(|x| *x == knots[d])
                && knots[d + 1..].iter().all(|x| *x == knots[d + 1]),
            "Support frame requires clamped original knots",
        )?;
    }
    let net = s
        .control_points
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| Ok([b.authored(p[0])?, b.authored(p[1])?, b.authored(p[2])?]))
                .collect()
        })
        .collect::<Result<Net>>()?;
    let box_ = bounds(&net, b)?;
    let mut normal = None;
    // A planar support plus a COMPLETE regular injective chart proves its
    // corner normal orientation persists over the connected whole support.
    // Merely sampling that corner without this premise would be unsound.
    if let Some(axis) = (0..3).find(|&k| box_[k][0] == box_[k][1]) {
        if super::polynomial_surface_embedding(s, b)?.is_some() {
            let i = (axis + 1) % 3;
            let j = (axis + 2) % 3;
            let ui = b.keep(&net[1][0][i] - &net[0][0][i])?;
            let uj = b.keep(&net[1][0][j] - &net[0][0][j])?;
            let vi = b.keep(&net[0][1][i] - &net[0][0][i])?;
            let vj = b.keep(&net[0][1][j] - &net[0][0][j])?;
            let a = b.keep(ui * vj)?;
            let z = b.keep(uj * vi)?;
            let cross = b.keep(a - z)?;
            if !cross.is_zero() {
                let sign = if cross.is_positive() != reversed {
                    1.
                } else {
                    -1.
                };
                let mut cone = [[0.; 2]; 3];
                cone[axis] = [sign, sign];
                normal = Some(cone);
            }
        }
    }
    if normal.is_none() {
        normal = raw_normal(&net, reversed, b)?;
    }
    Ok(SupportFrame {
        bounds: box_,
        normal,
    })
}
/// True excludes any nonzero INSIDE material chord with both endpoints
/// aligned within alpha to their outward normals (opposite entry/exit signs).
/// No material/normal ownership is inferred from these raw numeric enclosures.
pub fn opposed_normal_chords_excluded(
    a: [[f64; 2]; 3],
    z: [[f64; 2]; 3],
    max_sine_squared: f64,
) -> Result<bool> {
    check(
        max_sine_squared.is_finite() && (0. ..0.5).contains(&max_sine_squared),
        "Choose an endpoint normal angle below pi/4",
    )?;
    for r in a.into_iter().chain(z) {
        I::new(r[0], r[1])?;
        check(
            r[0] >= -1. && r[1] <= 1.,
            "Unit-normal component enclosure must lie in [-1,1]",
        )?;
    }
    // The physical normals are unit vectors, not arbitrary corners of their
    // interval boxes. A shared strict hemisphere cap can rule out opposition
    // even when independent component products lose that correlation.
    for k in 0..3 {
        let c = if a[k][0] > 0. && z[k][0] > 0. {
            a[k][0].min(z[k][0])
        } else if a[k][1] < 0. && z[k][1] < 0. {
            (-a[k][1]).min(-z[k][1])
        } else {
            0.
        };
        if c > 0. && square(I::point(c))?.lo > max_sine_squared {
            return Ok(true);
        }
    }
    let mut dot = I::point(0.);
    for k in 0..3 {
        dot = dot.add(I::new(a[k][0], a[k][1])?.mul(I::new(z[k][0], z[k][1])?)?)?;
    }
    // Opposed outward normals of an aligned entering/exiting chord necessarily
    // have dot <= -cos(2alpha) = 2*sin(alpha)^2-1.
    let threshold = I::point(max_sine_squared)
        .mul(I::point(2.))?
        .sub(I::point(1.))?;
    Ok(dot.lo > threshold.hi)
}
pub fn frame_distance_at_least(
    a: &SupportFrame,
    z: &SupportFrame,
    minimum: &Q,
    b: &mut Budget,
) -> Result<bool> {
    let minimum = b.keep(minimum.clone())?;
    check(minimum > Q::zero(), "Positive exact wall distance required")?;
    let mut squared = Q::zero();
    for k in 0..3 {
        let gap = if a.bounds[k][1] < z.bounds[k][0] {
            b.keep(&z.bounds[k][0] - &a.bounds[k][1])?
        } else if z.bounds[k][1] < a.bounds[k][0] {
            b.keep(&a.bounds[k][0] - &z.bounds[k][1])?
        } else {
            Q::zero()
        };
        let term = b.keep(&gap * &gap)?;
        squared = b.keep(squared + term)?;
    }
    let limit = b.keep(&minimum * &minimum)?;
    Ok(squared >= limit)
}

/// Exact inverse on a coordinate-plane polynomial support with an affine U
/// coordinate clock and linear V. Does not establish trimmed membership.
pub fn polynomial_plane_point(
    s: &Surface,
    axis: usize,
    target: [Q; 3],
    b: &mut Budget,
) -> Result<Option<[Q; 2]>> {
    s.validate()?;
    let n = s.degree_u;
    check(
        axis < 3
            && !s.periodic_u
            && !s.periodic_v
            && (1..=32).contains(&n)
            && s.degree_v == 1
            && s.control_points.len() == n + 1
            && s.control_points[0].len() == 2
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Plane inverse requires polynomial Bezier U and linear V",
    )?;
    for (knots, d) in [(&s.knots_u, n), (&s.knots_v, 1)] {
        check(
            knots[..=d].iter().all(|x| *x == knots[d])
                && knots[d + 1..].iter().all(|x| *x == knots[d + 1]),
            "Plane inverse requires original clamped domains",
        )?;
    }
    for x in &target {
        b.keep(x.clone())?;
    }
    let plane = b.authored(s.control_points[0][0][axis])?;
    if target[axis] != plane {
        return Ok(None);
    }
    for p in s.control_points.iter().flatten() {
        if b.authored(p[axis])? != plane {
            return Ok(None);
        }
    }
    let remaining = (0..3).filter(|&k| k != axis).collect::<Vec<_>>();
    for order in 0..2 {
        let a = remaining[order];
        let z = remaining[1 - order];
        let low = b.authored(s.control_points[0][0][a])?;
        let high = b.authored(s.control_points[n][0][a])?;
        let span = b.keep(&high - &low)?;
        if span.is_zero() {
            continue;
        }
        let mut affine = true;
        for (i, row) in s.control_points.iter().enumerate() {
            let expected = b.keep(&low + &span * ratio(i as u64, n as u64))?;
            for p in row {
                if b.authored(p[a])? != expected {
                    affine = false;
                }
            }
        }
        if !affine {
            continue;
        }
        let u = b.keep((&target[a] - &low) / &span)?;
        if u < Q::zero() || u > Q::one() {
            continue;
        }
        let ulow = b.authored(s.knots_u[n])?;
        let uhigh = b.authored(s.knots_u[n + 1])?;
        let vlow = b.authored(s.knots_v[1])?;
        let vhigh = b.authored(s.knots_v[2])?;
        let u = b.keep(&ulow + u * (uhigh - ulow.clone()))?;
        let line = super::polynomial_surface_line(
            s,
            [[u.clone(), vlow.clone()], [u.clone(), vhigh.clone()]],
            b,
        )?;
        let bottom = &line[0][z];
        let top = &line.last().unwrap()[z];
        let height = b.keep(top - bottom)?;
        if height.is_zero() {
            continue;
        }
        let v = b.keep((&target[z] - bottom) / height)?;
        if v < Q::zero() || v > Q::one() {
            continue;
        }
        let v = b.keep(&vlow + v * (vhigh - vlow.clone()))?;
        let uv = [u, v];
        let points = super::polynomial_surface_line(s, [uv.clone(), uv.clone()], b)?;
        if points.iter().all(|p| *p == target) {
            return Ok(Some(uv));
        }
    }
    Ok(None)
}
/// Strict interior of a fully checked convex CCW authored UV polygon.
pub fn convex_uv_strict_contains(
    lines: &[[[Q; 2]; 2]],
    point: &[Q; 2],
    b: &mut Budget,
) -> Result<bool> {
    check(
        (3..=128).contains(&lines.len()),
        "Choose a bounded convex UV polygon",
    )?;
    fn cross(a: &[Q; 2], z: &[Q; 2], p: &[Q; 2], b: &mut Budget) -> Result<Q> {
        let x = b.keep(&z[0] - &a[0])?;
        let y = b.keep(&z[1] - &a[1])?;
        let px = b.keep(&p[0] - &a[0])?;
        let py = b.keep(&p[1] - &a[1])?;
        let left = b.keep(x * py)?;
        let right = b.keep(y * px)?;
        b.keep(left - right)
    }
    let mut inside = true;
    for (i, line) in lines.iter().enumerate() {
        check(
            line[0] != line[1] && line[1] == lines[(i + 1) % lines.len()][0],
            "UV polygon must be exactly closed and nondegenerate",
        )?;
        for other in lines {
            check(
                cross(&line[0], &line[1], &other[0], b)? >= Q::zero(),
                "UV polygon must be convex and CCW",
            )?;
        }
        if cross(&line[0], &line[1], point, b)? <= Q::zero() {
            inside = false;
        }
    }
    Ok(inside)
}
