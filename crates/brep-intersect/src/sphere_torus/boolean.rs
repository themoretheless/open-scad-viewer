//! Off-axis sphere × torus Boolean on the certified traced network.
//!
//! The intersection comes from [`intersect_sphere_torus`]: traced loops
//! certified within `TRACE_TOLERANCE` of both surfaces, with a completeness
//! certificate for the component set. Every loop is cut at the patch seams
//! of both surfaces, so each piece lies in one sphere face and one torus
//! face; its 3D curve and both pcurves are interpolated on one common
//! uniform parameter, each pcurve image is certified within
//! `LIFT_TOLERANCE` of the other surface, and the 3D curve is checked
//! against both images. Regions are classified exactly by the sign of the
//! other operand's distance function (a sphere or torus distance in closed
//! form) with a margin; a sample inside the margin is a refusal. The
//! network is assembled by `brep_core::network_boolean`.
//!
//! Refused: axial pairs (exact circles, handled by `brep_core::boolean`),
//! tangencies, an uncertified component set, a network vertex at an
//! original vertex, and any piece whose fits miss the tolerance.
use super::traced_lift::{Context, Patch, point, seam_breaks};
use super::*;
use brep_core::network_boolean::{NetworkEnd, NetworkPiece, network_boolean};
use nurbs_core::foundation::interpolation::{Parameterization, interpolate_curve};

/// Classification margin: samples closer than this to the other surface
/// are refused instead of guessed.
const MARGIN: f64 = 1e-6;
/// Largest distance from a cut point to the seam edge it lies on.
const EDGE_GAP: f64 = 1e-6;

fn refuse(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}

/// Exact distance-sign predicates of the two canonical solids.
fn inside_sphere(sphere: &CanonicalSphere, p: [f64; 3]) -> Result<bool> {
    let d = sub(p, sphere.center);
    let signed = d[0].hypot(d[1]).hypot(d[2]) - sphere.radius;
    if signed.abs() <= MARGIN {
        return Err(refuse("Sphere-torus Boolean: region sample on the sphere surface"));
    }
    Ok(signed < 0.)
}
fn inside_torus(torus: &CanonicalTorus, p: [f64; 3]) -> Result<bool> {
    let d = sub(p, torus.center);
    let z = dot(d, torus.axis);
    let rho = dot(d, torus.frame[0]).hypot(dot(d, torus.frame[1]));
    let signed = (rho - torus.major).hypot(z) - torus.minor;
    if signed.abs() <= MARGIN {
        return Err(refuse("Sphere-torus Boolean: region sample on the torus surface"));
    }
    Ok(signed < 0.)
}

/// The original edge of `model` nearest to `p`, as `(edge, t)`, when `p`
/// lies within `EDGE_GAP` of it (a traced point is up to the trace bound off
/// the true curve, hence off the seam edge); refuses at original vertices.
/// The original vertex of `model` at `p`, if any.
fn at_vertex(model: &Model, p: [f64; 3]) -> Option<usize> {
    model.vertices.iter().position(|v| {
        let d = sub(v.point, p);
        dot(d, d).sqrt() <= 1e-6
    })
}

/// How a cut point attaches to the operands' original topology.
fn network_end(models: [&Model; 2], p: [f64; 3], free_id: &mut usize) -> Result<NetworkEnd> {
    let vertices = models.map(|m| at_vertex(m, p));
    match vertices {
        [Some(_), Some(_)] => Err(refuse("Pair Boolean: network vertex at original vertices of both operands")),
        [Some(v), None] | [None, Some(v)] => {
            let operand = usize::from(vertices[0].is_none());
            let other = 1 - operand;
            let on = on_edge(models[other], p)?.map(|(edge, t)| (other, edge, t));
            Ok(NetworkEnd::Vertex { operand, vertex: v, on, point: models[operand].vertices[v].point })
        }
        [None, None] => {
            let mut on = [None, None];
            for (o, model) in models.iter().enumerate() {
                on[o] = on_edge(model, p)?.map(|(edge, te)| (o, edge, te));
            }
            if on.iter().any(Option::is_some) {
                return Ok(NetworkEnd::Edges { on, point: p });
            }
            *free_id += 1;
            Ok(NetworkEnd::Free { id: *free_id, point: p })
        }
    }
}

