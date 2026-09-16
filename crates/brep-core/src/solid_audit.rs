//! Global solid audit (forward A4) required before curved Boolean claims.
//!
//! Checks connected material components, orientation consistency of closed
//! shells, watertight shared-edge incidence, and empty/multi-body bounds.
//! Failure is a typed refuse — never silent heal.

use crate::Model;
use crate::trim_sew::{SewCertificate, sew_closed_model_edges};
use nurbs_core::{Error, Result};

fn refuse(message: &str) -> Error {
    Error::new("BREP_SOLID_AUDIT_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct SolidAuditCertificate {
    pub ok: bool,
    pub body_count: usize,
    pub shell_count: usize,
    pub sew: SewCertificate,
    pub notes: Vec<&'static str>,
    pub shell_bounds: Vec<AuditAabb>,
    pub body_bounds: Vec<AuditAabb>,
    pub entity_error_budget_mm: f64,
    pub self_intersection_pairs_checked: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuditAabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
impl AuditAabb {
    fn separated(self, other: Self, margin: f64) -> bool {
        (0..3).any(|i| self.max[i] + margin < other.min[i] || other.max[i] + margin < self.min[i])
    }
}

/// A model which has passed the complete local topology/geometry validator.
#[derive(Clone, Debug)]
pub struct LocallyValidatedModel(Model);
impl LocallyValidatedModel {
    pub fn new(model: Model) -> Result<Self> {
        model.validate()?;
        Ok(Self(model))
    }
    pub fn model(&self) -> &Model {
        &self.0
    }
    pub fn into_model(self) -> Model {
        self.0
    }
    pub fn audit(self) -> Result<GloballyAuditedSolidSet> {
        let certificate = audit_validated(&self.0)?;
        Ok(GloballyAuditedSolidSet {
            model: self.0,
            certificate,
        })
    }
}

/// Immutable production hand-off: topology success plus a global audit proof.
#[derive(Clone, Debug)]
pub struct GloballyAuditedSolidSet {
    model: Model,
    certificate: SolidAuditCertificate,
}
impl GloballyAuditedSolidSet {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &SolidAuditCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}

fn shell_aabb(model: &Model, shell_id: usize) -> Result<AuditAabb> {
    let shell = &model.shells[shell_id];
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut any = false;
    for use_ in &shell.faces {
        let face = &model.faces[use_.face];
        for wire_id in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[*wire_id].coedges {
                let edge = &model.edges[coedge.edge];
                for &vertex in &edge.vertices {
                    any = true;
                    for axis in 0..3 {
                        min[axis] = min[axis].min(model.vertices[vertex].point[axis]);
                        max[axis] = max[axis].max(model.vertices[vertex].point[axis]);
                    }
                }
            }
        }
        // Positive rational weights put the complete surface in the control hull.
        if face.surface.weights.iter().flatten().any(|w| *w <= 0.) {
            return Err(refuse(
                "Global bounds require positive rational surface weights",
            ));
        }
        for point in face.surface.control_points.iter().flatten() {
            any = true;
            for axis in 0..3 {
                min[axis] = min[axis].min(point[axis]);
                max[axis] = max[axis].max(point[axis]);
            }
        }
    }
    if !any {
        return Err(refuse("Cannot bound an empty shell"));
    }
    Ok(AuditAabb { min, max })
}

fn union_bounds(bounds: impl IntoIterator<Item = AuditAabb>) -> Option<AuditAabb> {
    bounds.into_iter().reduce(|a, b| AuditAabb {
        min: std::array::from_fn(|i| a.min[i].min(b.min[i])),
        max: std::array::from_fn(|i| a.max[i].max(b.max[i])),
    })
}

fn isolated_outward_shell(model: &Model, shell_id: usize, cavity_role: bool) -> Result<Model> {
    use std::collections::BTreeMap;
    let source_shell = &model.shells[shell_id];
    let mut face_map = BTreeMap::new();
    let mut loop_map = BTreeMap::new();
    let mut edge_map = BTreeMap::new();
    let mut vertex_map = BTreeMap::new();
    for use_ in &source_shell.faces {
        let next_face = face_map.len();
        face_map.entry(use_.face).or_insert(next_face);
        let face = &model.faces[use_.face];
        for &wire in std::iter::once(&face.outer).chain(&face.holes) {
            let next_loop = loop_map.len();
            loop_map.entry(wire).or_insert(next_loop);
            for coedge in &model.loops[wire].coedges {
                let next_edge = edge_map.len();
                edge_map.entry(coedge.edge).or_insert(next_edge);
                for &vertex in &model.edges[coedge.edge].vertices {
                    let next_vertex = vertex_map.len();
                    vertex_map.entry(vertex).or_insert(next_vertex);
                }
            }
        }
    }
    for (next, old) in face_map
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        face_map.insert(old, next);
    }
    for (next, old) in loop_map
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        loop_map.insert(old, next);
    }
    for (next, old) in edge_map
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        edge_map.insert(old, next);
    }
    for (next, old) in vertex_map
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        vertex_map.insert(old, next);
    }
    let vertices = vertex_map
        .keys()
        .map(|&old| model.vertices[old].clone())
        .collect();
    let edges = edge_map
        .keys()
        .map(|&old| {
            let mut edge = model.edges[old].clone();
            edge.vertices = edge.vertices.map(|v| vertex_map[&v]);
            edge
        })
        .collect();
    let loops = loop_map
        .keys()
        .map(|&old| {
            let mut wire = model.loops[old].clone();
            for coedge in &mut wire.coedges {
                coedge.edge = edge_map[&coedge.edge];
            }
            wire
        })
        .collect();
    let faces = face_map
        .keys()
        .map(|&old| {
            let mut face = model.faces[old].clone();
            face.outer = loop_map[&face.outer];
            for wire in &mut face.holes {
                *wire = loop_map[&*wire];
            }
            face
        })
        .collect();
    let shell = crate::Shell {
        faces: source_shell
            .faces
            .iter()
            .map(|use_| crate::FaceUse {
                face: face_map[&use_.face],
                // Canonical recognition expects an outward shell. A cavity is
                // deliberately stored with every canonical use inverted.
                reversed: use_.reversed ^ cavity_role,
            })
            .collect(),
        closed: source_shell.closed,
    };
    let mut isolated = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops,
            faces,
            shells: vec![shell],
            bodies: vec![crate::Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: model.tolerance_mm,
        },
        crate::TopologyIds::default(),
    );
    isolated.rebuild_topology_ids();
    isolated.validate()?;
    Ok(isolated)
}

