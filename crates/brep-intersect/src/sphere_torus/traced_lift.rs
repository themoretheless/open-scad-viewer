//! UV lifts of traced sphere/torus curves.
//!
//! Each traced cubic piece is cut at the patch seams of both surfaces
//! (seams located by bisection on the exact patch labels), every run is
//! sampled, inverted exactly into its patch UV (the certified stereographic
//! inversion of `CanonicalSphere::invert_uv`; on the torus the exact
//! 90-degree-arc parameter of the revolution and profile angles) and
//! interpolated by a cubic UV curve. The lift is accepted only when the
//! image of the pcurve on the authored face surface is certified within
//! `LIFT_TOLERANCE` of the other operand: the patch composed with each cubic
//! UV span is a rational polynomial, the other surface's implicit equation
//! in homogeneous form becomes a polynomial in the Bernstein basis, and
//! outward interval bounds with subdivision turn it into a distance bound.
//! Dense samples serve only as a cheap screen before the certificate.
use super::*;
use crate::plane_torus::arc_parameter;
use nurbs_core::foundation::interpolation::{Parameterization, interpolate_curve};

/// Certified bound on the distance of the pcurve images from the other
/// surface.
pub const LIFT_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Patch {
    Sphere(usize),
    /// A face of the other operand (torus, cylinder or cone).
    Torus(usize),
}

