//! Independent finite-domain Full Round, reblend and face-feature patterning.
//! Every operation stages a fresh B-rep, validates it and runs the owning audit.
use crate::{
    Error, Model, Result,
    solid_audit::{SolidAuditCertificate, audit_solid},
};
use nurbs_core::{curve::Curve, foundation::guards::Budget};
use std::collections::BTreeSet;
/// Coupled original-curve refinement of shared closed-model edges.
pub mod edge_partition;
pub mod edge_transition;
pub mod freeform;
mod full_round_caps;
pub mod full_round_curved;
pub use full_round_curved::admitted::{AdmittedFullRound, build_admitted as full_round_exact};
pub mod full_round_prism;
pub mod full_round_seams;
pub use full_round_prism::groups::{FullRoundGroupsReport, full_round_groups};
pub mod profile_blend;
/// Original patch extraction and named feature occurrences, owned by BRep features.
pub mod region_pattern;
pub mod retrim;

fn error(message: impl Into<String>) -> Error {
    Error::new("BREP_FEATURE_REFUSED", message)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: [f64; 3]) -> Result<[f64; 3]> {
    let n = v[0].hypot(v[1]).hypot(v[2]);
    if !n.is_finite() || n == 0. {
        return Err(error("Degenerate feature frame"));
    }
    Ok(v.map(|v| v / n))
}
fn plane(model: &Model, face: usize) -> Result<([f64; 3], [f64; 3])> {
    let f = model
        .faces
        .get(face)
        .ok_or_else(|| error("Unknown selected face"))?;
    let s = &f.surface;
    let u = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
    let v = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
    let e = s.evaluate(u[0] / 2. + u[1] / 2., v[0] / 2. + v[1] / 2.)?;
    let mut n = e
        .unit_normal()
        .ok_or_else(|| error("Selected face has unresolved normal"))?;
    let p = [e.point[0], e.point[1], e.point[2]];
    if s.control_points
        .iter()
        .flatten()
        .any(|q| dot(n, sub([q[0], q[1], q[2]], p)).abs() > model.tolerance_mm)
    {
        return Err(error("Operation requires planar selected faces"));
    }
    let uses = model
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| u.face == face)
        .collect::<Vec<_>>();
    if uses.len() != 1 {
        return Err(error("A single owned shell use is required"));
    }
    if uses[0].reversed {
        n = n.map(|v| -v);
    }
    Ok((p, n))
}
fn line(a: [f64; 2], b: [f64; 2]) -> Result<Curve> {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()])
}