/// The original edge of `model` through `p`, away from its vertices.
fn on_edge(model: &Model, p: [f64; 3]) -> Result<Option<(usize, f64)>> {
    let options = nurbs_core::analysis::point_inversion::PointInversionOptions {
        epsilon1: 1e-12,
        epsilon2: 1e-12,
        samples: 256,
        max_iterations: 64,
    };
    let mut best: Option<(f64, usize, f64)> = None;
    for (index, edge) in model.edges.iter().enumerate() {
        if edge.degenerate {
            continue;
        }
        let report = nurbs_core::analysis::point_inversion::invert_point(&edge.curve, &p, options)?;
        let q = edge.curve.evaluate(report.parameter)?.point;
        let d = sub([q[0], q[1], q[2]], p);
        let gap = dot(d, d).sqrt();
        if gap <= EDGE_GAP && best.is_none_or(|(g, _, _)| gap < g) {
            best = Some((gap, index, report.parameter));
        }
    }
    Ok(best.map(|(_, e, t)| (e, t)))
}

fn fit(points: &[[f64; 3]]) -> Result<Curve> {
    interpolate_curve(points, 3, Parameterization::Uniform, None)
}

fn planar(c: Curve) -> Curve {
    Curve {
        control_points: c.control_points.iter().map(|p| vec![p[0], p[1]]).collect(),
        ..c
    }
}

/// `union`, `difference` or `intersection` of a canonical sphere solid and
/// a canonical ring-torus solid in either order, for off-axis contacts.
pub fn sphere_torus_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    let (sphere_index, sphere_model, torus_model) = match (
        sphere_sphere::recognize_any(a)?,
        recognize_torus(b)?,
    ) {
        (Some(_), Some(_)) => (0, a, b),
        _ => match (recognize_torus(a)?, sphere_sphere::recognize_any(b)?) {
            (Some(_), Some(_)) => (1, b, a),
            _ => return Err(refuse("Sphere-torus Boolean needs a canonical sphere and torus")),
        },
    };
    let sphere = sphere_sphere::recognize_any(sphere_model)?.expect("recognized above");
    let torus = recognize_torus(torus_model)?.expect("recognized above");
    let report = intersect_sphere_torus(sphere_model, torus_model, Options::default())?;
    if !report.unresolved.is_empty() {
        return Err(refuse("Sphere-torus Boolean: the intersection is not resolved"));
    }
    let operands = [a, b];
    let inside = |o: usize, p: [f64; 3]| -> Result<bool> {
        if o == sphere_index {
            inside_torus(&torus, p)
        } else {
            inside_sphere(&sphere, p)
        }
    };
    let mut loops = Vec::new();
    let mut tolerance = 1e-6_f64;
    for component in &report.components {
        match component {
            SphereTorusComponent::Traced {
                pieces,
                distance_bound,
                complete,
                ..
            } => {
                if !complete {
                    return Err(refuse(
                        "Sphere-torus Boolean: the traced component set is not certified complete",
                    ));
                }
                tolerance = tolerance.max(2. * distance_bound);
                loops.push(pieces.clone());
            }
            SphereTorusComponent::Circle { .. } => {
                return Err(refuse(
                    "Sphere-torus Boolean: axial pairs are exact circles; use brep_core::boolean",
                ));
            }
        }
    }
    traced_boolean(
        operands,
        operation,
        sphere_index,
        &sphere,
        super::traced_lift::Other::Torus(&torus),
        sphere_model,
        torus_model,
        &loops,
        tolerance,
        &inside,
    )
}

