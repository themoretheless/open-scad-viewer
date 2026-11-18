//! Certified deviation for the mesh actually emitted on general NURBS faces:
//! variable rational weights, lofted walls and curved sweep walls.
use super::*;

pub const GENERAL_TESSELLATION_CAPABILITY: &str = "certified-general-nurbs-tessellation/1";

fn refuse(message: impl Into<String>) -> nurbs_core::Error {
    nurbs_core::Error::new("BREP_CERTIFIED_TESSELLATION_REFUSED", message)
}

fn straight(edge: &brep_core::Edge) -> bool {
    edge.degenerate || (edge.curve.degree == 1 && edge.curve.control_points.len() == 2)
}

/// Two-sided deviation bound for `nurbs_with_schedule(model, segments, _, false)`.
///
/// Curved faces must take the rectangular full-chart mesher. Each emitted grid
/// cell is bounded by the second-order Taylor enclosure of the original
/// rational surface over that cell plus the certified displacement between
/// `S(uv)` and every emitted (possibly shared edge) vertex. The parameter
/// triangles cover the face chart, so the cell maximum bounds both
/// surface-to-mesh and mesh-to-surface distance. Planar faces are exact
/// interiors; each of their curved boundary edges must also bound a certified
/// curved face, whose boundary cells then bound the shared chord polyline.
/// Trimmed curved faces (holes, curved or interior pcurves) refuse with
/// `BREP_CERTIFIED_TESSELLATION_TRIMMED_REFUSED`; cells that cross a knot
/// below C1 refuse with `BREP_CERTIFIED_TESSELLATION_REFUSED`. Edge/pcurve
/// agreement remains the model's validated tolerance contract.
pub fn certified_general_tessellation_deviation(
    model: &brep_core::Model,
    segments: usize,
) -> Result<f64> {
    if !(1..=32).contains(&segments) {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified tessellation needs 1..32 subdivisions per patch",
        ));
    }
    model.validate()?;
    if model.bodies.is_empty() || !model.shells.iter().all(|shell| shell.closed) {
        return Err(refuse("Certified general tessellation requires closed solid shells"));
    }
    let registry = EdgeSamplingRegistry::new(model, segments, false)?;
    let position = |index: usize| -> [f64; 3] {
        let p = &registry.mesh.positions[3 * index..3 * index + 3];
        [p[0], p[1], p[2]]
    };
    let mut maximum = 0f64;
    let mut covered = vec![false; model.edges.len()];
    let mut planar_curved = Vec::new();
    for shell in &model.shells {
        for use_ in &shell.faces {
            let face = &model.faces[use_.face];
            if is_affine_plane(&face.surface, model.tolerance_mm) {
                for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for coedge in &model.loops[wire].coedges {
                        if !straight(&model.edges[coedge.edge]) {
                            planar_curved.push(coedge.edge);
                        }
                    }
                }
                continue;
            }
            let Some(mesh) = rectangular_face(model, face, segments, &registry)? else {
                return Err(nurbs_core::Error::new(
                    "BREP_CERTIFIED_TESSELLATION_TRIMMED_REFUSED",
                    format!(
                        "Face {} is a trimmed curved face; certified general tessellation admits full rectangular charts only",
                        use_.face
                    ),
                ));
            };
            let columns = mesh.uv.iter().take_while(|uv| uv[1] == mesh.uv[0][1]).count();
            if columns < 2 || mesh.uv.len() % columns != 0 || mesh.uv.len() / columns < 2 {
                return Err(refuse("Rectangular face grid is malformed"));
            }
            let rows = mesh.uv.len() / columns;
            let displacement = mesh
                .uv
                .iter()
                .zip(&mesh.shared)
                .map(|(&uv, shared)| {
                    let q = match shared {
                        Some(index) => position(*index),
                        None => {
                            let p = face.surface.evaluate(uv[0], uv[1])?.point;
                            [p[0], p[1], p[2]]
                        }
                    };
                    nurbs_core::surface_chord_bound::point_distance(&face.surface, uv, q)
                })
                .collect::<Result<Vec<_>>>()
                .map_err(|e| refuse(format!("Face {} vertex enclosure: {}", use_.face, e.message)))?;
            for y in 0..rows - 1 {
                for x in 0..columns - 1 {
                    let a = y * columns + x;
                    let c = a + columns + 1;
                    let domain = [[mesh.uv[a][0], mesh.uv[c][0]], [mesh.uv[a][1], mesh.uv[c][1]]];
                    let flat = nurbs_core::surface_chord_bound::cell_flatness(&face.surface, domain)
                        .map_err(|e| refuse(format!("Face {} cell bound: {}", use_.face, e.message)))?;
                    let shift = [a, a + 1, c, c - 1]
                        .into_iter()
                        .map(|i| displacement[i])
                        .fold(0f64, f64::max);
                    maximum = maximum.max((flat + shift).next_up());
                }
            }
            for coedge in &model.loops[face.outer].coedges {
                covered[coedge.edge] = true;
            }
        }
    }
    if let Some(edge) = planar_curved.into_iter().find(|&edge| !covered[edge]) {
        return Err(refuse(format!(
            "Curved edge {edge} bounds only planar faces; its chord deviation is not certified"
        )));
    }
    if !maximum.is_finite() {
        return Err(refuse("Certified general deviation is not finite"));
    }
    Ok(maximum)
}

/// Analytic shells keep their closed-form bounds; general NURBS faces fall
/// back to the emitted-mesh cell bound. Returns the bound and capability.
pub fn certified_deviation(
    model: &brep_core::Model,
    segments: usize,
) -> Result<(f64, &'static str)> {
    match brep_core::analysis::certified_tessellation_deviation(model, segments) {
        Ok(bound) => Ok((bound, CERTIFIED_TESSELLATION_CAPABILITY)),
        Err(error) if error.code == "BREP_CERTIFIED_TESSELLATION_REFUSED" => {
            certified_general_tessellation_deviation(model, segments)
                .map(|bound| (bound, GENERAL_TESSELLATION_CAPABILITY))
        }
        Err(error) => Err(error),
    }
}