#[derive(Clone, Debug)]
pub struct FullRoundReport {
    pub model: Model,
    pub replaced_face: usize,
    pub support_faces: [usize; 2],
    pub radius_mm: f64,
    pub audit: SolidAuditCertificate,
    pub tangency: std::sync::Arc<full_round_seams::Qualification>,
}
/// Replace the middle end face between two opposing side faces of one oriented
/// cuboid with a rational half-cylinder, radius fixed by side-face separation.
/// This is a real full-width round, not two edge fillets with guessed radii.
pub fn full_round(model: &Model, faces: [usize; 3]) -> Result<FullRoundReport> {
    model.validate()?;
    if model.faces.len() != 6 || model.vertices.len() != 8 || model.edges.len() != 12 {
        return full_round_prism::build(model, faces);
    }
    if model.vertices.len() != 8
        || model.edges.len() != 12
        || model.faces.len() != 6
        || model.bodies.len() != 1
        || model.shells.len() != 1
        || faces.iter().copied().collect::<BTreeSet<_>>().len() != 3
    {
        return Err(error(
            "Full Round currently admits one cuboid and three distinct faces",
        ));
    }
    let (_, na) = plane(model, faces[0])?;
    let (_, ny) = plane(model, faces[1])?;
    let (_, nb) = plane(model, faces[2])?;
    if dot(na, nb) > -1. + 1e-10 || dot(na, ny).abs() > 1e-10 {
        return Err(error(
            "Select opposing supports with a perpendicular middle face",
        ));
    }
    let nx = na.map(|v| -v);
    let nz = unit(cross(nx, ny))?;
    // Complete an orthonormal frame rather than magnifying the accepted
    // normal residual when mapping the constructed body back to world space.
    let ny = unit(cross(nz, nx))?;
    let axes = [nx, ny, nz];
    let origin = model.vertices[0].point;
    let project = |p| axes.map(|axis| dot(axis, sub(p, origin)));
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        let p = project(v.point);
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let tol = model.tolerance_mm * 8.;
    let mut corners = BTreeSet::new();
    for v in &model.vertices {
        let p = project(v.point);
        let mut bits = 0;
        for i in 0..3 {
            if (p[i] - lo[i]).abs() <= tol {
            } else if (p[i] - hi[i]).abs() <= tol {
                bits |= 1 << i;
            } else {
                return Err(error("Full Round requires an orthogonal cuboid"));
            }
        }
        corners.insert(bits);
    }
    if corners.len() != 8 {
        return Err(error("Collapsed cuboid"));
    }
    // Corner positions alone do not admit a cuboid: a curved carrier can share
    // all eight corners. Require six distinct planar bounding faces as well.
    let mut sides = BTreeSet::new();
    for face in 0..model.faces.len() {
        let (p, n) = plane(model, face)?;
        let n = axes.map(|axis| dot(axis, n));
        let p = project(p);
        let nonzero = (0..3).filter(|&i| n[i].abs() > 1e-10).collect::<Vec<_>>();
        if nonzero.len() != 1 || !model.faces[face].holes.is_empty() {
            return Err(error("Full Round requires six planar cuboid faces"));
        }
        let axis = nonzero[0];
        let side = if n[axis] < 0. { 0 } else { 1 };
        let bound = if side == 0 { lo[axis] } else { hi[axis] };
        if (p[axis] - bound).abs() > tol {
            return Err(error(
                "Cuboid face orientation or location disagrees with bounds",
            ));
        }
        sides.insert((axis, side));
    }
    if sides.len() != 6 || model.edges.iter().any(|e| e.curve.degree != 1) {
        return Err(error(
            "Full Round requires all six bounding planes and straight edges",
        ));
    }
    let radius = (hi[0] - lo[0]) / 2.;
    let cy = hi[1] - radius;
    let cx = lo[0] + radius;
    if !radius.is_finite() || radius <= tol || cy - lo[1] <= tol {
        return Err(error(
            "Insufficient depth for a nondegenerate full-width round",
        ));
    }
    let mut wire = vec![
        line([lo[0], lo[1]], [hi[0], lo[1]])?,
        line([hi[0], lo[1]], [hi[0], cy])?,
    ];
    for start in [0., 90.] {
        let mut arc =
            nurbs_core::primitives::circle_arc([cx, cy, 0.], [0., 0., 1.], radius, start, 90.)?;
        for p in &mut arc.control_points {
            p.truncate(2);
        }
        arc.validate()?;
        wire.push(arc);
    }
    wire.push(line([lo[0], cy], [lo[0], lo[1]])?);
    let mut local = crate::prism::extrude(&[wire], lo[2], hi[2])?;
    full_round_caps::world_charts(&mut local)?;
    let mut matrix = [[0.; 4]; 4];
    for i in 0..3 {
        matrix[i] = [nx[i], ny[i], nz[i], origin[i]];
    }
    matrix[3][3] = 1.;
    let mut result = crate::transform::affine(&local, matrix)?;
    result.inherit_topology_ids(&[model]);
    result.validate()?;
    let audit = audit_solid(&result)?;
    if !audit.ok {
        return Err(error("Full Round solid audit did not qualify"));
    }
    let contacts = [[hi[0], cy], [cx, hi[1]], [lo[0], cy]];
    let seams = contacts.map(|p| {
        [lo[2], hi[2]]
            .map(|z| std::array::from_fn(|i| origin[i] + nx[i] * p[0] + ny[i] * p[1] + nz[i] * z))
    });
    let centers = [lo[2], hi[2]]
        .map(|z| std::array::from_fn(|i| origin[i] + nx[i] * cx + ny[i] * cy + nz[i] * z));
    let tangency = full_round_seams::qualify(&result, seams, centers, radius)?;
    Ok(FullRoundReport {
        model: result,
        replaced_face: faces[1],
        support_faces: [faces[0], faces[2]],
        radius_mm: radius,
        audit,
        tangency,
    })
}