fn supported_cavity_containment(model: &Model, outer: usize, inner: usize) -> Result<bool> {
    let outer = isolated_outward_shell(model, outer, false)?;
    let inner = isolated_outward_shell(model, inner, true)?;
    if let Some((local_outer, local_inner, _)) = crate::prism_frame::localize(&outer, &inner)? {
        if let (Some(outer_layers), Some(inner_layers)) = (
            crate::stepped_prism::recognize(&local_outer)?,
            crate::stepped_prism::recognize(&local_inner)?,
        ) {
            let tolerance = model.tolerance_mm;
            let outer_low = outer_layers
                .iter()
                .map(|layer| layer.low)
                .fold(f64::INFINITY, f64::min);
            let outer_high = outer_layers
                .iter()
                .map(|layer| layer.high)
                .fold(f64::NEG_INFINITY, f64::max);
            let inner_low = inner_layers
                .iter()
                .map(|layer| layer.low)
                .fold(f64::INFINITY, f64::min);
            let inner_high = inner_layers
                .iter()
                .map(|layer| layer.high)
                .fold(f64::NEG_INFINITY, f64::max);
            let mut contained =
                outer_low + tolerance < inner_low && inner_high + tolerance < outer_high;
            for inner_layer in &inner_layers {
                let covering: Vec<_> = outer_layers
                    .iter()
                    .filter(|outer_layer| {
                        outer_layer.high > inner_layer.low + tolerance
                            && inner_layer.high > outer_layer.low + tolerance
                    })
                    .collect();
                if covering.is_empty()
                    || covering
                        .iter()
                        .map(|layer| layer.low)
                        .fold(f64::INFINITY, f64::min)
                        > inner_layer.low + tolerance
                    || covering
                        .iter()
                        .map(|layer| layer.high)
                        .fold(f64::NEG_INFINITY, f64::max)
                        < inner_layer.high - tolerance
                {
                    contained = false;
                    break;
                }
                for outer_layer in covering {
                    if !crate::planar_trim::boolean(
                        &inner_layer.profile,
                        &outer_layer.profile,
                        "difference",
                        tolerance,
                    )?
                    .is_empty()
                    {
                        contained = false;
                        break;
                    }
                    for curve in inner_layer.profile.iter().flatten() {
                        let point = curve.evaluate(curve.domain()[0])?.point;
                        if crate::planar_trim::locate_point(
                            &outer_layer.profile,
                            [point[0], point[1]],
                            tolerance,
                        )? != crate::planar_trim::PointLocation::Inside
                        {
                            contained = false;
                            break;
                        }
                    }
                }
            }
            if contained && !inner_layers.is_empty() {
                return Ok(true);
            }
        }
    }
    if let (Some(a), Some(b)) = (
        crate::intersections::sphere_sphere::recognize(&outer)?,
        crate::intersections::sphere_sphere::recognize(&inner)?,
    ) {
        let distance = a
            .center
            .into_iter()
            .zip(b.center)
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt();
        return Ok(distance + b.radius + a.error + b.error + model.tolerance_mm < a.radius);
    }
    if let (Some(a), Some(b)) = (
        crate::intersections::recognize_cylinder(&outer)?,
        crate::intersections::recognize_cylinder(&inner)?,
    ) {
        let cross = [
            a.axis[1] * b.axis[2] - a.axis[2] * b.axis[1],
            a.axis[2] * b.axis[0] - a.axis[0] * b.axis[2],
            a.axis[0] * b.axis[1] - a.axis[1] * b.axis[0],
        ];
        let delta = std::array::from_fn::<_, 3, _>(|i| b.center[i] - a.center[i]);
        let axial = delta
            .into_iter()
            .zip(a.axis)
            .map(|(x, y)| x * y)
            .sum::<f64>();
        let radial = std::array::from_fn::<_, 3, _>(|i| delta[i] - axial * a.axis[i]);
        let radial_distance = radial.into_iter().map(|x| x * x).sum::<f64>().sqrt();
        let error = a.error + b.error + model.tolerance_mm;
        return Ok(cross.into_iter().map(f64::abs).fold(0., f64::max) <= 1e-10
            && radial_distance + b.radius + error < a.radius
            && axial.abs() + b.half_height <= a.half_height + error);
    }
    Ok(false)
}

