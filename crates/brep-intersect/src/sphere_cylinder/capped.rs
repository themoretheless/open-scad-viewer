//! Off-axis sphere reaching a cap plane of the canonical cylinder.
//!
//! The wall contact is traced on the infinite cylinder (where tracing and
//! its completeness certificate do not care about the caps) and clipped to
//! the slab between the cap planes; a cap contact is the exact circle where
//! the sphere meets a cap plane, clipped to the cap disk. The clip points
//! of both kinds are the circle/rim crossings, so the kept wall arcs and cap
//! arcs join into closed cycles. Every cycle is returned as cubic pieces
//! (wall arcs resampled from the certified trace, cap arcs sampled from the
//! exact circle) with exactly shared ends; the caller lifts and certifies
//! them like any traced loop. Tangent crossings (a wall arc touching a cap
//! plane, a cap circle touching the rim) are refusals.
use super::*;
use nurbs_core::foundation::interpolation::{Parameterization, interpolate_curve};

type P = [f64; 3];

fn refuse(message: &str) -> nurbs_core::Error {
    nurbs_core::Error::new("BREP_INTERSECTION_TRACE_REFUSED", message)
}

fn point(curve: &Curve, t: f64) -> Result<P> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1], p[2]])
}

/// One kept arc: a sampler from parameter in [0, 1] to a point, with its
/// end points.
struct Arc {
    samples: Vec<P>,
}

fn fit(samples: &[P]) -> Result<Curve> {
    interpolate_curve(samples, 3, Parameterization::ChordLength, None)
}

/// Split a long arc into pieces of at most `per` samples so each fitted
/// cubic stays small; consecutive pieces share their end sample exactly.
fn pieces_of(arc: &Arc, per: usize) -> Result<Vec<Curve>> {
    let n = arc.samples.len();
    let count = (n - 1).div_ceil(per - 1).max(1);
    let mut out = Vec::with_capacity(count);
    for k in 0..count {
        let a = k * (n - 1) / count;
        let b = (k + 1) * (n - 1) / count;
        let mut curve = fit(&arc.samples[a..=b])?;
        // Exact shared ends.
        let last = curve.control_points.len() - 1;
        curve.control_points[0] = arc.samples[a].to_vec();
        curve.control_points[last] = arc.samples[b].to_vec();
        out.push(curve);
    }
    Ok(out)
}