#[derive(Clone, Debug)]
pub struct ReblendReport {
    pub model: Model,
    pub original_faces: Vec<usize>,
    pub old_radius_range: [f64; 2],
    pub new_radius_mm: f64,
    pub reconstructed_edges: Vec<usize>,
    pub audit: SolidAuditCertificate,
}
/// Rebuild a complete recognized constant-radius chain with planar supports
/// and planar surroundings. Unsupported suppression/authoring fails atomically.
pub fn reblend_faces(
    model: &Model,
    faces: &[usize],
    radius: f64,
    budget: &Budget,
) -> Result<ReblendReport> {
    model.validate()?;
    if !radius.is_finite()
        || radius <= 0.
        || faces.is_empty()
        || faces.iter().copied().collect::<BTreeSet<_>>().len() != faces.len()
    {
        return Err(error(
            "Select a complete unique fillet face set and positive radius",
        ));
    }
    let mut graph = crate::aag::Aag::build(model, budget)?;
    graph.attach_face_attrs(model, budget)?;
    let chains = crate::aag_fillets::find_fillet_chains(model, &graph, budget)?;
    let selected = faces.iter().copied().collect::<BTreeSet<_>>();
    let chain = chains
        .into_iter()
        .find(|c| c.faces.iter().copied().collect::<BTreeSet<_>>() == selected)
        .ok_or_else(|| error("Selection must exactly match one recognized fillet chain"))?;
    if chain.kind != crate::aag_fillets::FilletKind::ConstantRadius {
        return Err(error("Reblend currently requires a constant-radius chain"));
    }
    let (pa, na) = plane(model, chain.supports[0])?;
    let (pb, nb) = plane(model, chain.supports[1])?;
    let axis = unit(cross(na, nb))?;
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for &face in &chain.faces {
        let face = &model.faces[face];
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            for v in crate::operations::loop_vertices(model, wire)? {
                let t = dot(axis, model.vertices[v].point);
                low = low.min(t);
                high = high.max(t);
            }
        }
    }
    let sharp = crate::direct_edit::suppress_feature(
        model,
        &crate::direct_edit::FeatureRef::FilletChain(chain.clone()),
        budget,
    )?
    .model;
    let tol = model.tolerance_mm * 32.;
    let edges = sharp
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            if e.curve.degree != 1 {
                return None;
            }
            let points = e.vertices.map(|v| sharp.vertices[v].point);
            points
                .iter()
                .all(|p| {
                    dot(na, sub(*p, pa)).abs() <= tol
                        && dot(nb, sub(*p, pb)).abs() <= tol
                        && dot(axis, *p) >= low - tol
                        && dot(axis, *p) <= high + tol
                })
                .then_some(i)
        })
        .collect::<Vec<_>>();
    if edges.is_empty() {
        return Err(error(
            "Suppressed chain has no uniquely bounded support intersection edges",
        ));
    }
    let authored = crate::exact_simple_prism_fillet(&sharp, &edges, radius)?;
    let mut result = authored.model;
    result.inherit_topology_ids(&[model]);
    result.validate()?;
    let audit = audit_solid(&result)?;
    Ok(ReblendReport {
        model: result,
        original_faces: chain.faces,
        old_radius_range: [chain.radius_min, chain.radius_max],
        new_radius_mm: radius,
        reconstructed_edges: edges,
        audit,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum PatternMode {
    Join,
    Cut,
}
#[derive(Clone, Debug)]
pub struct FacePatternReport {
    pub model: Model,
    pub source_faces: Vec<usize>,
    pub instances: usize,
    pub audit: SolidAuditCertificate,
}
fn face_tool(model: &Model, face: usize, depth: f64) -> Result<Model> {
    let (origin, n) = plane(model, face)?;
    let f = &model.faces[face];
    let s = &f.surface;
    let e = s.evaluate(s.knots_u[s.degree_u], s.knots_v[s.degree_v])?;
    let (du, _) = e
        .first_derivatives()
        .ok_or_else(|| error("Planar face parameter frame unresolved"))?;
    let x = unit(du)?;
    let y = unit(cross(n, x))?;
    let mut profiles = Vec::new();
    for (loop_index, &wire) in std::iter::once(&f.outer).chain(&f.holes).enumerate() {
        let mut curves = Vec::new();
        for coedge in &model.loops[wire].coedges {
            let mut c = model.edges[coedge.edge].curve.clone();
            if coedge.reversed {
                c = c.reverse()?;
            }
            for p in &mut c.control_points {
                let d = sub([p[0], p[1], p[2]], origin);
                *p = vec![dot(d, x), dot(d, y)];
            }
            c.validate()?;
            curves.push(c);
        }
        let area = nurbs_core::planar_area::measure(
            &curves,
            model.tolerance_mm.powi(2).max(1e-9),
            100000,
        )?;
        if !area.converged {
            return Err(error("Face-profile orientation unresolved"));
        }
        let positive = if area.area_interval_mm2[0] > 0. {
            true
        } else if area.area_interval_mm2[1] < 0. {
            false
        } else {
            return Err(error("Face-profile signed area unresolved"));
        };
        if positive != (loop_index == 0) {
            curves.reverse();
            for c in &mut curves {
                *c = c.reverse()?;
            }
        }
        profiles.push(curves);
    }
    let local = crate::prism::extrude(&profiles, depth.min(0.), depth.max(0.))?;
    let mut matrix = [[0.; 4]; 4];
    for i in 0..3 {
        matrix[i] = [x[i], y[i], n[i], origin[i]];
    }
    matrix[3][3] = 1.;
    let mut tool = crate::transform::affine(&local, matrix)?;
    tool.inherit_topology_ids(&[model]);
    Ok(tool)
}
/// Pattern extrusion features from selected planar faces. Depth is signed along
/// each outward face normal. Placements are explicit affine transforms of each
/// feature volume; no detached face copies or display meshes author the result.
pub fn pattern_faces(
    model: &Model,
    faces: &[usize],
    signed_depth: f64,
    placements: &[[[f64; 4]; 4]],
    mode: PatternMode,
    budget: &Budget,
) -> Result<FacePatternReport> {
    model.validate()?;
    if faces.is_empty()
        || faces.len() > 16
        || placements.is_empty()
        || placements.len() > 32
        || faces.len() * placements.len() > 64
        || faces.iter().copied().collect::<BTreeSet<_>>().len() != faces.len()
        || !signed_depth.is_finite()
        || signed_depth.abs() < 1e-5
    {
        return Err(error(
            "Face pattern selection, depth or instance budget invalid",
        ));
    }
    let mut guard = budget.guard("face-pattern");
    let tools = faces
        .iter()
        .map(|&face| face_tool(model, face, signed_depth))
        .collect::<Result<Vec<_>>>()?;
    let mut result = model.clone();
    for placement in placements {
        for tool in &tools {
            guard.tick()?;
            let moved = crate::transform::affine(tool, *placement)?;
            result = crate::operations::boolean(
                &result,
                &moved,
                match mode {
                    PatternMode::Join => "union",
                    PatternMode::Cut => "difference",
                },
            )?;
        }
    }
    result.inherit_topology_ids(&[model]);
    result.validate()?;
    let audit = audit_solid(&result)?;
    Ok(FacePatternReport {
        model: result,
        source_faces: faces.to_vec(),
        instances: faces.len() * placements.len(),
        audit,
    })
}