/// The non-sphere operand of a traced pair.
#[derive(Clone, Copy)]
pub(crate) enum Other<'a> {
    Torus(&'a CanonicalTorus),
    Cylinder(&'a crate::sphere_cylinder::CanonicalCylinder),
    Cone(&'a brep_core::intersections::CanonicalCone),
}

/// Unwrap an angle into quadrant `q` and map it to the exact 90-degree arc
/// parameter.
fn arc_local(angle: f64, q: usize) -> f64 {
    let mut a = angle - q as f64 * QUARTER;
    if a > TAU - QUARTER {
        a -= TAU;
    } else if a < -QUARTER {
        a += TAU;
    }
    arc_parameter(a.clamp(0., QUARTER)).clamp(0., 1.)
}

impl Other<'_> {
    /// Origin, in-plane frame and axis of the revolution chart.
    fn chart(&self) -> ([f64; 3], [[f64; 3]; 2], [f64; 3]) {
        match self {
            Other::Torus(t) => (t.center, t.frame, t.axis),
            Other::Cylinder(c) => (c.center, c.frame, c.axis),
            Other::Cone(c) => (c.bottom, c.frame, c.axis),
        }
    }
    /// Revolution angle in [0, TAU), axial coordinate and axis distance.
    fn cylindrical(&self, p: [f64; 3]) -> (f64, f64, f64) {
        let (o, f, axis) = self.chart();
        let d = sub(p, o);
        let (x, y) = (dot(d, f[0]), dot(d, f[1]));
        (y.atan2(x).rem_euclid(TAU), dot(d, axis), x.hypot(y))
    }
    pub(crate) fn patch(&self, p: [f64; 3]) -> Option<usize> {
        match self {
            Other::Torus(t) => Some(torus_patch(t, p)),
            Other::Cylinder(c) => {
                let (rev, z, rho) = self.cylindrical(p);
                // The nearer of the wall and the cap planes owns the point.
                if (rho - c.radius).abs() <= (z.abs() - c.half_height).abs() {
                    (z.abs() <= c.half_height + 1e-9).then(|| c.sides[quadrant(rev)])
                } else {
                    (rho <= c.radius + 1e-9).then(|| c.caps[usize::from(z > 0.)])
                }
            }
            Other::Cone(c) => {
                let (rev, z, rho) = self.cylindrical(p);
                let wall = (rho - c.r_bottom - c.slope * z).abs() / c.slope.hypot(1.);
                let plane = z.abs().min((z - c.height).abs());
                if wall <= plane {
                    (-1e-9..=c.height + 1e-9).contains(&z).then(|| c.sides[quadrant(rev)])
                } else {
                    let slot = usize::from(z > 0.5 * c.height);
                    let disk = if slot == 0 { c.r_bottom } else { c.r_top };
                    if rho <= disk + 1e-9 { c.caps[slot] } else { None }
                }
            }
        }
    }
    pub(crate) fn uv(&self, face: usize, p: [f64; 3]) -> Option<[f64; 2]> {
        match self {
            Other::Torus(t) => torus_uv(t, face, p),
            Other::Cylinder(c) => {
                if let Some(i) = c.sides.iter().position(|&f| f == face) {
                    let (rev, z, _) = self.cylindrical(p);
                    return Some([arc_local(rev, i), ((z + c.half_height) / (2. * c.half_height)).clamp(0., 1.)]);
                }
                c.caps.contains(&face).then(|| {
                    let d = sub(p, c.center);
                    let x = dot(d, c.frame[0]) / c.radius;
                    let y = dot(d, c.frame[1]) / c.radius;
                    [(0.5 * (x + 1.)).clamp(0., 1.), (0.5 * (y + 1.)).clamp(0., 1.)]
                })
            }
            Other::Cone(c) => {
                if let Some(i) = c.sides.iter().position(|&f| f == face) {
                    let (rev, z, _) = self.cylindrical(p);
                    return Some([arc_local(rev, i), (z / c.height).clamp(0., 1.)]);
                }
                let slot = c.caps.iter().position(|&f| f == Some(face))?;
                let disk = if slot == 0 { c.r_bottom } else { c.r_top };
                let d = sub(p, c.bottom);
                let x = dot(d, c.frame[0]) / disk;
                let y = dot(d, c.frame[1]) / disk;
                Some([(0.5 * (x + 1.)).clamp(0., 1.), (0.5 * (y + 1.)).clamp(0., 1.)])
            }
        }
    }
    fn distance(&self, p: [f64; 3]) -> f64 {
        match self {
            Other::Torus(t) => torus_distance(t, p),
            Other::Cylinder(c) => {
                let (_, z, rho) = self.cylindrical(p);
                (rho - c.radius).max(z.abs() - c.half_height).abs()
            }
            Other::Cone(c) => {
                let (_, z, rho) = self.cylindrical(p);
                let wall = (rho - c.r_bottom - c.slope * z) / c.slope.hypot(1.);
                wall.max(-z).max(z - c.height).abs()
            }
        }
    }
    /// Implicit equation of the surface carrying `face` (a cap plane for a
    /// cylinder cap, the curved surface otherwise).
    fn implicit_for(&self, face: usize) -> Box<dyn crate::implicit_trace::Implicit> {
        if let Other::Cylinder(c) = self
            && let Some(slot) = c.caps.iter().position(|&f| f == face)
        {
            let sign = if slot == 0 { -1. } else { 1. };
            return Box::new(crate::implicit_trace::PlaneImplicit {
                origin: std::array::from_fn(|k| c.center[k] + sign * c.half_height * c.axis[k]),
                normal: c.axis,
            });
        }
        if let Other::Cone(c) = self
            && let Some(slot) = c.caps.iter().position(|&f| f == Some(face))
        {
            let z = if slot == 0 { 0. } else { c.height };
            return Box::new(crate::implicit_trace::PlaneImplicit {
                origin: std::array::from_fn(|k| c.bottom[k] + z * c.axis[k]),
                normal: c.axis,
            });
        }
        self.implicit()
    }
    pub(crate) fn implicit(&self) -> Box<dyn crate::implicit_trace::Implicit> {
        let (o, f, axis) = self.chart();
        let frame = [f[0], f[1], axis];
        match self {
            Other::Torus(t) => Box::new(crate::implicit_trace::TorusImplicit {
                center: o,
                frame,
                major: t.major,
                minor: t.minor,
            }),
            Other::Cylinder(c) => Box::new(crate::implicit_trace::CylinderImplicit {
                center: o,
                frame,
                radius: c.radius,
            }),
            Other::Cone(c) => Box::new(crate::implicit_trace::ConeImplicit {
                bottom: o,
                frame,
                r_bottom: c.r_bottom,
                slope: c.slope,
            }),
        }
    }
}

/// The sphere patch whose exact inversion lands in its quarter disk; at a
/// seam, the first one (the label is only used to cut runs).
fn sphere_patch(sphere: &CanonicalSphere, faces: usize, p: [f64; 3]) -> Option<usize> {
    let violation = |f: usize| {
        sphere
            .invert_uv(f, p)
            .map(|uv| (-uv[0]).max(-uv[1]).max(uv[0].hypot(uv[1]) - 1.).max(0.))
    };
    (0..faces)
        .filter_map(|f| violation(f).map(|v| (f, v)))
        .find(|&(_, v)| v <= 1e-12)
        .map(|(f, _)| f)
}

/// Revolution and profile angles of a point in [0, TAU).
fn torus_angles(torus: &CanonicalTorus, p: [f64; 3]) -> (f64, f64) {
    let d = sub(p, torus.center);
    let x = dot(d, torus.frame[0]);
    let y = dot(d, torus.frame[1]);
    let z = dot(d, torus.axis);
    (
        y.atan2(x).rem_euclid(TAU),
        z.atan2(x.hypot(y) - torus.major).rem_euclid(TAU),
    )
}

fn quadrant(angle: f64) -> usize {
    ((angle / QUARTER).floor() as usize).min(3)
}

fn torus_patch(torus: &CanonicalTorus, p: [f64; 3]) -> usize {
    let (rev, prof) = torus_angles(torus, p);
    torus.patches[quadrant(prof)][quadrant(rev)]
}

fn torus_uv(torus: &CanonicalTorus, patch: usize, p: [f64; 3]) -> Option<[f64; 2]> {
    let (j, i) = (0..16)
        .map(|k| (k / 4, k % 4))
        .find(|&(j, i)| torus.patches[j][i] == patch)?;
    let (rev, prof) = torus_angles(torus, p);
    // Unwrap the angle into this patch's quadrant (seam points may report
    // the neighbouring quadrant or wrap through TAU).
    let local = |angle: f64, q: usize| {
        let mut a = angle - q as f64 * QUARTER;
        if a > TAU - QUARTER {
            a -= TAU;
        } else if a < -QUARTER {
            a += TAU;
        }
        arc_parameter(a.clamp(0., QUARTER)).clamp(0., 1.)
    };
    Some([local(rev, i), local(prof, j)])
}

pub(crate) struct Context<'a> {
    sphere: &'a CanonicalSphere,
    other: Other<'a>,
    sphere_model: &'a Model,
    torus_model: &'a Model,
    sphere_implicit: crate::implicit_trace::SphereImplicit,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        sphere: &'a CanonicalSphere,
        other: Other<'a>,
        sphere_model: &'a Model,
        other_model: &'a Model,
    ) -> Self {
        Context {
            sphere,
            other,
            sphere_model,
            torus_model: other_model,
            sphere_implicit: crate::implicit_trace::SphereImplicit {
                center: sphere.center,
                radius: sphere.radius,
            },
        }
    }
}

