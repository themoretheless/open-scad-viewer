//! One exact affine quintic motion for ALL original profile poles. Independent
//! pole snapping cannot preserve general profile joins; this candidate never
//! edits the original profile. No embedding, Solid or full-boundary G2 credit.
use super::*;
use nurbs_core::interval_eval::Interval as I;
type F = [f64; 3];
/// Columns of the affine linear part followed by translation.
pub type AffinePose = [[f64; 3]; 4];
#[derive(Clone, Copy)]
pub struct AffineStationLimits {
    pub quantum: f64,
    pub tolerance: f64,
    pub work: u64,
    pub matrix_cells: usize,
}
pub struct AffineStationWalls {
    pub sides: Option<Vec<Vec<Surface>>>,
    pub sections: Option<Vec<Vec<Vec<Curve>>>>,
    pub poses: Option<Vec<[AffinePose; 6]>>,
    /// Poses act on original coordinates minus this exact common origin.
    pub source_origin: Option<[f64; 3]>,
    pub displacement_upper: Option<f64>,
    pub affine_regular: bool,
    pub work: u64,
    pub matrix_cells: usize,
    pub reason: &'static str,
}
fn sub(a: F, b: F) -> F {
    std::array::from_fn(|k| a[k] - b[k])
}
fn cross(a: F, b: F) -> F {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: F, b: F) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn exact_add(a: f64, b: f64) -> Option<f64> {
    let s = a + b;
    let bv = s - a;
    let av = s - bv;
    let br = b - bv;
    let ar = a - av;
    (s.is_finite() && ar + br == 0.).then_some(s)
}
fn exact_mul(a: f64, b: f64) -> Option<f64> {
    let p = a * b;
    // A zero product is exact only when one factor is literally zero. Keep
    // multiplication residuals normal so FMA cannot hide underflow loss.
    if a == 0. || b == 0. {
        return Some(0.);
    }
    (p.is_finite()
        && p.abs() >= 1e-100
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && a.mul_add(b, -p) == 0.)
        .then_some(p)
}
fn charge(out: &mut AffineStationWalls, max: u64) -> bool {
    if out.work == max {
        false
    } else {
        out.work += 1;
        true
    }
}
fn snap(x: f64, q: f64) -> Option<f64> {
    let n = (x / q).round();
    let y = n * q;
    (n.is_finite() && n.abs() < 2f64.powi(48) && y.is_finite() && y / q == n).then_some(y)
}
fn source_grid(points: &[F]) -> Option<f64> {
    let mut exponent = 1024;
    for x in points.iter().flatten().copied().filter(|x| *x != 0.) {
        let bits = x.abs().to_bits();
        let e = ((bits >> 52) & 2047) as i32;
        let mantissa = if e == 0 {
            bits & ((1u64 << 52) - 1)
        } else {
            (bits & ((1u64 << 52) - 1)) | (1u64 << 52)
        };
        let power = if e == 0 { -1074 } else { e - 1023 - 52 };
        exponent = exponent.min(power + mantissa.trailing_zeros() as i32);
    }
    let q = 2f64.powi(exponent);
    (q.is_normal() && q > 0.).then_some(q)
}
fn fit(source: &[F], target: &[F]) -> Option<AffinePose> {
    let p = source[0];
    let r = target[0];
    let i = (1..source.len()).max_by(|&a, &b| {
        dot(sub(source[a], p), sub(source[a], p))
            .total_cmp(&dot(sub(source[b], p), sub(source[b], p)))
    })?;
    let a = sub(source[i], p);
    let x = sub(target[i], r);
    let j = (1..source.len()).max_by(|&u, &v| {
        dot(cross(a, sub(source[u], p)), cross(a, sub(source[u], p))).total_cmp(&dot(
            cross(a, sub(source[v], p)),
            cross(a, sub(source[v], p)),
        ))
    })?;
    let b = sub(source[j], p);
    let y = sub(target[j], r);
    let normal = cross(a, b);
    let k = (1..source.len()).max_by(|&u, &v| {
        dot(normal, sub(source[u], p))
            .abs()
            .total_cmp(&dot(normal, sub(source[v], p)).abs())
    })?;
    let (c, z) = if dot(normal, sub(source[k], p)) != 0. {
        (sub(source[k], p), sub(target[k], r))
    } else {
        (normal, cross(x, y))
    };
    let cofactors = [cross(b, c), cross(c, a), cross(a, b)];
    let det = dot(a, cofactors[0]);
    if !det.is_finite() || det == 0. {
        return None;
    }
    let inverse = cofactors.map(|r| r.map(|x| x / det));
    let world = [x, y, z];
    let mut pose = [[0.; 3]; 4];
    for col in 0..3 {
        for row in 0..3 {
            pose[col][row] = (0..3).map(|k| world[k][row] * inverse[k][col]).sum();
        }
    }
    pose[3] =
        std::array::from_fn(|row| r[row] - (0..3).map(|col| pose[col][row] * p[col]).sum::<f64>());
    pose.iter().flatten().all(|x| x.is_finite()).then_some(pose)
}
fn apply(pose: AffinePose, p: F) -> Option<F> {
    let mut q = pose[3];
    for col in 0..3 {
        for row in 0..3 {
            q[row] = exact_add(q[row], exact_mul(pose[col][row], p[col])?)?;
        }
    }
    Some(q)
}
fn bezier(values: [f64; 6], t: I) -> Result<I> {
    let mut a = values.map(I::point);
    let one = I::point(1.).sub(t)?;
    for n in (1..6).rev() {
        for i in 0..n {
            let x = a[i];
            let y = a[i + 1];
            let direct = x.mul(one)?.add(y.mul(t)?)?;
            let affine = x.add(y.sub(x)?.mul(t)?)?;
            a[i] = I::new(direct.lo.max(affine.lo), direct.hi.min(affine.hi))?;
        }
    }
    Ok(a[0])
}
fn determinant(p: [AffinePose; 6], t: I) -> Result<I> {
    let mut a = [[I::point(0.); 3]; 3];
    for j in 0..3 {
        for k in 0..3 {
            a[j][k] = bezier(p.map(|p| p[j][k]), t)?;
        }
    }
    let cross = [
        a[1][1].mul(a[2][2])?.sub(a[1][2].mul(a[2][1])?)?,
        a[1][2].mul(a[2][0])?.sub(a[1][0].mul(a[2][2])?)?,
        a[1][0].mul(a[2][1])?.sub(a[1][1].mul(a[2][0])?)?,
    ];
    a[0][0]
        .mul(cross[0])?
        .add(a[0][1].mul(cross[1])?)?
        .add(a[0][2].mul(cross[2])?)
}
fn regular(poses: &[[AffinePose; 6]], out: &mut AffineStationWalls, max: usize) -> Result<bool> {
    let mut sign = None;
    for &p in poses {
        let mut pending = vec![[0., 1.]];
        while let Some([a, b]) = pending.pop() {
            if out.matrix_cells == max {
                out.reason = "affine-regularity-work-limit";
                return Ok(false);
            }
            out.matrix_cells += 1;
            let d = determinant(p, I::new(a, b)?)?;
            let current = if d.lo > 0. {
                Some(true)
            } else if d.hi < 0. {
                Some(false)
            } else {
                None
            };
            if let Some(s) = current {
                if sign.is_some_and(|old| old != s) {
                    out.reason = "affine-orientation-unproved";
                    return Ok(false);
                }
                sign = Some(s);
                continue;
            }
            let mid = a * 0.5 + b * 0.5;
            if !(a < mid && mid < b) {
                out.reason = "affine-matrix-regularity-unproved";
                return Ok(false);
            }
            pending.push([mid, b]);
            pending.push([a, mid]);
        }
    }
    Ok(sign.is_some())
}
fn compatible(a: &Curve, b: &Curve) -> bool {
    let n = a.degree + 1;
    a.degree == b.degree
        && a.control_points.len() == n
        && b.control_points.len() == n
        && a.weights == b.weights
        && !a.periodic
        && !b.periodic
        && a.knots.len() == 2 * n
        && b.knots.len() == 2 * n
        && [&a.knots, &b.knots]
            .iter()
            .all(|k| k[..n].iter().all(|x| *x == k[0]) && k[n..].iter().all(|x| *x == k[n]))
}
/// A bounded candidate preserving each original profile by ONE nonsingular
/// affine map, not independent pole rounding. The caller must still certify
/// actual complete-boundary smoothness, original-law correspondence and Solid.
pub fn affine_station_walls(
    original: &[Vec<Curve>],
    sections: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    limits: AffineStationLimits,
) -> Result<AffineStationWalls> {
    let AffineStationLimits {
        quantum,
        tolerance,
        work: max,
        matrix_cells: max_cells,
    } = limits;
    if !(2..=1025).contains(&sections.len())
        || original.is_empty()
        || original.len() > 16
        || !quantum.is_normal()
        || quantum <= 0.
        || quantum.to_bits() & ((1u64 << 52) - 1) != 0
        || !tolerance.is_finite()
        || tolerance < 0.
        || max > 1000000
        || max_cells > 100000
        || sharp.iter().any(|i| *i >= sections.len())
        || sharp
            .iter()
            .enumerate()
            .any(|(i, x)| sharp[..i].contains(x))
    {
        return Err(err("Invalid affine station candidate limits"));
    }
    let mut out = AffineStationWalls {
        sides: None,
        sections: None,
        poses: None,
        source_origin: None,
        displacement_upper: None,
        affine_regular: false,
        work: 0,
        matrix_cells: 0,
        reason: "work-limit",
    };
    let n = sections.len();
    // Reject oversized caller arrays before validating or copying their poles.
    // The face ceiling also bounds profile spans independently of supplied work.
    let span_limit = (MAX_FACES - if closed { 0 } else { 2 }) / (n - 1);
    let Some(spans) = original.iter().try_fold(0usize, |count, wire| {
        count.checked_add(wire.len()).filter(|&x| x <= span_limit)
    }) else {
        out.reason = "face-limit";
        return Ok(out);
    };
    if spans == 0
        || original.iter().any(Vec::is_empty)
        || original
            .iter()
            .flatten()
            .any(|c| c.control_points.len() > 33)
        || sections.iter().any(|s| {
            s.len() != original.len()
                || s.iter().zip(original).any(|(a, b)| {
                    a.len() != b.len() || a.iter().any(|c| c.control_points.len() > 33)
                })
        })
    {
        out.reason = "source-affine-profile-basis-unproved";
        return Ok(out);
    }
    for c in original.iter().flatten() {
        c.validate()?;
    }
    for c in sections.iter().flatten().flatten() {
        c.validate()?;
    }
    if closed && (n < 4 || sections.first() != sections.last()) {
        out.reason = "periodic-source-mismatch";
        return Ok(out);
    }
    if original.iter().any(Vec::is_empty)
        || sections.iter().any(|s| {
            s.len() != original.len()
                || s.iter().zip(original).any(|(a, b)| {
                    a.len() != b.len() || a.iter().zip(b).any(|(a, b)| !compatible(a, b))
                })
        })
        || sections.iter().any(|s| {
            s.iter()
                .zip(&sections[0])
                .any(|(a, b)| a.iter().zip(b).any(|(a, b)| a.knots != b.knots))
        })
        || original
            .iter()
            .flatten()
            .chain(sections.iter().flatten().flatten())
            .any(|c| c.control_points.iter().any(|p| p.len() != 3))
    {
        out.reason = "source-affine-profile-basis-unproved";
        return Ok(out);
    }
    let points = original
        .iter()
        .flatten()
        .flat_map(|c| &c.control_points)
        .map(|p| [p[0], p[1], p[2]])
        .collect::<Vec<_>>();
    let mut origin = points[0];
    if points
        .iter()
        .any(|p| (0..3).any(|k| exact_add(p[k], -origin[k]).is_none()))
    {
        origin = [0.; 3];
    }
    let local = points
        .iter()
        .map(|p| std::array::from_fn(|k| exact_add(p[k], -origin[k]).unwrap()))
        .collect::<Vec<F>>();
    let Some(grid) = source_grid(&local) else {
        out.reason = "source-profile-lattice-range";
        return Ok(out);
    };
    let translation_grid = grid * quantum;
    if !translation_grid.is_normal() {
        out.reason = "source-profile-lattice-range";
        return Ok(out);
    }
    let mut stations = Vec::with_capacity(n);
    for (index, row) in sections.iter().enumerate() {
        if closed && index == n - 1 {
            stations.push(stations[0]);
            break;
        }
        for _ in 0..local.len() {
            if !charge(&mut out, max) {
                return Ok(out);
            }
        }
        let target = row
            .iter()
            .flatten()
            .flat_map(|c| &c.control_points)
            .map(|p| [p[0], p[1], p[2]])
            .collect::<Vec<_>>();
        let Some(mut pose) = fit(&local, &target) else {
            out.reason = "affine-fit-proposal-unavailable";
            return Ok(out);
        };
        for col in 0..4 {
            for k in 0..3 {
                if !charge(&mut out, max) {
                    return Ok(out);
                }
                let Some(x) = snap(
                    pose[col][k],
                    if col == 3 { translation_grid } else { quantum },
                ) else {
                    out.reason = "affine-fit-lattice-range";
                    return Ok(out);
                };
                pose[col][k] = x;
            }
        }
        stations.push(pose);
    }
    let is_sharp = |i: usize| {
        sharp.contains(&i)
            || closed && (i == 0 || i == n - 1) && (sharp.contains(&0) || sharp.contains(&(n - 1)))
    };
    let tangent = |i: usize, previous: bool, col: usize, k: usize| {
        let (a, b, den) = if !closed && i == 0 {
            (0, 1, 5.)
        } else if !closed && i == n - 1 {
            (n - 2, n - 1, 5.)
        } else if is_sharp(i) {
            if previous {
                (i.saturating_sub(1), i, 5.)
            } else {
                (i, (i + 1).min(n - 1), 5.)
            }
        } else {
            (
                if i == 0 || i == n - 1 { n - 2 } else { i - 1 },
                if i == 0 || i == n - 1 { 1 } else { i + 1 },
                10.,
            )
        };
        snap(
            (stations[b][col][k] - stations[a][col][k]) / den,
            if col == 3 { translation_grid } else { quantum },
        )
    };
    let mut poses = Vec::with_capacity(n - 1);
    for layer in 0..n - 1 {
        let mut p = [stations[layer]; 6];
        p[5] = stations[layer + 1];
        for col in 0..4 {
            for k in 0..3 {
                if !charge(&mut out, max) {
                    return Ok(out);
                }
                let (Some(a), Some(b)) = (
                    tangent(layer, false, col, k),
                    tangent(layer + 1, true, col, k),
                ) else {
                    out.reason = "affine-tangent-lattice-range";
                    return Ok(out);
                };
                let Some(values) = exact_mul(2., a).zip(exact_mul(2., b)).and_then(|(aa, bb)| {
                    Some([
                        exact_add(p[0][col][k], a)?,
                        exact_add(p[0][col][k], aa)?,
                        exact_add(p[5][col][k], -bb)?,
                        exact_add(p[5][col][k], -b)?,
                    ])
                }) else {
                    out.reason = "affine-exact-station-jets-unproved";
                    return Ok(out);
                };
                for j in 0..4 {
                    p[j + 1][col][k] = values[j];
                }
            }
        }
        poses.push(p);
    }
    if !regular(&poses, &mut out, max_cells)? {
        return Ok(out);
    }
    out.affine_regular = true;
    let mut proposed = Vec::with_capacity(n);
    let mut sides = vec![Vec::new(); original.len()];
    let mut upper = 0_f64;
    for pose in stations.iter().copied() {
        let mut row = original.to_vec();
        let mut at = 0;
        for c in row.iter_mut().flatten() {
            for p in &mut c.control_points {
                if !charge(&mut out, max) {
                    return Ok(out);
                }
                let Some(q) = apply(pose, local[at]) else {
                    out.reason = "affine-profile-product-unproved";
                    return Ok(out);
                };
                *p = q.to_vec();
                at += 1;
            }
        }
        // The BRep wall atlas uses [0,1] per literal Bezier. Affine domain
        // normalization changes no coefficient or pointwise Bernstein bound.
        for c in row.iter_mut().flatten() {
            c.knots = [vec![0.; c.degree + 1], vec![1.; c.degree + 1]].concat();
        }
        proposed.push(row);
    }
    for (layer, pose) in poses.iter().enumerate() {
        let mut at = 0;
        for (ring, wire) in original.iter().enumerate() {
            for (span, c) in wire.iter().enumerate() {
                let mut poles = vec![vec![vec![0.; 3]; 6]; c.control_points.len()];
                for i in 0..c.control_points.len() {
                    for j in 0..6 {
                        if !charge(&mut out, max) {
                            return Ok(out);
                        }
                        let Some(q) = apply(pose[j], local[at + i]) else {
                            out.reason = "affine-profile-product-unproved";
                            return Ok(out);
                        };
                        let t = I::point(j as f64).div(I::point(5.))?;
                        let mut squared = I::point(0.);
                        for k in 0..3 {
                            if !charge(&mut out, max) {
                                return Ok(out);
                            }
                            let a = sections[layer][ring][span].control_points[i][k];
                            let b = sections[layer + 1][ring][span].control_points[i][k];
                            let old = I::point(a)
                                .mul(I::point(1.).sub(t)?)?
                                .add(I::point(b).mul(t)?)?;
                            let delta = I::point(q[k]).sub(old)?;
                            let radius = delta.lo.abs().max(delta.hi.abs());
                            squared = squared.add(I::point(radius).mul(I::point(radius))?)?;
                        }
                        upper = upper.max(squared.hi.max(0.).sqrt().next_up());
                        poles[i][j] = q.to_vec();
                    }
                }
                at += c.control_points.len();
                let surface = Surface {
                    degree_u: c.degree,
                    degree_v: 5,
                    knots_u: [vec![0.; c.degree + 1], vec![1.; c.degree + 1]].concat(),
                    knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
                    control_points: poles,
                    weights: c.weights.iter().map(|w| vec![*w; 6]).collect(),
                    periodic_u: false,
                    periodic_v: false,
                };
                surface.validate()?;
                sides[ring].push(surface);
            }
        }
    }
    out.displacement_upper = Some(upper);
    if upper > tolerance {
        out.reason = "affine-displacement-budget";
        return Ok(out);
    }
    out.sides = Some(sides);
    out.sections = Some(proposed);
    out.poses = Some(poses);
    out.source_origin = Some(origin);
    out.reason = "bounded-exact-affine-quintic-profiles";
    Ok(out)
}
#[cfg(test)]
mod tests;
