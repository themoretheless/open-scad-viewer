//! Retained annular cap representation without assuming a planar control hull.
//! Exact boundary ownership is separate from global embedding and source error.
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
pub mod reference_filling;
pub mod source_filling;

#[derive(Debug, Clone)]
pub struct RetainedAnnularCap {
    pub patches: Vec<Surface>,
    pub exact_work: u64,
    pub exact_boundary_uses: usize,
    pub global_embedding_certified: bool,
}
fn uv_line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn refused(message: &str) -> Error {
    Error::new("BREP_RETAINED_ANNULAR_CAP_REFUSED", message)
}

/// Construct radial patches between closed rings with explicitly corresponding
/// Bezier spans and the same traversal direction. All stored curve coefficients
/// are retained. This does not certify the filling's global embedding or its
/// distance from an authored planar cap, and must not authorize Solid by itself.
pub fn construct(outer: &[Curve], inner: &[Curve], max_work: u64) -> Result<RetainedAnnularCap> {
    if outer.is_empty() || outer.len() > 64 || outer.len() != inner.len() {
        return Err(refused("Need matching 1..64 annular spans"));
    }
    for ring in [outer, inner] {
        for (i, c) in ring.iter().enumerate() {
            c.validate()?;
            let d1 = c.degree + 1;
            if c.periodic
                || c.control_points.len() != d1
                || c.control_points.iter().any(|p| p.len() != 3)
                || c.knots.len() != 2 * d1
                || c.knots[..d1].iter().any(|&k| k != 0.)
                || c.knots[d1..].iter().any(|&k| k != 1.)
                || c.weights.iter().any(|&w| w <= 0.)
            {
                return Err(refused(
                    "Need positive-weight normalized retained Bezier spans",
                ));
            }
            let next = &ring[(i + 1) % ring.len()];
            if c.control_points.last() != next.control_points.first()
                || c.weights.last() != next.weights.first()
            {
                return Err(refused("Ring endpoint coefficients must agree exactly"));
            }
        }
    }
    let mut patches = Vec::with_capacity(outer.len());
    for (a, b) in outer.iter().zip(inner) {
        if a.degree != b.degree || a.knots != b.knots {
            return Err(refused(
                "Annular span degrees and parameter correspondence must agree exactly",
            ));
        }
        let surface = Surface {
            degree_u: a.degree,
            degree_v: 1,
            knots_u: a.knots.clone(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: a
                .control_points
                .iter()
                .zip(&b.control_points)
                .map(|(p, q)| vec![p.clone(), q.clone()])
                .collect(),
            weights: a
                .weights
                .iter()
                .zip(&b.weights)
                .map(|(&a, &b)| vec![a, b])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        patches.push(surface);
    }
    let mut exact_work = 0;
    let mut prove = |world: &Curve, uv: &Curve, surface: &Surface| -> Result<()> {
        let proof = nurbs_core::curve_surface_agreement::verify_exact(
            world,
            uv,
            surface,
            false,
            max_work - exact_work,
        )?
        .ok_or_else(|| refused("Annular boundary work exhausted"))?;
        if proof.work_used > max_work - exact_work {
            return Err(refused("Annular boundary exceeded shared work"));
        }
        exact_work += proof.work_used;
        match proof.outcome {
            cad_predicates::BezierIdentity::Equal => {}
            cad_predicates::BezierIdentity::Different => {
                return Err(refused("Annular boundary identity is not exact"));
            }
            cad_predicates::BezierIdentity::Indeterminate(reason) => {
                return Err(refused(&format!("Annular boundary identity remains unproved: {reason:?}")));
            }
        }
        Ok(())
    };
    let uv_u0 = uv_line([0., 0.], [1., 0.]);
    let uv_u1 = uv_line([0., 1.], [1., 1.]);
    let uv_v0 = uv_line([0., 0.], [0., 1.]);
    let uv_v1 = uv_line([1., 0.], [1., 1.]);
    let mut radial = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0.; 3], vec![0.; 3]],
        weights: vec![1., 1.],
        periodic: false,
    };
    for (i, surface) in patches.iter().enumerate() {
        prove(&outer[i], &uv_u0, surface)?;
        prove(&inner[i], &uv_u1, surface)?;
        for (end, uv_v) in [(0, &uv_v0), (outer[i].degree, &uv_v1)] {
            radial.control_points[0].clone_from(&outer[i].control_points[end]);
            radial.control_points[1].clone_from(&inner[i].control_points[end]);
            radial.weights[0] = outer[i].weights[end];
            radial.weights[1] = inner[i].weights[end];
            prove(&radial, uv_v, surface)?;
        }
    }
    Ok(RetainedAnnularCap {
        exact_boundary_uses: 4 * patches.len(),
        patches,
        exact_work,
        global_embedding_certified: false,
    })
}