impl Context<'_> {
    pub(crate) fn label(&self, p: [f64; 3], torus_side: bool) -> Result<Patch> {
        if torus_side {
            self.other
                .patch(p)
                .map(Patch::Torus)
                .ok_or_else(|| Error::new("BREP_INTERSECTION_LIFT_REFUSED", "traced point lies on no side patch"))
        } else {
            sphere_patch(self.sphere, self.sphere_model.faces.len(), p)
                .map(Patch::Sphere)
                .ok_or_else(|| {
                    Error::new(
                        "BREP_INTERSECTION_LIFT_REFUSED",
                        "traced point lies on no sphere patch",
                    )
                })
        }
    }

    pub(crate) fn uv(&self, patch: Patch, p: [f64; 3]) -> Result<[f64; 2]> {
        let uv = match patch {
            Patch::Sphere(f) => self.sphere.invert_uv(f, p).map(|uv| {
                // Seam points: snap rounding-scale negatives onto the patch.
                let uv = uv.map(|x| x.max(0.));
                let n = uv[0].hypot(uv[1]);
                if n > 1. { uv.map(|x| x / n) } else { uv }
            }),
            Patch::Torus(f) => self.other.uv(f, p),
        };
        uv.ok_or_else(|| {
            Error::new(
                "BREP_INTERSECTION_LIFT_REFUSED",
                "traced point does not invert into its patch",
            )
        })
    }

    /// Certified distance bound of the pcurve image from the other surface.
    pub(crate) fn certify(&self, patch: Patch, uv_curve: &Curve) -> Result<Option<f64>> {
        self.certify_against(patch, uv_curve, None)
    }
    /// Certify a pcurve image against the other surface; for a sphere
    /// pcurve, `other_face` selects which surface of the other operand the
    /// run lies on (a cylinder wall or cap plane).
    pub(crate) fn certify_against(
        &self,
        patch: Patch,
        uv_curve: &Curve,
        other_face: Option<usize>,
    ) -> Result<Option<f64>> {
        match patch {
            Patch::Sphere(f) => {
                let other = match other_face {
                    Some(face) => self.other.implicit_for(face),
                    None => self.other.implicit(),
                };
                crate::implicit_trace::certify_lift(&self.sphere_model.faces[f].surface, uv_curve, other.as_ref(), LIFT_TOLERANCE)
            }
            Patch::Torus(f) => crate::implicit_trace::certify_lift(
                &self.torus_model.faces[f].surface,
                uv_curve,
                &self.sphere_implicit,
                LIFT_TOLERANCE,
            ),
        }
    }

    /// Worst distance of mapped pcurve samples from the other surface.
    fn deviation(&self, patch: Patch, uv_curve: &Curve) -> Result<f64> {
        let (model, f) = match patch {
            Patch::Sphere(f) => (self.sphere_model, f),
            Patch::Torus(f) => (self.torus_model, f),
        };
        let surface = &model.faces[f].surface;
        let [lo, hi] = uv_curve.domain();
        let mut worst = 0_f64;
        for k in 0..=256 {
            let uv = uv_curve.evaluate(lo + (hi - lo) * k as f64 / 256.)?.point;
            let p = surface
                .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))?
                .point;
            let p = [p[0], p[1], p[2]];
            let other = match patch {
                Patch::Sphere(_) => self.other.distance(p),
                Patch::Torus(_) => {
                    let d = sub(p, self.sphere.center);
                    (d[0].hypot(d[1]).hypot(d[2]) - self.sphere.radius).abs()
                }
            };
            worst = worst.max(other);
        }
        Ok(worst)
    }
}

