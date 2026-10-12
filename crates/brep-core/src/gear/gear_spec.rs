use super::*;

fn invalid(message: impl Into<String>) -> Error {
    Error::new("BREP_INVALID_SIZE", message)
}

#[derive(Clone, Debug)]
pub struct GearSpec {
    pub module: f64,
    pub teeth: usize,
    pub pressure_angle_deg: f64,
    pub height: f64,
    /// Helix angle at the pitch circle, signed (positive twists
    /// counter-clockwise with height); zero is a spur gear.
    pub helix_angle_deg: f64,
    /// Two mirrored helical halves.
    pub herringbone: bool,
    /// Central bore diameter of an external gear (zero for none).
    pub bore: f64,
    /// Internal (ring) gear: teeth point inward, `rim_width` of material
    /// outside the root circle.
    pub internal: bool,
    pub rim_width: f64,
    pub clearance: f64,
    pub backlash: f64,
}

impl Default for GearSpec {
    fn default() -> Self {
        Self {
            module: 1.,
            teeth: 20,
            pressure_angle_deg: 20.,
            height: 5.,
            helix_angle_deg: 0.,
            herringbone: false,
            bore: 0.,
            internal: false,
            rim_width: 2.,
            clearance: 0.25,
            backlash: 0.,
        }
    }
}

/// Derived radii and angles of a spec, mirroring the sampled generator.
#[derive(Clone, Debug)]
pub struct GearGeometry {
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    pub outside_radius: f64,
    /// Deviation of the fitted flanks from the analytic involute.
    pub flank_deviation: f64,
    /// Deviation of the helical walls from the exact sweep (zero for spur).
    pub sweep_deviation: f64,
    pub face_count: usize,
}

// ---------------------------------------------------------------------------
// 2D outline
// ---------------------------------------------------------------------------

fn polar(r: f64, a: f64) -> [f64; 2] {
    [r * a.cos(), r * a.sin()]
}

fn line2(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}

/// Exact circular arc about the origin from `from` to `to` (sweep below a
/// half turn, either sense).
fn arc2(r: f64, from: f64, to: f64) -> Result<Curve> {
    let sweep = to - from;
    if !(sweep.abs() > 1e-12 && sweep.abs() < ARC_WEIGHT_MAX_SWEEP) {
        return Err(invalid("Gear arc must sweep less than a half turn"));
    }
    let mid = (from + to) / 2.;
    let w = (sweep / 2.).cos();
    Ok(Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            polar(r, from).to_vec(),
            polar(r / w, mid).to_vec(),
            polar(r, to).to_vec(),
        ],
        weights: vec![1., w, 1.],
        periodic: false,
    })
}

/// Full circle as `arcs` exact arcs (at least four), counter-clockwise
/// from +x.
fn circle2(r: f64, arcs: usize) -> Result<Vec<Curve>> {
    let arcs = arcs.max(4);
    (0..arcs)
        .map(|q| {
            arc2(
                r,
                q as f64 * TAU / arcs as f64,
                (q + 1) as f64 * TAU / arcs as f64,
            )
        })
        .collect()
}

/// Arc count for a circle of radius `r` that must stay outside radius
/// `inner` even when each arc is displayed as two chords: the inscribed
/// radius of the chord polygon must clear `inner`.
fn arcs_clearing(r: f64, inner: f64) -> usize {
    if inner <= 0. || inner >= r {
        return 4;
    }
    let half = (inner / r).acos();
    ((PI / (2. * half)).ceil() as usize).clamp(4, 64)
}

/// Arcs per full circle on a gear: one arc for every two teeth, so the rim
/// and the bore are displayed as round as the teeth at any tessellation
/// level, while the arc count stays exact and bounded.
pub fn circle_arcs(teeth: usize) -> usize {
    (teeth / 2).clamp(4, 64)
}