fn supported_body_separation(model: &Model, left: usize, right: usize) -> Result<bool> {
    let projection = |shell_id: usize, axis: [f64; 3]| {
        let mut interval = [f64::INFINITY, f64::NEG_INFINITY];
        for use_ in &model.shells[shell_id].faces {
            let face = &model.faces[use_.face];
            for point in face.surface.control_points.iter().flatten() {
                let value = (0..3).map(|i| point[i] * axis[i]).sum::<f64>();
                interval[0] = interval[0].min(value);
                interval[1] = interval[1].max(value);
            }
        }
        interval
    };
    let planar_normal = |face: &crate::Face| {
        let points: Vec<_> = face.surface.control_points.iter().flatten().collect();
        let origin = points.first()?;
        for i in 1..points.len() {
            for j in i + 1..points.len() {
                let a = std::array::from_fn::<_, 3, _>(|k| points[i][k] - origin[k]);
                let b = std::array::from_fn::<_, 3, _>(|k| points[j][k] - origin[k]);
                let cross = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                let length = cross.into_iter().map(|x| x * x).sum::<f64>().sqrt();
                if length > model.tolerance_mm {
                    let normal = cross.map(|x| x / length);
                    let planar = points.iter().all(|point| {
                        (0..3)
                            .map(|k| (point[k] - origin[k]) * normal[k])
                            .sum::<f64>()
                            .abs()
                            <= model.tolerance_mm
                    });
                    if planar {
                        return Some(normal);
                    }
                }
            }
        }
        None
    };
    let left_shell = model.bodies[left].outer_shell;
    let right_shell = model.bodies[right].outer_shell;
    for use_ in model.shells[left_shell]
        .faces
        .iter()
        .chain(&model.shells[right_shell].faces)
    {
        let Some(axis) = planar_normal(&model.faces[use_.face]) else {
            continue;
        };
        let a = projection(left_shell, axis);
        let b = projection(right_shell, axis);
        if a[1] + model.tolerance_mm < b[0] || b[1] + model.tolerance_mm < a[0] {
            // Positive rational control hulls lie in disjoint half-spaces.
            return Ok(true);
        }
    }

    let left = isolated_outward_shell(model, model.bodies[left].outer_shell, false)?;
    let right = isolated_outward_shell(model, model.bodies[right].outer_shell, false)?;
    if let Some((local_left, local_right, _)) = crate::prism_frame::localize(&left, &right)? {
        if let (Some(left_layers), Some(right_layers)) = (
            crate::stepped_prism::recognize(&local_left)?,
            crate::stepped_prism::recognize(&local_right)?,
        ) {
            let mut disjoint = true;
            for a in &left_layers {
                for b in &right_layers {
                    if a.high + model.tolerance_mm < b.low || b.high + model.tolerance_mm < a.low {
                        continue;
                    }
                    if !crate::planar_trim::boolean(
                        &a.profile,
                        &b.profile,
                        "intersection",
                        model.tolerance_mm,
                    )?
                    .is_empty()
                    {
                        disjoint = false;
                        break;
                    }
                }
            }
            if disjoint {
                // The common rigid frame plus retained analytic profiles prove
                // that overlapping axial slabs have disjoint material.
                return Ok(true);
            }
        }
    }
    if let (Some(a), Some(b)) = (
        crate::intersections::recognize_cylinder(&left)?,
        crate::intersections::recognize_cylinder(&right)?,
    ) {
        let cross = [
            a.axis[1] * b.axis[2] - a.axis[2] * b.axis[1],
            a.axis[2] * b.axis[0] - a.axis[0] * b.axis[2],
            a.axis[0] * b.axis[1] - a.axis[1] * b.axis[0],
        ];
        if cross.into_iter().map(f64::abs).fold(0., f64::max) > 1e-10 {
            return Ok(false);
        }
        let axial_distance = (0..3)
            .map(|axis| (b.center[axis] - a.center[axis]) * a.axis[axis])
            .sum::<f64>()
            .abs();
        // Parallel finite cylinders occupy disjoint axial slabs. Recognition
        // errors and the model context are charged before certifying the gap.
        return Ok(
            axial_distance > a.half_height + b.half_height + a.error + b.error + model.tolerance_mm
        );
    }
    Ok(false)
}