/// Closed cycles of a sphere/cylinder contact reaching the caps.
pub(super) fn capped_cycles(
    sphere: &CanonicalSphere,
    cylinder: &CanonicalCylinder,
    loops: &[Vec<Curve>],
) -> Result<Vec<Vec<Curve>>> {
    let axial = |p: P| dot(sub(p, cylinder.center), cylinder.axis);
    let half = cylinder.half_height;
    // 1. Wall arcs inside the slab, from each traced loop.
    let mut wall: Vec<Arc> = Vec::new();
    let mut closed: Vec<Vec<Curve>> = Vec::new();
    for pieces in loops {
        // Dense samples along the loop with their (piece, t).
        let mut pts: Vec<P> = Vec::new();
        for piece in pieces {
            let [lo, hi] = piece.domain();
            let n = 256;
            for k in 0..n {
                pts.push(point(piece, lo + (hi - lo) * k as f64 / n as f64)?);
            }
        }
        let inside = |p: P| axial(p).abs() < half;
        if pts.iter().all(|&p| inside(p)) {
            closed.push(pieces.clone());
            continue;
        }
        if pts.iter().all(|&p| !inside(p)) {
            continue;
        }
        // Bisection on the plane crossing between two samples, along the
        // straight chord refined onto the traced curve: locate the crossing
        // on the piece parameter instead.
        let mut flat: Vec<(usize, f64)> = Vec::new();
        for (i, piece) in pieces.iter().enumerate() {
            let [lo, hi] = piece.domain();
            for k in 0..256 {
                flat.push((i, lo + (hi - lo) * k as f64 / 256.));
            }
        }
        let start = (0..flat.len()).find(|&k| !inside(pts[k])).expect("some sample outside");
        let n = flat.len();
        let mut current: Option<Vec<P>> = None;
        for step in 1..=n {
            let (k0, k1) = ((start + step - 1) % n, (start + step) % n);
            let (in0, in1) = (inside(pts[k0]), inside(pts[k1]));
            if in0 != in1 {
                // The crossing lies on the piece of k0 between its two
                // parameters (or at the shared piece end).
                let (i, t0) = flat[k0];
                let t1 = if flat[k1].0 == i { flat[k1].1 } else { pieces[i].domain()[1] };
                let q = if axial(pts[k0]).abs().max(axial(pts[k1]).abs()) > 0. {
                    if axial(pts[k0]) + axial(pts[k1]) > 0. { half } else { -half }
                } else {
                    half
                };
                let f = |t: f64| -> Result<f64> { Ok(axial(point(&pieces[i], t)?) - q) };
                let (mut a, mut b) = (t0, t1);
                let fa = f(a)?;
                if fa * f(b)? > 0. {
                    return Err(refuse("cap crossing could not be bracketed"));
                }
                for _ in 0..80 {
                    let m = 0.5 * (a + b);
                    if f(m)? * fa > 0. {
                        a = m;
                    } else {
                        b = m;
                    }
                }
                let cross = point(&pieces[i], 0.5 * (a + b))?;
                if in1 {
                    current = Some(vec![cross]);
                } else if let Some(mut arc) = current.take() {
                    arc.push(cross);
                    wall.push(Arc { samples: arc });
                }
            } else if in1 && let Some(arc) = current.as_mut() {
                arc.push(pts[k1]);
            }
        }
        if current.is_some() {
            return Err(refuse("wall arc did not close at a cap plane"));
        }
    }
    // A wall arc grazing a cap plane would show as a sample near it.
    for arc in &wall {
        for &p in &arc.samples[1..arc.samples.len() - 1] {
            if (axial(p).abs() - half).abs() < 1e-6 {
                return Err(refuse("wall arc touches a cap plane"));
            }
        }
    }
    // 2. Cap arcs: the sphere/cap-plane circle inside the disk.
    let mut caps: Vec<Arc> = Vec::new();
    let offset = sub(sphere.center, cylinder.center);
    let s = dot(offset, cylinder.axis);
    let perp = sub(offset, cylinder.axis.map(|x| x * s));
    let ex = cylinder.frame[0];
    let ey = cylinder.frame[1];
    let c2 = [dot(perp, ex), dot(perp, ey)];
    let d = c2[0].hypot(c2[1]);
    let big_r = cylinder.radius;
    for q in [-half, half] {
        let h = q - s;
        let rho2 = sphere.radius * sphere.radius - h * h;
        if rho2 <= 0. {
            continue;
        }
        let rho = rho2.sqrt();
        let at = |phi: f64| -> P {
            let (sn, cs) = phi.sin_cos();
            let x = c2[0] + rho * cs;
            let y = c2[1] + rho * sn;
            std::array::from_fn(|k| cylinder.center[k] + q * cylinder.axis[k] + x * ex[k] + y * ey[k])
        };
        if (d + rho - big_r).abs() < 1e-7 || (d - rho - big_r).abs() < 1e-7 || (rho - d - big_r).abs() < 1e-7 {
            return Err(refuse("cap circle touches the rim"));
        }
        if d + rho < big_r {
            // Whole circle on the cap: its own closed cycle.
            let mut samples: Vec<P> = (0..=512).map(|k| at(std::f64::consts::TAU * k as f64 / 512.)).collect();
            samples[512] = samples[0];
            closed.push(pieces_of(&Arc { samples }, 65)?);
            continue;
        }
        if d - rho > big_r || rho - d > big_r {
            continue;
        }
        // Rim crossings: |c + rho e(phi)| = R  <=>  cos(phi - phi0) = k.
        let phi0 = c2[1].atan2(c2[0]);
        let k = (big_r * big_r - d * d - rho * rho) / (2. * d * rho);
        let delta = k.clamp(-1., 1.).acos();
        // Inside the disk is the side facing the axis: phi0 + delta .. phi0 + 2pi - delta.
        let (a, b) = (phi0 + delta, phi0 + std::f64::consts::TAU - delta);
        let n = 256;
        let samples: Vec<P> = (0..=n).map(|i| at(a + (b - a) * i as f64 / n as f64)).collect();
        caps.push(Arc { samples });
    }
    // 3. Join arcs at their shared rim points into cycles.
    let mut arcs: Vec<(Arc, bool)> = wall.into_iter().map(|a| (a, false)).collect();
    arcs.extend(caps.into_iter().map(|a| (a, false)));
    let same = |a: P, b: P| {
        let d = sub(a, b);
        dot(d, d).sqrt() < 1e-5
    };
    while let Some(first) = arcs.iter().position(|(_, used)| !used) {
        arcs[first].1 = true;
        let mut cycle = pieces_of(&arcs[first].0, 65)?;
        let head = arcs[first].0.samples[0];
        let mut tail = *arcs[first].0.samples.last().unwrap();
        let mut guard = 0;
        while !same(tail, head) {
            guard += 1;
            if guard > arcs.len() + 1 {
                return Err(refuse("contact arcs do not close into cycles"));
            }
            let Some((next, forward)) = arcs.iter().enumerate().find_map(|(i, (arc, used))| {
                if *used {
                    return None;
                }
                if same(arc.samples[0], tail) {
                    Some((i, true))
                } else if same(*arc.samples.last().unwrap(), tail) {
                    Some((i, false))
                } else {
                    None
                }
            }) else {
                return Err(refuse("a contact arc has no continuation"));
            };
            arcs[next].1 = true;
            let mut samples = arcs[next].0.samples.clone();
            if !forward {
                samples.reverse();
            }
            // Snap the join exactly onto the previous end.
            samples[0] = tail;
            tail = *samples.last().unwrap();
            cycle.extend(pieces_of(&Arc { samples }, 65)?);
        }
        // Close exactly.
        let last = cycle.len() - 1;
        let lc = cycle[last].control_points.len() - 1;
        cycle[last].control_points[lc] = cycle[0].control_points[0].clone();
        closed.push(cycle);
    }
    Ok(closed)
}