/// Euclidean distance of a point from the torus surface.
fn torus_distance(torus: &CanonicalTorus, p: [f64; 3]) -> f64 {
    let d = sub(p, torus.center);
    let z = dot(d, torus.axis);
    let rho = dot(d, torus.frame[0]).hypot(dot(d, torus.frame[1]));
    ((rho - torus.major).hypot(z) - torus.minor).abs()
}

pub(crate) fn point(curve: &Curve, t: f64) -> Result<[f64; 3]> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1], p[2]])
}

/// Parameters where `piece` crosses a patch seam of one surface, framed by
/// the piece's domain ends.
pub(crate) fn seam_breaks(ctx: &Context, piece: &Curve, torus_side: bool) -> Result<Vec<f64>> {
    let [lo, hi] = piece.domain();
    // Seam breakpoints by bisection between samples of different labels.
    const SAMPLES: usize = 512;
    // Seam slivers shorter than this (a piece starting or ending on a
    // seam) merge into the neighbouring run; their points are clamped
    // onto that patch at rounding scale.
    const MERGE: f64 = 1e-9;
    let mut breaks = vec![lo];
    let mut prev_t = lo;
    let mut prev = ctx.label(point(piece, lo)?, torus_side)?;
    for k in 1..=SAMPLES {
        let t = lo + (hi - lo) * k as f64 / SAMPLES as f64;
        let label = ctx.label(point(piece, t)?, torus_side)?;
        if label != prev {
            let (mut a, mut b) = (prev_t, t);
            for _ in 0..60 {
                let m = 0.5 * (a + b);
                if ctx.label(point(piece, m)?, torus_side)? == prev {
                    a = m;
                } else {
                    b = m;
                }
            }
            let cut = 0.5 * (a + b);
            if cut - breaks.last().unwrap() > MERGE * (hi - lo) {
                breaks.push(cut);
            }
        }
        prev = label;
        prev_t = t;
    }
    if breaks.len() > 1 && hi - breaks.last().unwrap() <= MERGE * (hi - lo) {
        breaks.pop();
    }
    breaks.push(hi);
    Ok(breaks)
}