/// Involute of the base circle `base` between roll parameters `t0..t1`,
/// placed at the tooth centre with half-angle function `half`; `sign` picks
/// the flank side. Fitted by interpolation and verified.
fn involute(
    base: f64,
    t0: f64,
    t1: f64,
    center: f64,
    sign: f64,
    half: &dyn Fn(f64) -> f64,
) -> Result<(Curve, f64)> {
    let point = |t: f64| -> [f64; 2] {
        let r = base * (1. + t * t).sqrt();
        polar(r, center + sign * half(r))
    };
    // NURBS curves and surfaces admit at most 32 control points per
    // direction, so the fit is refined only up to 24 spans.
    let schedule = [8usize, 12, 16, 24];
    for (step, &samples) in schedule.iter().enumerate() {
        let last = step + 1 == schedule.len();
        let params: Vec<f64> = (0..=samples).map(|i| i as f64 / samples as f64).collect();
        let data: Vec<Vec<f64>> = params
            .iter()
            .map(|&s| point(t0 + (t1 - t0) * s).to_vec())
            .collect();
        let curve = interpolate(&params, &data)?;
        let mut worst: f64 = 0.;
        for i in 0..samples {
            for k in 1..4 {
                let s = (i as f64 + k as f64 / 4.) / samples as f64;
                let p = curve.evaluate(s)?.point;
                let q = point(t0 + (t1 - t0) * s);
                worst = worst.max((p[0] - q[0]).hypot(p[1] - q[1]));
            }
        }
        if worst <= FLANK_FIT {
            return Ok((curve, worst));
        }
        if last {
            if worst > FLANK_FIT * 100. {
                return Err(invalid(format!(
                    "Gear flank could not be fitted within {:.0e} mm (achieved {worst:.2e})",
                    FLANK_FIT * 100.
                )));
            }
            return Ok((curve, worst));
        }
    }
    unreachable!("flank fit schedule is not empty")
}

fn reverse2(c: &Curve) -> Curve {
    let mut r = c.clone();
    r.control_points.reverse();
    r.weights.reverse();
    let [a, b] = c.domain();
    r.knots = c.knots.iter().rev().map(|k| a + b - k).collect();
    r
}

