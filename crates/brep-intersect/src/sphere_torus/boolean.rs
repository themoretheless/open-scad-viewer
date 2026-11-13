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
fn on_edge(model: &Model, p: [f64; 3]) -> Result<Option<(usize, f64)>> {
    if model.vertices.iter().any(|v| {
        let d = sub(v.point, p);
        dot(d, d).sqrt() <= 1e-6
    }) {
        return Err(refuse("Sphere-torus Boolean: network vertex at an original vertex"));
    }
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
    let torus_index = 1 - sphere_index;
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
    let ctx = Context::new(&sphere, &torus, sphere_model, torus_model);
    let mut network = Vec::new();
    let mut free_id = 0;
    for pieces in &loops {
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
                if t < hi {
                    cuts.push((index, t));
                }
            }
        }
        // End descriptor of every cut, in loop order.
        let ends = cuts
            .iter()
            .map(|&(index, t)| -> Result<NetworkEnd> {
                let p = point(&pieces[index], t)?;
                let mut on = [None, None];
                for (o, model) in [(sphere_index, sphere_model), (torus_index, torus_model)] {
                    on[o] = on_edge(model, p)?.map(|(edge, te)| (o, edge, te));
                }
                if on.iter().any(Option::is_some) {
                    return Ok(NetworkEnd::Edges { on, point: p });
                }
                free_id += 1;
                Ok(NetworkEnd::Free { id: free_id, point: p })
            })
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
                        ctx.certify(Patch::Sphere(sf), &pcs)?,
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
    network_boolean(operands[0], operands[1], operation, &network, &inside, tolerance)
}