/// Lifts of one traced loop onto one surface, grouped by face, plus the
/// worst sampled deviation.
fn lift_side(
    ctx: &Context,
    pieces: &[Curve],
    torus_side: bool,
) -> Result<(Vec<(usize, Vec<Curve>)>, f64)> {
    let mut grouped: Vec<(usize, Vec<Curve>)> = Vec::new();
    let mut worst = 0_f64;
    for piece in pieces {
        let breaks = seam_breaks(ctx, piece, torus_side)?;
        for w in breaks.windows(2) {
            let (a, b) = (w[0], w[1]);
            let patch = ctx.label(point(piece, 0.5 * (a + b))?, torus_side)?;
            let mut count = 17;
            let (curve, deviation) = loop {
                let uvs = (0..count)
                    .map(|k| {
                        let t = a + (b - a) * k as f64 / (count - 1) as f64;
                        let uv = ctx.uv(patch, point(piece, t)?)?;
                        Ok([uv[0], uv[1], 0.])
                    })
                    .collect::<Result<Vec<_>>>()?;
                let fitted = interpolate_curve(&uvs, 3, Parameterization::ChordLength, None)?;
                let curve = Curve {
                    control_points: fitted
                        .control_points
                        .iter()
                        .map(|c| vec![c[0], c[1]])
                        .collect(),
                    ..fitted
                };
                // Cheap sampled screen first, then the certificate.
                let mut deviation = ctx.deviation(patch, &curve)?;
                if deviation <= LIFT_TOLERANCE {
                    let other_face = match (patch, ctx.label(point(piece, 0.5 * (a + b))?, true)) {
                        (Patch::Sphere(_), Ok(Patch::Torus(f))) => Some(f),
                        _ => None,
                    };
                    deviation = ctx.certify_against(patch, &curve, other_face)?.unwrap_or(f64::INFINITY);
                }
                if deviation <= LIFT_TOLERANCE || count >= 129 {
                    break (curve, deviation);
                }
                count = 2 * count - 1;
            };
            if deviation > LIFT_TOLERANCE {
                return Err(Error::new(
                    "BREP_INTERSECTION_LIFT_REFUSED",
                    "traced curve UV lift exceeds its tolerance",
                ));
            }
            worst = worst.max(deviation);
            let face = match patch {
                Patch::Sphere(f) | Patch::Torus(f) => f,
            };
            match grouped.iter_mut().find(|(f, _)| *f == face) {
                Some((_, arcs)) => arcs.push(curve),
                None => grouped.push((face, vec![curve])),
            }
        }
    }
    Ok((grouped, worst))
}

/// Sphere and other-surface UV lifts of one traced loop, grouped by face,
/// and their worst certified deviation.
pub(crate) fn lift_traced_with(
    sphere: &CanonicalSphere,
    other: Other<'_>,
    sphere_model: &Model,
    other_model: &Model,
    pieces: &[Curve],
) -> Result<(Vec<SpherePatchCircle>, Vec<(usize, Vec<Curve>)>, f64)> {
    let ctx = Context::new(sphere, other, sphere_model, other_model);
    let (s, ds) = lift_side(&ctx, pieces, false)?;
    let (t, dt) = lift_side(&ctx, pieces, true)?;
    Ok((
        s.into_iter()
            .map(|(patch, arcs)| SpherePatchCircle {
                patch,
                arcs,
                implicit: None,
            })
            .collect(),
        t,
        ds.max(dt),
    ))
}

/// Sphere and torus UV lifts of one traced loop and their worst certified
/// deviation from the other surface.
pub(super) fn lift_traced(
    sphere: &CanonicalSphere,
    torus: &CanonicalTorus,
    sphere_model: &Model,
    torus_model: &Model,
    pieces: &[Curve],
) -> Result<(Vec<SpherePatchCircle>, Vec<TorusPatchCurve>, f64)> {
    let (s, t, d) = lift_traced_with(sphere, Other::Torus(torus), sphere_model, torus_model, pieces)?;
    Ok((
        s,
        t.into_iter()
            .map(|(patch, arcs)| TorusPatchCurve { patch, arcs })
            .collect(),
        d,
    ))
}