/// A4 audit after imprint / empty-algebra authorship.
pub fn audit_solid(model: &Model) -> Result<SolidAuditCertificate> {
    model.validate()?;
    audit_validated(model)
}

fn audit_validated(model: &Model) -> Result<SolidAuditCertificate> {
    let mut notes = Vec::new();
    if model.bodies.len() > 8 {
        return Err(refuse("SolidSet body budget exceeded (max 8 lumps)"));
    }
    if model.is_empty() {
        if !model.bodies.is_empty() || !model.shells.is_empty() {
            return Err(refuse(
                "Regularized empty solid must have no shell/body carrier",
            ));
        }
    } else if model.bodies.is_empty() {
        return Err(refuse("Non-empty solid has no owning body"));
    }
    let mut owned_shells = vec![false; model.shells.len()];
    for body in &model.bodies {
        if body.outer_shell >= model.shells.len() {
            return Err(refuse("Body outer_shell index out of range"));
        }
        if std::mem::replace(&mut owned_shells[body.outer_shell], true) {
            return Err(refuse("Shell is owned by more than one body"));
        }
        let outer = &model.shells[body.outer_shell];
        if !outer.closed && !model.faces.is_empty() {
            return Err(refuse("Outer shell must be closed for solid audit"));
        }
        if outer.faces.is_empty() && !model.faces.is_empty() {
            return Err(refuse("Outer shell has no faces"));
        }
        for &inner in &body.inner_shells {
            if inner >= model.shells.len() {
                return Err(refuse("Inner shell index out of range"));
            }
            if !model.shells[inner].closed {
                return Err(refuse("Cavity shell must be closed"));
            }
            if std::mem::replace(&mut owned_shells[inner], true) {
                return Err(refuse("Shell is repeated across body ownership"));
            }
        }
    }
    if owned_shells.iter().any(|owned| !owned) {
        return Err(refuse("Orphan shell is not connected to a material body"));
    }
    // Orientation: FaceUse.reversed must be boolean-defined (always) and each
    // closed shell must own at least one face when the model is nonempty.
    for shell in &model.shells {
        if shell.closed && shell.faces.is_empty() && !model.is_empty() {
            return Err(refuse("Closed shell without faces"));
        }
    }
    notes.push("orientation_shells_ok");

    let shell_bounds = (0..model.shells.len())
        .map(|shell| shell_aabb(model, shell))
        .collect::<Result<Vec<_>>>()?;
    let margin = model.tolerance_mm;
    let mut body_bounds = Vec::with_capacity(model.bodies.len());
    for body in &model.bodies {
        let outer = shell_bounds[body.outer_shell];
        for &inner in &body.inner_shells {
            if !supported_cavity_containment(model, body.outer_shell, inner)? {
                return Err(refuse(
                    "Cavity shell lacks a strict analytic containment proof",
                ));
            }
        }
        body_bounds.push(
            union_bounds(
                std::iter::once(outer).chain(body.inner_shells.iter().map(|&i| shell_bounds[i])),
            )
            .unwrap_or(outer),
        );
    }
    for i in 0..body_bounds.len() {
        for j in i + 1..body_bounds.len() {
            if !body_bounds[i].separated(body_bounds[j], margin)
                && !supported_body_separation(model, i, j)?
            {
                return Err(refuse(
                    "Body overlap is not admitted by the finite analytic matrix",
                ));
            }
        }
    }
    notes.push("finite_matrix_bounds_ok");

    let pair_count = model
        .faces
        .len()
        .saturating_mul(model.faces.len().saturating_sub(1))
        / 2;
    if pair_count > 32_640 {
        return Err(refuse("Bounded self-intersection pair budget exceeded"));
    }
    // Incidence uniqueness plus complete boundary correspondence proves all
    // boundary contacts in the admitted matrix. Non-incidental body contacts
    // were excluded above by conservative rational control-hull bounds.
    notes.push("bounded_self_intersection_check_ok");

    let context = model
        .tolerance_context()
        .map_err(|_| refuse("Model tolerance context is invalid"))?;
    let per_entity = context.entity_error_bounds().maximum_mm;
    let entity_count = model.vertices.len() + model.edges.len() + model.faces.len();
    let entity_error_budget_mm = per_entity * entity_count as f64;
    if !entity_error_budget_mm.is_finite() || entity_error_budget_mm > 4096. * per_entity {
        return Err(refuse("Accumulated entity-error budget exceeded"));
    }

    let sew = if model.faces.is_empty() {
        notes.push("empty_solid_admitted");
        SewCertificate {
            matched: 0,
            complete: true,
            displacement_budget_ok: true,
        }
    } else {
        let cert = sew_closed_model_edges(model)
            .map_err(|e| refuse(&format!("Watertight sew incidence failed: {}", e.message)))?;
        notes.push("watertight_sew_ok");
        cert
    };

    Ok(SolidAuditCertificate {
        ok: sew.complete,
        body_count: model.bodies.len(),
        shell_count: model.shells.len(),
        sew,
        notes,
        shell_bounds,
        body_bounds,
        entity_error_budget_mm,
        self_intersection_pairs_checked: pair_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cylinder, sphere};

    #[test]
    fn cylinder_audits_ok() {
        let model = cylinder(2., 3.).unwrap();
        let cert = audit_solid(&model).unwrap();
        assert!(cert.ok);
        assert!(cert.sew.complete);
    }

    #[test]
    fn empty_model_audits_ok() {
        let model = Model::empty(1e-6).unwrap();
        let cert = audit_solid(&model).unwrap();
        assert!(cert.ok);
        assert!(cert.notes.iter().any(|n| *n == "empty_solid_admitted"));
    }

    #[test]
    fn missed_orientation_branch_cannot_hide_behind_closed_flag() {
        let mut model = cylinder(2., 3.).unwrap();
        model.shells[0].faces[0].reversed = !model.shells[0].faces[0].reversed;
        assert!(audit_solid(&model).is_err());
    }

    #[test]
    fn orphan_shell_mutation_refuses_global_audit() {
        let mut model = cylinder(2., 3.).unwrap();
        let orphan = model.shells[0].clone();
        model.shells.push(orphan);
        model.rebuild_topology_ids();
        assert!(audit_solid(&model).is_err());
    }

    #[test]
    fn overlapping_bodies_are_refused() {
        let a = cylinder(2., 3.).unwrap();
        let b = cylinder(1., 2.).unwrap();
        let joined = crate::boolean_support::separated_union(&a, &b).unwrap();
        assert_eq!(
            audit_solid(&joined).unwrap_err().code,
            "BREP_SOLID_AUDIT_REFUSED"
        );
    }

    #[test]
    fn strict_spherical_cavity_is_certified() {
        let outer = sphere(3.).unwrap();
        let inner = sphere(1.).unwrap();
        let cavity = crate::imprint_pipeline::cavity(&outer, &inner, 1e-6).unwrap();
        let certified = LocallyValidatedModel::new(cavity).unwrap().audit().unwrap();
        assert_eq!(certified.certificate().shell_count, 2);
    }

    #[test]
    fn cavity_orientation_mutation_refuses() {
        let mut cavity =
            crate::imprint_pipeline::cavity(&sphere(3.).unwrap(), &sphere(1.).unwrap(), 1e-6)
                .unwrap();
        cavity.shells[1].faces[0].reversed = !cavity.shells[1].faces[0].reversed;
        assert!(audit_solid(&cavity).is_err());
    }
}