/// An assembled cap replacement remains unqualified for global embedding.
#[derive(Debug, Clone)]
pub struct CapReplacement {
    pub model: crate::Model,
    pub replaced_face: usize,
    pub cap_faces: Vec<usize>,
    pub source_face_map: Vec<Option<usize>>,
    pub exact_work: u64,
    pub global_embedding_certified: bool,
}

/// Replace an annular face using its authored ring correspondence. The hole
/// traversal is reversed to align it with the outer ring, retaining its anchor.
/// Existing vertices, edges and all other surfaces stay unchanged. Global
/// intersection, material orientation and source-error audits remain mandatory.
pub fn replace(model: &crate::Model, face: usize, max_work: u64) -> Result<CapReplacement> {
    model.validate()?;
    let old = model
        .faces
        .get(face)
        .ok_or_else(|| refused("Invalid annular face"))?;
    if old.holes.len() != 1 {
        return Err(refused("Need exactly one authored hole"));
    }
    let source_wires = [old.outer, old.holes[0]];
    if model.faces.iter().enumerate().any(|(f, v)| {
        f != face
            && std::iter::once(&v.outer)
                .chain(&v.holes)
                .any(|w| source_wires.contains(w))
    }) {
        return Err(refused("Cap wires must have unique face ownership"));
    }
    let mut owner_iter = model
        .shells
        .iter()
        .enumerate()
        .flat_map(|(s, v)| {
            v.faces
                .iter()
                .filter(move |u| u.face == face)
                .map(move |u| (s, u.reversed))
        });
    let Some(owner) = owner_iter.next() else {
        return Err(refused("Cap must have one shell owner"));
    };
    if owner_iter.next().is_some() {
        return Err(refused("Cap must have one shell owner"));
    }
    let outer_uses = &model.loops[old.outer].coedges;
    let inner_coedges = &model.loops[old.holes[0]].coedges;
    let oriented = |c: &crate::Coedge| -> Result<Curve> {
        if c.reversed {
            model.edges[c.edge].curve.reverse()
        } else {
            Ok(model.edges[c.edge].curve.clone())
        }
    };
    let outer = outer_uses
        .iter()
        .map(oriented)
        .collect::<Result<Vec<_>>>()?;
    let inner = inner_coedges
        .iter()
        .rev()
        .map(|c| oriented(c)?.reverse())
        .collect::<Result<Vec<_>>>()?;
    let cap = construct(&outer, &inner, max_work)?;
    if model.faces.len() - 1 + cap.patches.len() > crate::MAX_FACES {
        return Err(refused("Annular cap exceeds existing face limit"));
    }
    let mut result = model.clone();
    result.faces.remove(face);
    for shell in &mut result.shells {
        shell.faces.retain(|u| u.face != face);
        for u in &mut shell.faces {
            if u.face > face {
                u.face -= 1;
            }
        }
    }
    let mut wire_map = vec![usize::MAX; result.loops.len()];
    let mut retained = Vec::with_capacity(result.loops.len().saturating_sub(2) + cap.patches.len());
    for (i, wire) in std::mem::take(&mut result.loops).into_iter().enumerate() {
        if !source_wires.contains(&i) {
            wire_map[i] = retained.len();
            retained.push(wire);
        }
    }
    result.loops = retained;
    for f in &mut result.faces {
        f.outer = wire_map[f.outer];
        for w in &mut f.holes {
            *w = wire_map[*w];
        }
    }
    let vertex = |c: &crate::Coedge, start: bool| {
        model.edges[c.edge].vertices[usize::from(start == c.reversed)]
    };
    let n_outer = outer.len();
    let mut radials = Vec::with_capacity(n_outer);
    result.edges.reserve(n_outer);
    for i in 0..n_outer {
        // inner_uses follows the original clockwise edge; its end is the
        // start of the aligned counterclockwise span.
        let inner_use = &inner_coedges[inner_coedges.len() - 1 - i];
        let vertices = [vertex(&outer_uses[i], true), vertex(inner_use, false)];
        radials.push(result.edges.len());
        result.edges.push(crate::Edge {
            degenerate: false,
            vertices,
            curve: Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![
                    outer[i].control_points[0].clone(),
                    inner[i].control_points[0].clone(),
                ],
                weights: vec![outer[i].weights[0], inner[i].weights[0]],
                periodic: false,
            },
        });
    }
    let mut cap_faces = Vec::with_capacity(cap.patches.len());
    result.faces.reserve(cap.patches.len());
    result.shells[owner.0].faces.reserve(cap.patches.len());
    for (i, surface) in cap.patches.into_iter().enumerate() {
        let mut a = outer_uses[i].clone();
        a.pcurve = uv_line([0., 0.], [1., 0.]);
        let mut b = inner_coedges[inner_coedges.len() - 1 - i].clone();
        b.pcurve = uv_line([1., 1.], [0., 1.]);
        let wire = result.loops.len();
        result.loops.push(crate::Loop {
            coedges: vec![
                a,
                crate::Coedge {
                    edge: radials[(i + 1) % radials.len()],
                    reversed: false,
                    pcurve: uv_line([1., 0.], [1., 1.]),
                },
                b,
                crate::Coedge {
                    edge: radials[i],
                    reversed: true,
                    pcurve: uv_line([0., 1.], [0., 0.]),
                },
            ],
        });
        let id = result.faces.len();
        cap_faces.push(id);
        result.faces.push(crate::Face {
            surface,
            outer: wire,
            holes: Vec::new(),
        });
        result.shells[owner.0].faces.push(crate::FaceUse {
            face: id,
            reversed: owner.1,
        });
    }
    result.rebuild_topology_ids();
    result.validate()?;
    let mut exact_work = cap.exact_work;
    for &f in &cap_faces {
        for c in &result.loops[result.faces[f].outer].coedges {
            let p = nurbs_core::curve_surface_agreement::verify_exact(
                &result.edges[c.edge].curve,
                &c.pcurve,
                &result.faces[f].surface,
                c.reversed,
                max_work - exact_work,
            )?
            .ok_or_else(|| refused("Assembled cap boundary work exhausted"))?;
            if p.work_used > max_work - exact_work {
                return Err(refused("Assembled cap exceeds shared work"));
            }
            exact_work += p.work_used;
            match p.outcome {
                cad_predicates::BezierIdentity::Equal => {}
                cad_predicates::BezierIdentity::Different => {
                    return Err(refused("Assembled cap coedge identity is not exact"));
                }
                cad_predicates::BezierIdentity::Indeterminate(reason) => {
                    return Err(refused(&format!("Assembled cap coedge identity remains unproved: {reason:?}")));
                }
            }
        }
    }
    Ok(CapReplacement {
        model: result,
        replaced_face: face,
        cap_faces,
        source_face_map: (0..model.faces.len())
            .map(|i| {
                if i == face {
                    None
                } else {
                    Some(i - usize::from(i > face))
                }
            })
            .collect(),
        exact_work,
        global_embedding_certified: false,
    })
}

#[cfg(test)]
#[path = "retained_annular_cap/tests/tests_integration.rs"]
mod tests;