/// UV lifts of one traced loop onto surface `this` (a torus, cylinder or
/// cone), cut at its patch seams, each pcurve image certified within
/// `LIFT_TOLERANCE` of the opposite surface `that`.
fn lift_onto(
    this: Other<'_>,
    this_model: &Model,
    that: &dyn crate::implicit_trace::Implicit,
    pieces: &[Curve],
) -> Result<(Vec<(usize, Vec<Curve>)>, f64)> {
    let refuse = |m: &str| Error::new("BREP_INTERSECTION_LIFT_REFUSED", m.to_string());
    let label = |p: [f64; 3]| this.patch(p).ok_or_else(|| refuse("traced point lies on no patch"));
    let mut grouped: Vec<(usize, Vec<Curve>)> = Vec::new();
    let mut worst = 0_f64;
    for piece in pieces {
        let [lo, hi] = piece.domain();
        let mut breaks = vec![lo];
        let (mut prev_t, mut prev) = (lo, label(point(piece, lo)?)?);
        for k in 1..=512 {
            let t = lo + (hi - lo) * k as f64 / 512.;
            let l = label(point(piece, t)?)?;
            if l != prev {
                let (mut a, mut b) = (prev_t, t);
                for _ in 0..60 {
                    let m = 0.5 * (a + b);
                    if label(point(piece, m)?)? == prev {
                        a = m;
                    } else {
                        b = m;
                    }
                }
                let cut = 0.5 * (a + b);
                if cut - breaks.last().unwrap() > 1e-9 * (hi - lo) {
                    breaks.push(cut);
                }
            }
            prev = l;
            prev_t = t;
        }
        if breaks.len() > 1 && hi - breaks.last().unwrap() <= 1e-9 * (hi - lo) {
            breaks.pop();
        }
        breaks.push(hi);
        // Runs between seams; a run whose fit does not certify is halved.
        let mut runs: Vec<(f64, f64, usize)> = breaks.windows(2).map(|w| (w[0], w[1], 0)).collect();
        runs.reverse();
        while let Some((a, b, depth)) = runs.pop() {
            let face = label(point(piece, 0.5 * (a + b))?)?;
            let surface = &this_model.faces[face].surface;
            let mut count = 17;
            let (curve, deviation) = loop {
                let uvs = (0..count)
                    .map(|k| {
                        let t = a + (b - a) * k as f64 / (count - 1) as f64;
                        let uv = this
                            .uv(face, point(piece, t)?)
                            .ok_or_else(|| refuse("traced point does not invert into its patch"))?;
                        Ok([uv[0], uv[1], 0.])
                    })
                    .collect::<Result<Vec<_>>>()?;
                let fitted = interpolate_curve(&uvs, 3, Parameterization::ChordLength, None)?;
                let curve = Curve {
                    control_points: fitted.control_points.iter().map(|c| vec![c[0], c[1]]).collect(),
                    ..fitted
                };
                let deviation = crate::implicit_trace::certify_lift(surface, &curve, that, LIFT_TOLERANCE)?
                    .unwrap_or(f64::INFINITY);
                if deviation <= LIFT_TOLERANCE || count >= 129 {
                    break (curve, deviation);
                }
                count = 2 * count - 1;
            };
            if deviation > LIFT_TOLERANCE {
                if depth < 4 {
                    let m = 0.5 * (a + b);
                    runs.push((m, b, depth + 1));
                    runs.push((a, m, depth + 1));
                    continue;
                }
                return Err(refuse("traced curve UV lift exceeds its tolerance"));
            }
            worst = worst.max(deviation);
            match grouped.iter_mut().find(|(f, _)| *f == face) {
                Some((_, arcs)) => arcs.push(curve),
                None => grouped.push((face, vec![curve])),
            }
        }
    }
    Ok((grouped, worst))
}

/// UV lifts of a traced loop onto two surfaces of revolution, grouped by
/// face, and the worst certified deviation.
pub(crate) fn lift_pair(
    a: Other<'_>,
    a_model: &Model,
    b: Other<'_>,
    b_model: &Model,
    pieces: &[Curve],
) -> Result<(Vec<(usize, Vec<Curve>)>, Vec<(usize, Vec<Curve>)>, f64)> {
    let (la, da) = lift_onto(a, a_model, b.implicit().as_ref(), pieces)?;
    let (lb, db) = lift_onto(b, b_model, a.implicit().as_ref(), pieces)?;
    Ok((la, lb, da.max(db)))
}
