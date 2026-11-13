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

/// The slab of a capped solid of revolution: axial coordinates measured
/// from `origin` along `frame[2]`, and per cap plane its axial position and
/// disk radius.
pub(crate) struct Slab {
    pub(crate) origin: P,
    /// In-plane x, in-plane y, axis.
    pub(crate) frame: [P; 3],
    pub(crate) planes: [(f64, f64); 2],
}

/// What meets the cap planes.
pub(crate) enum CapContacts<'a> {
    /// A sphere: its cap-plane sections are exact circles.
    Sphere(&'a CanonicalSphere),
    /// Traced loops of the other surface with each cap plane (bottom, top).
    Traced(&'a [Vec<Vec<Curve>>; 2]),
}

/// Split traced loops at the zero set of `h` (negative inside): closed loops
/// wholly inside, and open inside arcs between boundary crossings.
fn clip(loops: &[Vec<Curve>], h: &dyn Fn(P) -> f64) -> Result<(Vec<Arc>, Vec<Vec<Curve>>)> {
    let mut open: Vec<Arc> = Vec::new();
    let mut closed: Vec<Vec<Curve>> = Vec::new();
    let inside = |p: P| h(p) < 0.;
    for pieces in loops {
        let mut pts: Vec<P> = Vec::new();
        let mut flat: Vec<(usize, f64)> = Vec::new();
        for (i, piece) in pieces.iter().enumerate() {
            let [lo, hi] = piece.domain();
            for k in 0..256 {
                let t = lo + (hi - lo) * k as f64 / 256.;
                pts.push(point(piece, t)?);
                flat.push((i, t));
            }
        }
        if pts.iter().all(|&p| inside(p)) {
            closed.push(pieces.clone());
            continue;
        }
        if pts.iter().all(|&p| !inside(p)) {
            continue;
        }
        let start = (0..flat.len()).find(|&k| !inside(pts[k])).expect("some sample outside");
        let n = flat.len();
        let mut current: Option<Vec<P>> = None;
        for step in 1..=n {
            let (k0, k1) = ((start + step - 1) % n, (start + step) % n);
            let (in0, in1) = (inside(pts[k0]), inside(pts[k1]));
            if in0 != in1 {
                let (i, t0) = flat[k0];
                let t1 = if flat[k1].0 == i { flat[k1].1 } else { pieces[i].domain()[1] };
                let f = |t: f64| -> Result<f64> { Ok(h(point(&pieces[i], t)?)) };
                let (mut a, mut b) = (t0, t1);
                let fa = f(a)?;
                if fa * f(b)? > 0. {
                    return Err(refuse("boundary crossing could not be bracketed"));
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
                let near = |a: P, b: P| {
                    let d = sub(a, b);
                    dot(d, d).sqrt() < 1e-7
                };
                if in1 {
                    let mut arc = vec![cross];
                    if !near(cross, pts[k1]) {
                        arc.push(pts[k1]);
                    }
                    current = Some(arc);
                } else if let Some(mut arc) = current.take() {
                    // The crossing replaces a sample it nearly coincides with.
                    if arc.len() > 1 && near(*arc.last().unwrap(), cross) {
                        arc.pop();
                    }
                    arc.push(cross);
                    open.push(Arc { samples: arc });
                }
            } else if in1 && let Some(arc) = current.as_mut() {
                arc.push(pts[k1]);
            }
        }
        if current.is_some() {
            return Err(refuse("an inside arc did not close at the boundary"));
        }
    }
    // An arc grazing the boundary would show as an interior sample near it.
    for arc in &open {
        for &p in &arc.samples[1..arc.samples.len() - 1] {
            if h(p).abs() < 1e-6 {
                return Err(refuse("a contact arc touches a face boundary"));
            }
        }
    }
    Ok((open, closed))
}

/// Closed cycles of a contact with any capped solid of revolution: wall
/// loops clipped to the slab plus cap contacts clipped to the cap disks.
pub(crate) fn capped_cycles_in(
    contacts: CapContacts<'_>,
    slab: &Slab,
    loops: &[Vec<Curve>],
) -> Result<Vec<Vec<Curve>>> {
    let axis = slab.frame[2];
    let axial = |p: P| dot(sub(p, slab.origin), axis);
    let (z_lo, z_hi) = (slab.planes[0].0, slab.planes[1].0);
    let (wall, mut closed) = clip(loops, &|p| (z_lo - axial(p)).max(axial(p) - z_hi))?;
    let mut caps: Vec<Arc> = Vec::new();
    let sphere = match contacts {
        CapContacts::Sphere(sphere) => sphere,
        CapContacts::Traced(cap_loops) => {
            for (slot, (_, big_r)) in slab.planes.iter().enumerate() {
                if cap_loops[slot].is_empty() {
                    continue;
                }
                if *big_r <= 0. {
                    return Err(refuse("contact reaches the apex plane of a cone"));
                }
                let rho = |p: P| {
                    let d = sub(p, slab.origin);
                    dot(d, slab.frame[0]).hypot(dot(d, slab.frame[1]))
                };
                let (open, inside) = clip(&cap_loops[slot], &|p| rho(p) - big_r)?;
                caps.extend(open);
                closed.extend(inside);
            }
            return join(wall, caps, closed);
        }
    };
    // 2. Cap arcs: the sphere/cap-plane circle inside the disk.
    let mut caps: Vec<Arc> = Vec::new();
    let offset = sub(sphere.center, slab.origin);
    let s = dot(offset, axis);
    let perp = sub(offset, axis.map(|x| x * s));
    let ex = slab.frame[0];
    let ey = slab.frame[1];
    let c2 = [dot(perp, ex), dot(perp, ey)];
    let d = c2[0].hypot(c2[1]);
    for (q, big_r) in slab.planes {
        let h = q - s;
        let rho2 = sphere.radius * sphere.radius - h * h;
        if rho2 <= 0. {
            continue;
        }
        if big_r <= 0. {
            return Err(refuse("sphere reaches the apex plane of a cone"));
        }
        let rho = rho2.sqrt();
        let at = |phi: f64| -> P {
            let (sn, cs) = phi.sin_cos();
            let x = c2[0] + rho * cs;
            let y = c2[1] + rho * sn;
            std::array::from_fn(|k| slab.origin[k] + q * axis[k] + x * ex[k] + y * ey[k])
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
    join(wall, caps, closed)
}

/// Join open wall and cap arcs at their shared rim points into cycles.
fn join(wall: Vec<Arc>, caps: Vec<Arc>, mut closed: Vec<Vec<Curve>>) -> Result<Vec<Vec<Curve>>> {
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

/// One certified, lifted contact cycle.
pub(crate) struct Cycle {
    pub(crate) pieces: Vec<Curve>,
    pub(crate) distance_bound: f64,
    pub(crate) sphere_uv: Vec<SpherePatchCircle>,
    pub(crate) other_uv: Vec<(usize, Vec<Curve>)>,
    pub(crate) lift_deviation: f64,
}

/// Cycles of a sphere reaching a cap plane, each piece recertified against
/// the sphere and the surface it lies on (the wall or a cap plane) and
/// lifted onto both operands; `None` when anything does not certify.
#[allow(clippy::too_many_arguments)]
pub(crate) fn certified_cycles(
    sphere: &CanonicalSphere,
    other: crate::sphere_torus::traced_lift::Other<'_>,
    slab: &Slab,
    sphere_model: &Model,
    other_model: &Model,
    s_implicit: &dyn crate::implicit_trace::Implicit,
    wall: &dyn crate::implicit_trace::Implicit,
    wall_distance: &dyn Fn(P) -> f64,
    loops: &[Vec<Curve>],
) -> Result<Option<Vec<Cycle>>> {
    let Ok(cycles) = capped_cycles_in(CapContacts::Sphere(sphere), slab, loops) else {
        return Ok(None);
    };
    let axis = slab.frame[2];
    let mut out = Vec::with_capacity(cycles.len());
    for cycle in cycles {
        let mut bound = 0_f64;
        for piece in &cycle {
            let [lo, hi] = piece.domain();
            let mid = point(piece, 0.5 * (lo + hi))?;
            let z = dot(sub(mid, slab.origin), axis);
            let (plane_z, plane_gap) = slab
                .planes
                .iter()
                .map(|&(q, _)| (q, (z - q).abs()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .expect("two planes");
            let plane;
            let surface: &dyn crate::implicit_trace::Implicit = if wall_distance(mid) <= plane_gap {
                wall
            } else {
                plane = crate::implicit_trace::PlaneImplicit {
                    origin: std::array::from_fn(|k| slab.origin[k] + plane_z * axis[k]),
                    normal: axis,
                };
                &plane
            };
            match crate::implicit_trace::certify_curve([s_implicit, surface], piece, crate::TRACE_TOLERANCE)? {
                Some(b) => bound = bound.max(b),
                None => return Ok(None),
            }
        }
        let Ok((sphere_uv, other_uv, lift_deviation)) =
            crate::sphere_torus::traced_lift::lift_traced_with(sphere, other, sphere_model, other_model, &cycle)
        else {
            return Ok(None);
        };
        out.push(Cycle {
            pieces: cycle,
            distance_bound: bound,
            sphere_uv,
            other_uv,
            lift_deviation,
        });
    }
    Ok(Some(out))
}