/// Closed loops of the outline (each a connected CCW or CW chain of 2D
/// curves, material on the left), the geometry report, and the fit.
pub(super) fn outline(spec: &GearSpec) -> Result<(Vec<Vec<Curve>>, GearGeometry)> {
    let teeth = spec.teeth as f64;
    if !(3..=256).contains(&spec.teeth) {
        return Err(invalid("Gear teeth must be 3..256"));
    }
    if !(0.05..=100.).contains(&spec.module) {
        return Err(invalid("Gear module must be 0.05..100 mm"));
    }
    if !(10.0..=35.).contains(&spec.pressure_angle_deg) {
        return Err(invalid("Gear pressure angle must be 10..35 degrees"));
    }
    if !(0.05..=1000.).contains(&spec.height) {
        return Err(invalid("Gear height must be 0.05..1000 mm"));
    }
    if !spec.helix_angle_deg.is_finite() || spec.helix_angle_deg.abs() >= 80. {
        return Err(invalid("Gear helix angle must be within -80..80 degrees"));
    }
    if spec.backlash < 0.
        || spec.backlash > spec.module / 2.
        || spec.clearance < 0.
        || spec.clearance > spec.module
    {
        return Err(invalid(
            "Gear backlash must be 0..module/2 and clearance 0..module",
        ));
    }
    let alpha = spec.pressure_angle_deg.to_radians();
    let pitch = spec.module * teeth / 2.;
    let base = pitch * alpha.cos();
    let (tip, root) = if spec.internal {
        (pitch - spec.module, pitch + spec.module + spec.clearance)
    } else {
        (pitch + spec.module, pitch - spec.module - spec.clearance)
    };
    let outside = if spec.internal {
        root + spec.rim_width
    } else {
        tip
    };
    if root <= 0. || outside > 10000. {
        return Err(invalid(
            "Gear root radius must be positive and outside radius at most 10000 mm",
        ));
    }
    if spec.internal {
        if tip <= base + 1e-9 {
            return Err(invalid(
                "Internal gear tip must stay outside the base circle",
            ));
        }
        if spec.bore != 0. {
            return Err(invalid(
                "Internal gear has a toothed opening; bore must be zero",
            ));
        }
        if spec.rim_width < spec.module / 4. {
            return Err(invalid(
                "Internal gear rim must be at least a quarter module",
            ));
        }
    } else if spec.bore < 0. || (spec.bore > 0. && spec.bore > 2. * root - spec.module / 2.) {
        return Err(invalid(
            "Gear bore must leave a quarter module inside the root circle",
        ));
    }
    let low = if spec.internal { tip } else { root };
    let high = if spec.internal { root } else { tip };
    let start = base.max(low);
    let inv_pitch = alpha.tan() - alpha;
    let half_pitch = PI / (2. * teeth)
        + if spec.internal {
            spec.backlash / (2. * pitch)
        } else {
            -spec.backlash / (2. * pitch)
        };
    let half = move |r: f64| -> f64 {
        let t = ((r / base).powi(2) - 1.).max(0.).sqrt();
        half_pitch + inv_pitch - (t - t.atan())
    };
    let low_half = half(start);
    let high_half = half(high);
    let step = TAU / teeth;
    if high_half <= 1e-6 || low_half >= step / 2. - 1e-6 {
        return Err(invalid(
            "Gear tooth profile collapses or overlaps; change teeth, pressure angle, backlash or clearance",
        ));
    }
    let t0 = ((start / base).powi(2) - 1.).max(0.).sqrt();
    let t1 = ((high / base).powi(2) - 1.).max(0.).sqrt();
    let mut flank_deviation: f64 = 0.;
    // One tooth outline, counter-clockwise, starting at the low radius on
    // the leading side.
    let mut tooth: Vec<Curve> = Vec::new();
    for i in 0..spec.teeth {
        let center = i as f64 * step + if spec.internal { step / 2. } else { 0. };
        if start > low + 1e-12 {
            tooth.push(line2(
                polar(low, center - low_half),
                polar(start, center - low_half),
            ));
        }
        let (rising, d1) = involute(base, t0, t1, center, -1., &half)?;
        let (falling, d2) = involute(base, t1, t0, center, 1., &half)?;
        flank_deviation = flank_deviation.max(d1).max(d2);
        tooth.push(rising);
        tooth.push(arc2(high, center - high_half, center + high_half)?);
        tooth.push(falling);
        if start > low + 1e-12 {
            tooth.push(line2(
                polar(start, center + low_half),
                polar(low, center + low_half),
            ));
        }
        tooth.push(arc2(low, center + low_half, center + step - low_half)?);
    }
    let mut loops = Vec::new();
    if spec.internal {
        // Material outside: the outer circle is the CCW boundary, the tooth
        // outline runs clockwise as a hole.
        loops.push(circle2(
            outside,
            arcs_clearing(outside, root).max(circle_arcs(spec.teeth)),
        )?);
        let mut hole: Vec<Curve> = tooth.iter().rev().map(reverse2).collect();
        hole.iter_mut().for_each(|_| {});
        loops.push(hole);
    } else {
        loops.push(tooth);
        if spec.bore > 0. {
            let mut hole: Vec<Curve> = circle2(spec.bore / 2., circle_arcs(spec.teeth))?
                .iter()
                .rev()
                .map(reverse2)
                .collect();
            hole.iter_mut().for_each(|_| {});
            loops.push(hole);
        }
    }
    let geometry = GearGeometry {
        pitch_radius: pitch,
        base_radius: base,
        tip_radius: tip,
        root_radius: root,
        outside_radius: outside,
        flank_deviation,
        sweep_deviation: 0.,
        face_count: 0,
    };
    Ok((loops, geometry))
}

// ---------------------------------------------------------------------------
// Solid
// ---------------------------------------------------------------------------

pub(super) fn rotate2(c: &Curve, theta: f64) -> Curve {
    let (s, co) = theta.sin_cos();
    let mut r = c.clone();
    for p in &mut r.control_points {
        let (x, y) = (p[0], p[1]);
        p[0] = co * x - s * y;
        p[1] = s * x + co * y;
    }
    r
}

pub(super) fn lift(c: &Curve, z: f64) -> Curve {
    let mut r = c.clone();
    for p in &mut r.control_points {
        p.push(z);
    }
    r
}