/// Boolean of a sphere and another canonical solid from certified, complete
/// traced loops: cut at seams, fit on a common parameter, certify lifts and
/// assemble with `inside` as the material predicate.
#[allow(clippy::too_many_arguments)]
pub(crate) fn traced_boolean(
    operands: [&Model; 2],
    operation: &str,
    sphere_index: usize,
    sphere: &CanonicalSphere,
    other: super::traced_lift::Other<'_>,
    sphere_model: &Model,
    torus_model: &Model,
    loops: &[Vec<Curve>],
    mut tolerance: f64,
    inside: &dyn Fn(usize, [f64; 3]) -> Result<bool>,
) -> Result<Model> {
    let torus_index = 1 - sphere_index;
    let ctx = Context::new(sphere, other, sphere_model, torus_model);
    let mut network = Vec::new();
    let mut free_id = 0;
    for pieces in loops {
        // Cut points of the whole loop: piece joins and every seam of both
        // surfaces. Each cut becomes one shared network vertex.
        let mut cuts: Vec<(usize, f64)> = Vec::new();
        for (index, piece) in pieces.iter().enumerate() {
            let mut ts = seam_breaks(&ctx, piece, false)?;
            ts.extend(seam_breaks(&ctx, piece, true)?);
            ts.sort_by(f64::total_cmp);
            let [lo, hi] = piece.domain();
            ts.dedup_by(|x, y| (*x - *y).abs() <= 1e-9 * (hi - lo));
            for t in ts {
                if t < hi - 1e-9 * (hi - lo) {
                    cuts.push((index, t));
                }
            }
        }
        // End descriptor of every cut, in loop order.
        let ends = cuts
            .iter()
            .map(|&(index, t)| network_end(operands, point(&pieces[index], t)?, &mut free_id))
            .collect::<Result<Vec<_>>>()?;
        if cuts.len() < 2 {
            return Err(refuse("Sphere-torus Boolean: a traced loop needs two cuts"));
        }
        for k in 0..cuts.len() {
            let (index, t0) = cuts[k];
            let (next_index, t1) = cuts[(k + 1) % cuts.len()];
            let piece = &pieces[index];
            let [_, hi] = piece.domain();
            // A segment runs within one traced piece up to the next cut,
            // or to the piece end when the next cut is on the next piece
            // (whose start is then this piece's end point).
            let t1 = if next_index == index && t1 > t0 { t1 } else { hi };
            let mid = point(piece, 0.5 * (t0 + t1))?;
            let (Patch::Sphere(sf), Patch::Torus(tf)) = (ctx.label(mid, false)?, ctx.label(mid, true)?)
            else {
                return Err(refuse("Sphere-torus Boolean: patch labels disagree"));
            };
            let mut count = 17;
            let built = loop {
                let ts: Vec<f64> = (0..count)
                    .map(|i| t0 + (t1 - t0) * i as f64 / (count - 1) as f64)
                    .collect();
                let p3 = ts.iter().map(|&t| point(piece, t)).collect::<Result<Vec<_>>>()?;
                let uvs = |patch| -> Result<Vec<[f64; 3]>> {
                    p3.iter()
                        .map(|&p| ctx.uv(patch, p).map(|uv| [uv[0], uv[1], 0.]))
                        .collect()
                };
                let curve = fit(&p3)?;
                let pcs = planar(fit(&uvs(Patch::Sphere(sf))?)?);
                let pct = planar(fit(&uvs(Patch::Torus(tf))?)?);
                // The 3D curve against both pcurve images on one parameter.
                let mut agree = 0_f64;
                for i in 0..=256 {
                    let s = i as f64 / 256.;
                    let c = curve.evaluate(s)?.point;
                    for (model, face, pc) in [(sphere_model, sf, &pcs), (torus_model, tf, &pct)] {
                        let uv = pc.evaluate(s)?.point;
                        let q = model.faces[face]
                            .surface
                            .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))?
                            .point;
                        let d = sub([q[0], q[1], q[2]], [c[0], c[1], c[2]]);
                        agree = agree.max(dot(d, d).sqrt());
                    }
                }
                if agree <= LIFT_TOLERANCE {
                    let lifted = [
                        ctx.certify_against(Patch::Sphere(sf), &pcs, Some(tf))?,
                        ctx.certify(Patch::Torus(tf), &pct)?,
                    ];
                    if let [Some(x), Some(y)] = lifted {
                        tolerance = tolerance.max(2. * agree.max(x).max(y));
                        break Some((curve, pcs, pct));
                    }
                }
                if count >= 129 {
                    break None;
                }
                count = 2 * count - 1;
            };
            let Some((curve, pcs, pct)) = built else {
                return Err(refuse("Sphere-torus Boolean: a network piece misses its tolerance"));
            };
            let mut faces = [0; 2];
            faces[sphere_index] = sf;
            faces[torus_index] = tf;
            let mut pcurves = [pcs.clone(), pcs];
            pcurves[torus_index] = pct;
            network.push(NetworkPiece {
                faces,
                curve,
                pcurves,
                ends: [ends[k], ends[(k + 1) % ends.len()]],
            });
        }
    }
    network_boolean(operands[0], operands[1], operation, &network, inside, tolerance)
}

