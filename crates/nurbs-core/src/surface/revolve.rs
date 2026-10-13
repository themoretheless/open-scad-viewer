use super::*;

pub fn revolve(c: &Curve, origin: [f64; 3], axis: [f64; 3], angle: f64) -> Result<Surface> {
    c.validate()?;
    check(
        c.control_points[0].len() == 3
            && origin
                .iter()
                .chain(&axis)
                .chain([angle].iter())
                .all(|v| v.is_finite())
            && norm(axis) > 0.
            && angle != 0.
            && angle.abs() <= 360.,
        "Revolution requires 3D data, a nonzero axis and angle within +/-360 degrees.",
    )?;
    let unit = axis.map(|v| v / norm(axis));
    let arcs = (angle.abs() / 90.).ceil() as usize;
    let delta = angle * std::f64::consts::PI / 180. / arcs as f64;
    let mut kv = vec![0.; 3];
    for i in 1..arcs {
        kv.extend([i as f64, i as f64]);
    }
    kv.extend([arcs as f64; 3]);
    let aw: Vec<f64> = (0..=2 * arcs)
        .map(|j| if j % 2 == 1 { (delta / 2.).cos() } else { 1. })
        .collect();
    let points = c
        .control_points
        .iter()
        .map(|p| {
            let relative: [f64; 3] = std::array::from_fn(|i| p[i] - origin[i]);
            let projection = dot(relative, unit);
            let center: [f64; 3] = std::array::from_fn(|i| origin[i] + projection * unit[i]);
            let radial: [f64; 3] = std::array::from_fn(|i| p[i] - center[i]);
            let perpendicular = cross(unit, radial);
            aw.iter()
                .enumerate()
                .map(|(j, w)| {
                    if j == 2 * arcs && angle.abs() == 360. {
                        return p.clone();
                    }
                    (0..3)
                        .map(|i| {
                            center[i]
                                + (((j as f64 * delta / 2.).cos()) * radial[i]
                                    + ((j as f64 * delta / 2.).sin()) * perpendicular[i])
                                    / w
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let result = Surface {
        degree_u: c.degree,
        degree_v: 2,
        knots_u: c.knots.clone(),
        knots_v: kv,
        control_points: points,
        weights: c
            .weights
            .iter()
            .map(|w| aw.iter().map(|v| w * v).collect())
            .collect(),
        periodic_u: c.periodic,
        periodic_v: false,
    };
    result.validate()?;
    Ok(result)
}

/// Validated immutable snapshot for repeated sampling by any downstream adapter.
#[derive(Clone)]
pub struct SurfaceSampler {
    surface: Surface,
}
impl SurfaceSampler {
    pub fn new(surface: &Surface) -> Result<Self> {
        surface.validate()?;
        Ok(Self {
            surface: surface.clone(),
        })
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Evaluation> {
        self.surface.evaluate_validated(u, v)
    }
    pub fn definition(&self) -> &Surface {
        &self.surface
    }
}

/// Exact translational sweep P(u)+Q(v)-Q(v_start). Profile orientation is fixed;
/// this is not a Frenet-frame sweep. Rational data and both parameterizations survive.
pub fn sweep(profile: &Curve, path: &Curve) -> Result<Surface> {
    profile.validate()?;
    path.validate()?;
    check(
        profile.control_points[0].len() == 3 && path.control_points[0].len() == 3,
        "Sweep requires two 3D curves",
    )?;
    let start = path.evaluate(path.domain()[0])?.point;
    let s = Surface {
        degree_u: profile.degree,
        degree_v: path.degree,
        knots_u: profile.knots.clone(),
        knots_v: path.knots.clone(),
        control_points: profile
            .control_points
            .iter()
            .map(|p| {
                path.control_points
                    .iter()
                    .map(|q| (0..3).map(|i| p[i] + q[i] - start[i]).collect())
                    .collect()
            })
            .collect(),
        weights: profile
            .weights
            .iter()
            .map(|w| path.weights.iter().map(|v| w * v).collect())
            .collect(),
        periodic_u: profile.periodic,
        periodic_v: path.periodic,
    };
    s.validate()?;
    Ok(s)
}
/// Compatible section construction: normalize domains, elevate degrees and unify
/// knot multiplicities without changing section geometry. V remains piecewise linear.
pub fn loft_aligned(curves: &[Curve]) -> Result<Surface> {
    check(
        (2..=256).contains(&curves.len()),
        "Loft requires 2..256 sections",
    )?;
    for c in curves {
        c.validate()?;
        check(c.control_points[0].len() == 3, "Loft requires 3D sections")?;
    }
    let degree = curves.iter().map(|c| c.degree).max().unwrap();
    let mut sections = Vec::new();
    for c in curves {
        let [a, b] = c.domain();
        let mut c = c.trim(a, b)?;
        c.knots.iter_mut().for_each(|k| *k = (*k - a) / (b - a));
        if c.degree < degree {
            c = c.elevate(degree)?;
        }
        sections.push(c);
    }
    let mut unique: Vec<f64> = sections
        .iter()
        .flat_map(|c| c.knots.iter().copied())
        .filter(|k| *k > 0. && *k < 1.)
        .collect();
    unique.sort_by(f64::total_cmp);
    unique.dedup();
    for k in unique {
        let count = sections
            .iter()
            .map(|c| c.knots.iter().filter(|&&v| v == k).count())
            .max()
            .unwrap();
        for c in &mut sections {
            let existing = c.knots.iter().filter(|&&v| v == k).count();
            if existing < count {
                *c = c.insert(k, count - existing)?;
            }
            check(
                c.control_points.len() <= 256,
                "Aligned loft exceeds 256 section controls",
            )?;
        }
    }
    loft(&sections)
}
