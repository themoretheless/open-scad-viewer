//! UV lifts of traced sphere/torus curves.
//!
//! Each traced cubic piece is cut at the patch seams of both surfaces
//! (seams located by bisection on the exact patch labels), every run is
//! sampled, inverted exactly into its patch UV (the certified stereographic
//! inversion of `CanonicalSphere::invert_uv`; on the torus the exact
//! 90-degree-arc parameter of the revolution and profile angles) and
//! interpolated by a cubic UV curve. The lift is accepted only when dense
//! samples of the mapped pcurve, evaluated on the authored face surface,
//! stay within `LIFT_TOLERANCE` of the other operand. The bound is sampled,
//! not certified.
use super::*;
use crate::plane_torus::arc_parameter;
use nurbs_core::foundation::interpolation::{Parameterization, interpolate_curve};

/// Sampled bound on the distance of mapped pcurve points from the other
/// surface.
pub const LIFT_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Patch {
    Sphere(usize),
    Torus(usize),
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

struct Context<'a> {
    sphere: &'a CanonicalSphere,
    torus: &'a CanonicalTorus,
    sphere_model: &'a Model,
    torus_model: &'a Model,
}

impl Context<'_> {
    fn label(&self, p: [f64; 3], torus_side: bool) -> Result<Patch> {
        if torus_side {
            Ok(Patch::Torus(torus_patch(self.torus, p)))
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

    fn uv(&self, patch: Patch, p: [f64; 3]) -> Result<[f64; 2]> {
        let uv = match patch {
            Patch::Sphere(f) => self.sphere.invert_uv(f, p).map(|uv| {
                // Seam points: snap rounding-scale negatives onto the patch.
                let uv = uv.map(|x| x.max(0.));
                let n = uv[0].hypot(uv[1]);
                if n > 1. { uv.map(|x| x / n) } else { uv }
            }),
            Patch::Torus(f) => torus_uv(self.torus, f, p),
        };
        uv.ok_or_else(|| {
            Error::new(
                "BREP_INTERSECTION_LIFT_REFUSED",
                "traced point does not invert into its patch",
            )
        })
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
                Patch::Sphere(_) => torus_distance(self.torus, p),
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

fn point(curve: &Curve, t: f64) -> Result<[f64; 3]> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1], p[2]])
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
                let deviation = ctx.deviation(patch, &curve)?;
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

/// Sphere and torus UV lifts of one traced loop and their worst sampled
/// deviation from the other surface.
pub(super) fn lift_traced(
    sphere: &CanonicalSphere,
    torus: &CanonicalTorus,
    sphere_model: &Model,
    torus_model: &Model,
    pieces: &[Curve],
) -> Result<(Vec<SpherePatchCircle>, Vec<TorusPatchCurve>, f64)> {
    let ctx = Context {
        sphere,
        torus,
        sphere_model,
        torus_model,
    };
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
        t.into_iter()
            .map(|(patch, arcs)| TorusPatchCurve { patch, arcs })
            .collect(),
        ds.max(dt),
    ))
}