/// Off-axis sphere x cylinder Boolean (sphere strictly between the caps).
pub fn sphere_cylinder_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    use crate::sphere_cylinder::{SphereCylinderComponent, intersect_sphere_cylinder, recognize_cylinder};
    let (sphere_index, sphere_model, other_model) =
        match (sphere_sphere::recognize_any(a)?, recognize_cylinder(b)?) {
            (Some(_), Some(_)) => (0, a, b),
            _ => match (recognize_cylinder(a)?, sphere_sphere::recognize_any(b)?) {
                (Some(_), Some(_)) => (1, b, a),
                _ => return Err(refuse("Sphere-cylinder Boolean needs a canonical sphere and cylinder")),
            },
        };
    let sphere = sphere_sphere::recognize_any(sphere_model)?.expect("recognized above");
    let cylinder = recognize_cylinder(other_model)?.expect("recognized above");
    let report = intersect_sphere_cylinder(sphere_model, other_model, Options::default())?;
    if !report.unresolved.is_empty() {
        return Err(refuse("Sphere-cylinder Boolean: the intersection is not resolved"));
    }
    let mut loops = Vec::new();
    let mut tolerance = 1e-6_f64;
    for component in &report.components {
        let SphereCylinderComponent::Traced { pieces, distance_bound, complete, .. } = component else {
            return Err(refuse("Sphere-cylinder Boolean: axial pairs belong to brep_core::boolean"));
        };
        if !complete {
            return Err(refuse("Sphere-cylinder Boolean: the component set is not certified complete"));
        }
        tolerance = tolerance.max(2. * distance_bound);
        loops.push(pieces.clone());
    }
    let inside = |o: usize, p: [f64; 3]| -> Result<bool> {
        if o == sphere_index {
            let d = sub(p, cylinder.center);
            let z = dot(d, cylinder.axis);
            let rho = dot(d, cylinder.frame[0]).hypot(dot(d, cylinder.frame[1]));
            decide((rho - cylinder.radius).max(z.abs() - cylinder.half_height), "cylinder")
        } else {
            inside_sphere(&sphere, p)
        }
    };
    let operands = [a, b];
    traced_boolean(
        operands,
        operation,
        sphere_index,
        &sphere,
        super::traced_lift::Other::Cylinder(&cylinder),
        sphere_model,
        other_model,
        &loops,
        tolerance,
        &inside,
    )
}

/// Off-axis sphere x frustum Boolean (sphere strictly between the caps).
pub fn sphere_cone_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    use crate::sphere_cone::{SphereConeComponent, intersect_sphere_cone};
    use brep_core::intersections::recognize_cone;
    let (sphere_index, sphere_model, other_model) =
        match (sphere_sphere::recognize_any(a)?, recognize_cone(b)?) {
            (Some(_), Some(_)) => (0, a, b),
            _ => match (recognize_cone(a)?, sphere_sphere::recognize_any(b)?) {
                (Some(_), Some(_)) => (1, b, a),
                _ => return Err(refuse("Sphere-cone Boolean needs a canonical sphere and cone")),
            },
        };
    let sphere = sphere_sphere::recognize_any(sphere_model)?.expect("recognized above");
    let cone = recognize_cone(other_model)?.expect("recognized above");
    let report = intersect_sphere_cone(sphere_model, other_model, Options::default())?;
    if !report.unresolved.is_empty() {
        return Err(refuse("Sphere-cone Boolean: the intersection is not resolved"));
    }
    let mut loops = Vec::new();
    let mut tolerance = 1e-6_f64;
    for component in &report.components {
        let SphereConeComponent::Traced { pieces, distance_bound, complete, .. } = component else {
            return Err(refuse("Sphere-cone Boolean: axial pairs belong to brep_core::boolean"));
        };
        if !complete {
            return Err(refuse("Sphere-cone Boolean: the component set is not certified complete"));
        }
        tolerance = tolerance.max(2. * distance_bound);
        loops.push(pieces.clone());
    }
    let inside = |o: usize, p: [f64; 3]| -> Result<bool> {
        if o == sphere_index {
            let d = sub(p, cone.bottom);
            let z = dot(d, cone.axis);
            let rho = dot(d, cone.frame[0]).hypot(dot(d, cone.frame[1]));
            let wall = (rho - cone.r_bottom - cone.slope * z) / cone.slope.hypot(1.);
            decide(wall.max(-z).max(z - cone.height), "cone")
        } else {
            inside_sphere(&sphere, p)
        }
    };
    let operands = [a, b];
    traced_boolean(
        operands,
        operation,
        sphere_index,
        &sphere,
        super::traced_lift::Other::Cone(&cone),
        sphere_model,
        other_model,
        &loops,
        tolerance,
        &inside,
    )
}