/// Wall surface of one outline curve between `z0` and `z1`, rotated by
/// `twist` radians over the height: ruled for zero twist, else a cubic loft
/// through `levels + 1` rotated copies, refined until the helical sweep is
/// matched within `SWEEP_FIT`. Returns the surface and its deviation.
pub(super) fn wall(c: &Curve, z0: f64, z1: f64, theta0: f64, twist: f64) -> Result<(Surface, f64)> {
    let h = z1 - z0;
    if twist.abs() < 1e-15 {
        let bottom = lift(&rotate2(c, theta0), z0);
        let top = lift(&rotate2(c, theta0), z1);
        return Ok((nurbs_core::surface::loft(&[bottom, top])?, 0.));
    }
    let schedule = [4usize, 8, 16, 24];
    for (step, &levels) in schedule.iter().enumerate() {
        let last = step + 1 == schedule.len();
        let params: Vec<f64> = (0..=levels).map(|i| i as f64 / levels as f64).collect();
        let copies: Vec<Curve> = params
            .iter()
            .map(|&v| lift(&rotate2(c, theta0 + twist * v), z0 + h * v))
            .collect();
        // Interpolate every control-point column across the levels with one
        // shared knot vector (constant weights along v keep the rational
        // curves exact at the levels).
        let n = c.control_points.len();
        let mut columns: Vec<Curve> = Vec::with_capacity(n);
        for i in 0..n {
            let data: Vec<Vec<f64>> = copies.iter().map(|k| k.control_points[i].clone()).collect();
            columns.push(interpolate(&params, &data)?);
        }
        let knots_v = columns[0].knots.clone();
        let degree_v = columns[0].degree;
        let m = columns[0].control_points.len();
        let surface = Surface {
            degree_u: c.degree,
            degree_v,
            knots_u: c.knots.clone(),
            knots_v,
            control_points: (0..n).map(|i| columns[i].control_points.clone()).collect(),
            weights: (0..n).map(|i| vec![c.weights[i]; m]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        surface.validate()?;
        let mut worst: f64 = 0.;
        let [u0, u1] = c.domain();
        for i in 0..=8 {
            let u = u0 + (u1 - u0) * i as f64 / 8.;
            let base_point = c.evaluate(u)?.point;
            for j in 0..levels {
                for k in 1..4 {
                    let v = (j as f64 + k as f64 / 4.) / levels as f64;
                    let p = surface.evaluate(u, v)?.point;
                    let (s, co) = (theta0 + twist * v).sin_cos();
                    let q = [
                        co * base_point[0] - s * base_point[1],
                        s * base_point[0] + co * base_point[1],
                        z0 + h * v,
                    ];
                    worst = worst.max(
                        ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2))
                            .sqrt(),
                    );
                }
            }
        }
        if worst <= SWEEP_FIT || last {
            if worst > SWEEP_FIT {
                return Err(invalid(format!(
                    "Gear helical wall could not be lofted within {SWEEP_FIT} mm (achieved {worst:.2e}); \
                     reduce the helix angle or the height"
                )));
            }
            return Ok((surface, worst));
        }
    }
    unreachable!("sweep fit schedule is not empty")
}

/// Iso-curve of a surface at u = 0 or u = 1 (the shared vertical edge).
pub(super) fn column(surface: &Surface, last: bool) -> Curve {
    let i = if last {
        surface.control_points.len() - 1
    } else {
        0
    };
    Curve {
        degree: surface.degree_v,
        knots: surface.knots_v.clone(),
        control_points: surface.control_points[i].clone(),
        weights: surface.weights[i].clone(),
        periodic: false,
    }
}

/// Planar cap over the bounding box of the outline curves at height `z`:
/// the surface and the affine pcurve map.
pub(super) fn cap(curves: &[&Curve], z: f64) -> (Surface, [f64; 4]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for c in curves {
        for p in &c.control_points {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![lo[0], lo[1], z], vec![lo[0], hi[1], z]],
            vec![vec![hi[0], lo[1], z], vec![hi[0], hi[1], z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    (surface, [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]])
}

pub(super) fn cap_pcurve(c: &Curve, map: [f64; 4]) -> Curve {
    let mut p = c.clone();
    for point in &mut p.control_points {
        let (x, y) = (point[0], point[1]);
        *point = vec![(x - map[0]) / map[2], (y - map[1]) / map[3]];
    }
    p
}

pub(super) fn unit_line(a: [f64; 2], b: [f64; 2]) -> Curve {
    line2(a, b)
}