/// Inside when a signed distance-like value is negative beyond `MARGIN`.
fn decide(signed: f64, what: &str) -> Result<bool> {
    if signed.abs() <= MARGIN {
        return Err(Error::new(
            "BREP_UNSUPPORTED_OPERATION",
            format!("Sphere Boolean: region sample on the {what} boundary"),
        ));
    }
    Ok(signed < 0.)
}

/// Boolean of two canonical solids of revolution (`a` a cylinder or cone,
/// `b` a torus, or any pair of `Other`) from certified, complete traced
/// loops lying on their curved walls.
#[allow(clippy::too_many_arguments)]
/// Cuts closer than this in 3D are one crossing.
const SLIVER: f64 = 1e-6;

fn pair_boolean(
    a_model: &Model,
    b_model: &Model,
    operation: &str,
    a: super::traced_lift::Other<'_>,
    b: super::traced_lift::Other<'_>,
    loops: &[Vec<Curve>],
    mut tolerance: f64,
    inside: &dyn Fn(usize, [f64; 3]) -> Result<bool>,
) -> Result<Model> {
    let sides = [(a, a_model), (b, b_model)];
    let label = |o: usize, p: [f64; 3]| {
        sides[o]
            .0
            .patch(p)
            .ok_or_else(|| refuse("Pair Boolean: traced point lies on no patch"))
    };
    let mut network = Vec::new();
    let mut free_id = 0;
    for pieces in loops {
        let mut cuts: Vec<(usize, f64)> = Vec::new();
        for (index, piece) in pieces.iter().enumerate() {
            let [lo, hi] = piece.domain();
            let mut ts = vec![lo];
            for o in 0..2 {
                let (mut prev_t, mut prev) = (lo, label(o, point(piece, lo)?)?);
                for k in 1..=512 {
                    let t = lo + (hi - lo) * k as f64 / 512.;
                    let l = label(o, point(piece, t)?)?;
                    if l != prev {
                        let (mut x, mut y) = (prev_t, t);
                        for _ in 0..60 {
                            let m = 0.5 * (x + y);
                            if label(o, point(piece, m)?)? == prev {
                                x = m;
                            } else {
                                y = m;
                            }
                        }
                        ts.push(0.5 * (x + y));
                    }
                    prev = l;
                    prev_t = t;
                }
            }
            ts.sort_by(f64::total_cmp);
            ts.dedup_by(|x, y| (*x - *y).abs() <= 1e-9 * (hi - lo));
            // A seam cut at the piece end coincides with the next piece's
            // start; drop it instead of leaving a sliver.
            cuts.extend(ts.into_iter().filter(|&t| t < hi - 1e-9 * (hi - lo)).map(|t| (index, t)));
        }
        // A loop that crosses a rim at a grazing angle flips the patch label
        // back and forth within rounding; collapse cuts that coincide in 3D
        // so no sliver piece reaches the arrangement.
        let mut kept: Vec<((usize, f64), [f64; 3])> = Vec::with_capacity(cuts.len());
        for &(index, t) in &cuts {
            let p = point(&pieces[index], t)?;
            if kept.last().is_some_and(|&(_, q)| dot(sub(p, q), sub(p, q)).sqrt() <= SLIVER) {
                continue;
            }
            kept.push(((index, t), p));
        }
        while kept.len() > 2 && {
            let (a, b) = (kept[0].1, kept[kept.len() - 1].1);
            dot(sub(a, b), sub(a, b)).sqrt() <= SLIVER
        } {
            kept.pop();
        }
        let cuts: Vec<(usize, f64)> = kept.into_iter().map(|(c, _)| c).collect();
        if cuts.len() < 2 {
            return Err(refuse("Pair Boolean: a traced loop needs two cuts"));
        }
        let ends = cuts
            .iter()
            .map(|&(index, t)| network_end([a_model, b_model], point(&pieces[index], t)?, &mut free_id))
            .collect::<Result<Vec<_>>>()?;
        for k in 0..cuts.len() {
            let (index, t0) = cuts[k];
            let (next_index, t1) = cuts[(k + 1) % cuts.len()];
            // The run from this cut to the next may cross several traced
            // pieces; walk all of them, weighted by their parameter share.
            let mut run: Vec<(usize, f64, f64)> = Vec::new();
            if next_index == index && t1 > t0 {
                run.push((index, t0, t1));
            } else {
                run.push((index, t0, pieces[index].domain()[1]));
                let mut i = (index + 1) % pieces.len();
                while i != next_index {
                    let [lo, hi] = pieces[i].domain();
                    run.push((i, lo, hi));
                    i = (i + 1) % pieces.len();
                }
                run.push((next_index, pieces[next_index].domain()[0], t1));
            }
            let weights: Vec<f64> = run
                .iter()
                .map(|&(i, a, b)| {
                    let [lo, hi] = pieces[i].domain();
                    (b - a) / (hi - lo)
                })
                .collect();
            let total: f64 = weights.iter().sum();
            let at = |s: f64| -> Result<[f64; 3]> {
                let mut rest = s * total;
                let mut k = 0;
                while k + 1 < run.len() && rest > weights[k] {
                    rest -= weights[k];
                    k += 1;
                }
                let (i, a, b) = run[k];
                let f = if weights[k] > 0. { (rest / weights[k]).clamp(0., 1.) } else { 0. };
                point(&pieces[i], a + (b - a) * f)
            };
            let mid = at(0.5)?;
            let faces = [label(0, mid)?, label(1, mid)?];
            let mut count = 17;
            let built = loop {
                let p3 = (0..count).map(|i| at(i as f64 / (count - 1) as f64)).collect::<Result<Vec<_>>>()?;
                let curve = fit(&p3)?;
                let mut pcs = Vec::with_capacity(2);
                for (o, (other, _)) in sides.iter().enumerate() {
                    let uvs = p3
                        .iter()
                        .map(|&p| {
                            other
                                .uv(faces[o], p)
                                .map(|uv| [uv[0], uv[1], 0.])
                                .ok_or_else(|| refuse("Pair Boolean: point does not invert"))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    pcs.push(planar(fit(&uvs)?));
                }
                let mut agree = 0_f64;
                for i in 0..=256 {
                    let s = i as f64 / 256.;
                    let c = curve.evaluate(s)?.point;
                    for o in 0..2 {
                        let uv = pcs[o].evaluate(s)?.point;
                        let q = sides[o].1.faces[faces[o]]
                            .surface
                            .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))?
                            .point;
                        let d = sub([q[0], q[1], q[2]], [c[0], c[1], c[2]]);
                        agree = agree.max(dot(d, d).sqrt());
                    }
                }
                if agree <= LIFT_TOLERANCE {
                    let certified = [
                        crate::implicit_trace::certify_lift(
                            &a_model.faces[faces[0]].surface,
                            &pcs[0],
                            b.implicit_for(faces[1]).as_ref(),
                            LIFT_TOLERANCE,
                        )?,
                        crate::implicit_trace::certify_lift(
                            &b_model.faces[faces[1]].surface,
                            &pcs[1],
                            a.implicit_for(faces[0]).as_ref(),
                            LIFT_TOLERANCE,
                        )?,
                    ];
                    if let [Some(x), Some(y)] = certified {
                        tolerance = tolerance.max(2. * agree.max(x).max(y));
                        let second = pcs.pop().expect("two pcurves");
                        let first = pcs.pop().expect("two pcurves");
                        break Some((curve, [first, second]));
                    }
                }
                if count >= 129 {
                    break None;
                }
                count = 2 * count - 1;
            };
            let Some((curve, pcurves)) = built else {
                return Err(refuse("Pair Boolean: a network piece misses its tolerance"));
            };
            network.push(NetworkPiece {
                faces,
                curve,
                pcurves,
                ends: [ends[k], ends[(k + 1) % ends.len()]],
            });
        }
    }
    network_boolean(a_model, b_model, operation, &network, inside, tolerance)
}

fn inside_cylinder(c: &crate::sphere_cylinder::CanonicalCylinder, p: [f64; 3]) -> Result<bool> {
    let d = sub(p, c.center);
    let z = dot(d, c.axis);
    let rho = dot(d, c.frame[0]).hypot(dot(d, c.frame[1]));
    decide((rho - c.radius).max(z.abs() - c.half_height), "cylinder")
}

fn inside_cone(c: &brep_core::intersections::CanonicalCone, p: [f64; 3]) -> Result<bool> {
    let d = sub(p, c.bottom);
    let z = dot(d, c.axis);
    let rho = dot(d, c.frame[0]).hypot(dot(d, c.frame[1]));
    let wall = (rho - c.r_bottom - c.slope * z) / c.slope.hypot(1.);
    decide(wall.max(-z).max(z - c.height), "cone")
}

/// Off-axis cylinder x torus Boolean (cylinder first, torus between the caps).
pub fn cylinder_torus_boolean(cylinder_model: &Model, torus_model: &Model, operation: &str) -> Result<Model> {
    use crate::cylinder_torus::{CylinderTorusComponent, intersect_cylinder_torus};
    use crate::sphere_cylinder::recognize_cylinder;
    let (Some(cylinder), Some(torus)) = (recognize_cylinder(cylinder_model)?, recognize_torus(torus_model)?) else {
        return Err(refuse("Cylinder-torus Boolean needs a canonical cylinder and torus"));
    };
    let report = intersect_cylinder_torus(cylinder_model, torus_model, Options::default())?;
    if !report.unresolved.is_empty() {
        return Err(refuse("Cylinder-torus Boolean: the intersection is not resolved"));
    }
    let mut loops = Vec::new();
    let mut tolerance = 1e-6_f64;
    for component in &report.components {
        let CylinderTorusComponent::Traced { pieces, distance_bound, complete, .. } = component else {
            return Err(refuse("Cylinder-torus Boolean: coaxial pairs belong to brep_core::boolean"));
        };
        if !complete {
            return Err(refuse("Cylinder-torus Boolean: the component set is not certified complete"));
        }
        tolerance = tolerance.max(2. * distance_bound);
        loops.push(pieces.clone());
    }
    let inside = |o: usize, p: [f64; 3]| -> Result<bool> {
        if o == 0 { inside_torus(&torus, p) } else { inside_cylinder(&cylinder, p) }
    };
    pair_boolean(
        cylinder_model,
        torus_model,
        operation,
        super::traced_lift::Other::Cylinder(&cylinder),
        super::traced_lift::Other::Torus(&torus),
        &loops,
        tolerance,
        &inside,
    )
}

/// Off-axis frustum x torus Boolean (frustum first, torus between the caps).
pub fn cone_torus_boolean(cone_model: &Model, torus_model: &Model, operation: &str) -> Result<Model> {
    use crate::cone_torus::{ConeTorusComponent, intersect_cone_torus};
    use brep_core::intersections::recognize_cone;
    let (Some(cone), Some(torus)) = (recognize_cone(cone_model)?, recognize_torus(torus_model)?) else {
        return Err(refuse("Cone-torus Boolean needs a canonical frustum and torus"));
    };
    let report = intersect_cone_torus(cone_model, torus_model, Options::default())?;
    if !report.unresolved.is_empty() {
        return Err(refuse("Cone-torus Boolean: the intersection is not resolved"));
    }
    let mut loops = Vec::new();
    let mut tolerance = 1e-6_f64;
    for component in &report.components {
        let ConeTorusComponent::Traced { pieces, distance_bound, complete, .. } = component else {
            return Err(refuse("Cone-torus Boolean: coaxial pairs belong to brep_core::boolean"));
        };
        if !complete {
            return Err(refuse("Cone-torus Boolean: the component set is not certified complete"));
        }
        tolerance = tolerance.max(2. * distance_bound);
        loops.push(pieces.clone());
    }
    let inside = |o: usize, p: [f64; 3]| -> Result<bool> {
        if o == 0 { inside_torus(&torus, p) } else { inside_cone(&cone, p) }
    };
    pair_boolean(
        cone_model,
        torus_model,
        operation,
        super::traced_lift::Other::Cone(&cone),
        super::traced_lift::Other::Torus(&torus),
        &loops,
        tolerance,
        &inside,
    )
}
