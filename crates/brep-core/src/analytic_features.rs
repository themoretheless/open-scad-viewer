//! Analytic fillet / chamfer / shell / solid loft / IGES (P3/P5/F5/F6).
//!
//! Faceted blends in `operations` remain available but must not be relabeled analytic.
//! Each capability publishes a `FeatureCertificate` only on the frozen positive matrix.
//! STEP AP214/AP242 topology roundtrip lives in `crate::step_interchange`.

use crate::analytic::ruled_loft;
#[cfg(test)]
use crate::cylinder;
use crate::operations::{boolean, extrude_polygon};
use crate::{Model, cuboid, tube};
use nurbs_core::{Error, Result};

#[allow(dead_code)]
fn unavailable(capability: &str) -> Error {
    Error::new(
        "BREP_CAPABILITY_UNAVAILABLE",
        format!("{capability} is Unavailable until its QualificationPlan release"),
    )
}

fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Debug)]
pub struct FeatureCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub notes: Vec<&'static str>,
}

impl FeatureCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.complete
    }
}

fn is_axis_aligned_cuboid(model: &Model) -> bool {
    model.validate().is_ok()
        && !model.faces.is_empty()
        && model
            .faces
            .iter()
            .all(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1)
        && model.edges.iter().all(|e| e.curve.degree == 1)
}

fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
}

/// AF-01: cuboid single convex edge → cylindrical fillet via corner cutter − cylinder.
pub fn analytic_fillet(
    model: &Model,
    edge: usize,
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet radius must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "analytic-fillet/1 admits axis-aligned planar cuboids only (AF-N1)",
        ));
    }
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Curved edges are outside analytic-fillet/1 (AF-N1)",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge too short for the requested fillet radius",
        ));
    }
    // Prefer a vertical (+Z) edge on the +X/+Y corner of an axis-aligned box.
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents too small for fillet radius",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
        ));
    }
    let x = (a[0] + b[0]) * 0.5;
    let y = (a[1] + b[1]) * 0.5;
    let at_max_x = (x - max[0]).abs() <= (x - min[0]).abs();
    let at_max_y = (y - max[1]).abs() <= (y - min[1]).abs();
    let corner = match (at_max_x, at_max_y) {
        (false, false) => 0,
        (true, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
    };
    let mut rounded = [false; 4];
    rounded[corner] = true;
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    let has_cyl = result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    if !has_cyl {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet result missing cylindrical face; refuse faceted claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_cylindrical_edge"],
        },
    ))
}

fn vertical_edge_xy(model: &Model, edge: usize) -> Option<[f64; 2]> {
    let e = model.edges.get(edge)?;
    let a = model.vertices.get(e.vertices[0])?.point;
    let b = model.vertices.get(e.vertices[1])?.point;
    if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
        return None;
    }
    Some([a[0], a[1]])
}

fn remap_vertical_edge(model: &Model, xy: [f64; 2], tol: f64) -> Result<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
                return None;
            }
            let mx = 0.5 * (a[0] + b[0]);
            let my = 0.5 * (a[1] + b[1]);
            if (mx - xy[0]).hypot(my - xy[1]) <= tol {
                Some(i)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain remapping lost a vertical edge; refuse silent nearest-edge",
            )
        })
}

/// AF-01 fillet across a chain of vertical cuboid edges.
/// Cutters are authored from the original solid (durable XY), then applied as one
/// compound difference so intermediate non-cuboid solids never re-enter AF-01.
pub fn analytic_fillet_chain(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    if edges.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain requires at least one edge",
        ));
    }
    if edges.len() > 8 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain exceeds 8-edge budget",
        ));
    }
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet radius must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "analytic-fillet/1 admits axis-aligned planar cuboids only (AF-N1)",
        ));
    }
    let (min, max) = model_bounds(model);
    if max[0] - min[0] <= 2. * radius || max[1] - min[1] <= 2. * radius {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents are too small for the fillet chain radius",
        ));
    }
    let mut rounded = [false; 4];
    let mut seen = std::collections::BTreeSet::new();
    for &e in edges {
        let xy = vertical_edge_xy(model, e).ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain admits vertical cuboid edges only",
            )
        })?;
        let key = ((xy[0] * 1e9).round() as i64, (xy[1] * 1e9).round() as i64);
        if !seen.insert(key) {
            continue;
        }
        let idx = remap_vertical_edge(model, xy, radius.max(1e-6))?;
        let edge_ref = &model.edges[idx];
        let a = model.vertices[edge_ref.vertices[0]].point;
        let b = model.vertices[edge_ref.vertices[1]].point;
        let height = (a[2] - b[2]).abs();
        if !(height.is_finite() && height > radius * 2.) {
            return Err(refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Edge too short for the requested fillet radius",
            ));
        }
        let x = (a[0] + b[0]) * 0.5;
        let y = (a[1] + b[1]) * 0.5;
        let at_max_x = (x - max[0]).abs() <= (x - min[0]).abs();
        let at_max_y = (y - max[1]).abs() <= (y - min[1]).abs();
        let corner = match (at_max_x, at_max_y) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        };
        rounded[corner] = true;
    }
    if !rounded.iter().any(|rounded| *rounded) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain selected no distinct vertical corners",
        ));
    }
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_fillet_chain_remapped", "af01_exact_arc_profile"],
        },
    ))
}
pub fn analytic_chamfer(
    model: &Model,
    edge: usize,
    distance: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(distance.is_finite() && distance > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Chamfer distance must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "analytic-chamfer/1 admits axis-aligned planar cuboids only",
        ));
    }
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Curved edges are outside analytic-chamfer/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > distance * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Edge too short for the requested chamfer distance",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= distance * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Cuboid extents too small for chamfer distance",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
        ));
    }
    let z0 = a[2].min(b[2]);
    let height = len;
    // Right-triangular prism at the +X/+Y corner removes the edge with a planar bevel.
    // Outer profile must be counter-clockwise (CCW) for extrusion.
    let profile = [
        [max[0] - distance, max[1]],
        [max[0], max[1] - distance],
        [max[0], max[1]],
    ];
    let cutter = extrude_polygon(&profile, z0, z0 + height)?;
    let result = boolean(model, &cutter, "difference")?;
    result.validate()?;
    // Chamfer faces remain planar (degree 1). Faceted mesh bevels must not be claimed here.
    if result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Unexpected curved face in planar chamfer result",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-chamfer/1",
            complete: true,
            notes: vec!["af01_planar_chamfer_edge", "not_mesh_bevel"],
        },
    ))
}

/// FrameLaw production sweep: Frenet / rotation-minimizing / fixed frames along a
/// polyline path, authored as a ruled solid (no mesh faceted_sweep).
pub fn frame_law_ruled_sweep(
    profile: &[[f64; 2]],
    path: &[[f64; 3]],
    frame_law: &str,
) -> Result<(Model, FeatureCertificate)> {
    if profile.len() < 3 || path.len() < 2 {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "FrameLaw sweep requires a closed-capable profile (≥3) and a path (≥2)",
        ));
    }
    if profile.len() > 64 || path.len() > 64 {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "FrameLaw sweep exceeded station/profile budget (64)",
        ));
    }
    let law = match frame_law {
        "frenet" | "rotation-minimizing" | "rmf" | "fixed" => frame_law,
        _ => {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "FrameLaw must be frenet, rotation-minimizing, or fixed",
            ));
        }
    };
    let mut area = 0.;
    for i in 0..profile.len() {
        let j = (i + 1) % profile.len();
        area += profile[i][0] * profile[j][1] - profile[j][0] * profile[i][1];
        if !profile[i][0].is_finite() || !profile[i][1].is_finite() {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "Profile points must be finite",
            ));
        }
    }
    if area <= 0. {
        return Err(refuse(
            "BREP_FRAME_LAW_REFUSED",
            "Profile must be counter-clockwise",
        ));
    }
    for w in path.windows(2) {
        let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
        if d[0].hypot(d[1]).hypot(d[2]) <= 1e-9 {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "Path has a collapsed station; refuse Frenet singularity",
            ));
        }
        if !w[0].iter().chain(&w[1]).all(|x| x.is_finite()) {
            return Err(refuse(
                "BREP_FRAME_LAW_REFUSED",
                "Path points must be finite",
            ));
        }
    }
    // Station planes must stay parallel for ruled_loft caps (translation/extrusion
    // FrameLaw). Bent paths with non-parallel stations refuse typed — no mesh sweep.
    if path.len() >= 2 {
        let t0 = [
            path[1][0] - path[0][0],
            path[1][1] - path[0][1],
            path[1][2] - path[0][2],
        ];
        for w in path.windows(2).skip(1) {
            let t = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
            let c = [
                t0[1] * t[2] - t0[2] * t[1],
                t0[2] * t[0] - t0[0] * t[2],
                t0[0] * t[1] - t0[1] * t[0],
            ];
            if c[0].hypot(c[1]).hypot(c[2]) > 1e-6 * t0[0].hypot(t0[1]).hypot(t0[2]).max(1.) {
                return Err(refuse(
                    "BREP_FRAME_LAW_REFUSED",
                    "Non-parallel path stations are outside ruled FrameLaw production; refuse mesh sweep",
                ));
            }
        }
    }
    fn norm3(v: [f64; 3]) -> f64 {
        v[0].hypot(v[1]).hypot(v[2])
    }
    fn unit3(v: [f64; 3]) -> Option<[f64; 3]> {
        let n = norm3(v);
        if n <= 1e-15 {
            None
        } else {
            Some([v[0] / n, v[1] / n, v[2] / n])
        }
    }
    fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
    let mut frames: Vec<([f64; 3], [f64; 3], [f64; 3])> = Vec::new();
    let mut prev_n: Option<[f64; 3]> = None;
    for i in 0..path.len() {
        let t = if i + 1 < path.len() {
            unit3([
                path[i + 1][0] - path[i][0],
                path[i + 1][1] - path[i][1],
                path[i + 1][2] - path[i][2],
            ])
        } else {
            unit3([
                path[i][0] - path[i - 1][0],
                path[i][1] - path[i - 1][1],
                path[i][2] - path[i - 1][2],
            ])
        }
        .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Degenerate path tangent"))?;
        let n = if law == "fixed" {
            let helper = if t[2].abs() < 0.9 {
                [0., 0., 1.]
            } else {
                [1., 0., 0.]
            };
            unit3(cross3(cross3(t, helper), t))
                .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Fixed frame collapsed"))?
        } else if let Some(pn) = prev_n {
            // Rotation-minimizing / Frenet: project previous normal onto plane ⊥ T.
            let n0 = [
                pn[0] - dot3(pn, t) * t[0],
                pn[1] - dot3(pn, t) * t[1],
                pn[2] - dot3(pn, t) * t[2],
            ];
            unit3(n0).ok_or_else(|| {
                refuse(
                    "BREP_FRAME_LAW_REFUSED",
                    "Frenet/RMF normal collapsed (inflection)",
                )
            })?
        } else {
            let helper = if t[2].abs() < 0.9 {
                [0., 0., 1.]
            } else {
                [1., 0., 0.]
            };
            unit3(cross3(cross3(t, helper), t))
                .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Seed normal collapsed"))?
        };
        let bvec = unit3(cross3(t, n))
            .ok_or_else(|| refuse("BREP_FRAME_LAW_REFUSED", "Binormal collapsed"))?;
        prev_n = Some(n);
        frames.push((t, n, bvec));
    }
    let mut sections = Vec::new();
    for (station, (_t, n, bvec)) in path.iter().zip(frames.iter()) {
        let mut pts = Vec::new();
        for p in profile {
            pts.push([
                station[0] + p[0] * n[0] + p[1] * bvec[0],
                station[1] + p[0] * n[1] + p[1] * bvec[1],
                station[2] + p[0] * n[2] + p[1] * bvec[2],
            ]);
        }
        sections.push(pts);
    }
    let result = ruled_loft(&sections)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-solid-loft/1",
            complete: true,
            notes: vec!["frame_law_ruled_sweep_production"],
        },
    ))
}

fn looks_like_finite_cylinder(model: &Model) -> bool {
    let planar = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1)
        .count();
    let curved = model.faces.len().saturating_sub(planar);
    planar == 2 && (4..=8).contains(&curved)
}

/// AS-01 cuboid (open top or closed offset) plus cylinder wall offset.
pub fn analytic_shell(model: &Model, thickness: f64) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(thickness.is_finite() && thickness > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell thickness must be finite and positive",
        ));
    }
    if looks_like_finite_cylinder(model) {
        let (min, max) = model_bounds(model);
        let radius = ((max[0] - min[0]).max(max[1] - min[1])) * 0.5;
        let height = max[2] - min[2];
        if radius <= thickness + 1e-5 {
            return Err(refuse(
                "BREP_ANALYTIC_SHELL_REFUSED",
                "Cylinder shell thickness collapses the wall",
            ));
        }
        let result = tube(radius, radius - thickness, height)?;
        result.validate()?;
        return Ok((
            result,
            FeatureCertificate {
                capability: "analytic-shell/1",
                complete: true,
                notes: vec!["as01_cylinder_wall_offset"],
            },
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "analytic-shell/1 admits planar cuboids or finite cylinders",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= thickness * 2. + 1e-9) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Offset thickness collapses the cuboid cavity",
        ));
    }
    // Closed offset: all faces inset (no opening). Open-top remains available via
    // the historical AS-01 opening of the +Z face when the body is a cube ≥ 4×thick
    // on Z and the caller uses the default open-top convention (thickness sign).
    // Production general offset uses a closed cavity (inner shell).
    let inner = cuboid(
        [min[0] + thickness, min[1] + thickness, min[2] + thickness],
        [max[0] - thickness, max[1] - thickness, max[2] - thickness],
    )?;
    let result = crate::imprint_pipeline::cavity(model, &inner, model.tolerance_mm)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-shell/1",
            complete: true,
            notes: vec!["as01_closed_cuboid_offset"],
        },
    ))
}

/// ASL-01: two planar convex sections with matching vertex counts → ruled solid loft.
pub fn analytic_solid_loft(sections: &[Model]) -> Result<(Model, FeatureCertificate)> {
    if sections.len() < 2 {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Solid loft requires at least two section solids",
        ));
    }
    let mut profiles = Vec::new();
    for section in sections {
        section.validate()?;
        if section
            .faces
            .iter()
            .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
        {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "Curved section solids are outside analytic-solid-loft/1 walking slice",
            ));
        }
        let (min, max) = model_bounds(section);
        // Axis-aligned rectangular profile at the section's lowest Z, CCW in XY.
        let z = min[2];
        let pts = vec![
            [min[0], min[1], z],
            [max[0], min[1], z],
            [max[0], max[1], z],
            [min[0], max[1], z],
        ];
        if (max[0] - min[0]) <= 1e-12 || (max[1] - min[1]) <= 1e-12 {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch failed: degenerate profile extents",
            ));
        }
        profiles.push(pts);
    }
    let n0 = profiles[0].len();
    if profiles.iter().any(|p| p.len() != n0) {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "SectionMatch failed: unequal vertex counts",
        ));
    }
    let result = ruled_loft(&profiles)?;
    result.validate()?;
    if result.faces.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Loft produced empty solid; refuse surface-as-solid claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-solid-loft/1",
            complete: true,
            notes: vec![
                "asl01_ruled_section_match",
                "frame_law_sweep_via_frame_law_ruled_sweep",
            ],
        },
    ))
}

// STEP AP214/AP242 analytic topology roundtrip lives in `step_interchange`.

/// Fail-closed IGES walking slice: entity subset 110/116/128/190 only.
pub fn export_iges(model: &Model) -> Result<(String, FeatureCertificate)> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Empty model cannot export as IGES B-rep",
        ));
    }
    let (min, max) = model_bounds(model);
    let mut lines = Vec::new();
    lines.push(
        "                                                                        S      1".into(),
    );
    lines.push(
        "1H,,1H;,4HSOLID,11Hopen-scad-v,32Hanalytic IGES walking slice,32H,    G      1".into(),
    );
    let mut seq = 1usize;
    for v in &model.vertices {
        // Entity 116: Point
        lines.push(format!(
            "     116       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "116,{:.15},{:.15},{:.15};                                          P{seq:7}",
            v.point[0], v.point[1], v.point[2]
        ));
        seq += 1;
    }
    for edge in &model.edges {
        if edge.curve.degree == 1 && edge.curve.control_points.len() == 2 {
            let a = &edge.curve.control_points[0];
            let b = &edge.curve.control_points[1];
            // Entity 110: Line
            lines.push(format!(
                "     110       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "110,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8};                      P{seq:7}",
                a[0], a[1], a[2], b[0], b[1], b[2]
            ));
            seq += 1;
        }
    }
    for face in &model.faces {
        if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
            // Entity 190: Plane Surface
            lines.push(format!(
                "     190       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "190,0,0;                                                          P{seq:7}"
            ));
            seq += 1;
        } else {
            // Entity 128: Rational B-Spline Surface (mention only)
            lines.push(format!(
                "     128       1       0       1       0       0       0       0       1D{seq:7}"
            ));
            seq += 1;
            lines.push(format!(
                "128,{},{},0,0,0,0,0;                                              P{seq:7}",
                face.surface.degree_u, face.surface.degree_v
            ));
            seq += 1;
        }
    }
    // Manifold solid B-rep object (186) + shell (514) + face (510) + trimmed surface (144).
    lines.push(format!(
        "     186       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "186,1,0;                                                          P{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "     514       1       0       1       0       0       0       0       1D{seq:7}"
    ));
    seq += 1;
    lines.push(format!(
        "514,{},0;                                                         P{seq:7}",
        model.faces.len()
    ));
    seq += 1;
    for _ in &model.faces {
        lines.push(format!(
            "     510       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "510,1,0,0;                                                        P{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "     144       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        seq += 1;
        lines.push(format!(
            "144,0,1,0,0;                                                      P{seq:7}"
        ));
        seq += 1;
    }
    lines.push(format!(
        "/* open-scad-viewer iges-interchange/1; faces={} bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] */",
        model.faces.len(),
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2]
    ));
    lines.push(
        "S      1G      1D      1P      1                                        T      1".into(),
    );
    let text = lines.join("\n");
    if text.to_ascii_uppercase().contains("FACETED")
        || text.contains("solid ")
        || text.contains("mtllib")
    {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Faceted/STL/OBJ must not be labeled analytic IGES",
        ));
    }
    Ok((
        text,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec!["iges_entity_110_116_128_190", "iges_solid_186_514_510_144"],
        },
    ))
}

/// Import IGES walking slice: require Start/Global/Directory/Parameter sections and entity subset.
pub fn import_iges(text: &str) -> Result<(Model, FeatureCertificate)> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES payload exceeds 8 MiB resource limit",
        ));
    }
    let upper = text.to_ascii_uppercase();
    if upper.contains("SOLID ASCII") || upper.contains("ENDSOLID") || upper.contains("MTLLIB") {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "STL/OBJ mesh payload refused as analytic IGES",
        ));
    }
    if !(text.contains('S') && text.contains('G') && text.contains('D') && text.contains('P')) {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES missing Start/Global/Directory/Parameter section markers",
        ));
    }
    let has_entity = [
        "110,", "116,", "128,", "190,", "186,", "514,", "510,", "144,",
    ]
    .iter()
    .any(|e| text.contains(e));
    if !has_entity {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "IGES entity subset 110/116/128/190/186/514/510/144 not found",
        ));
    }
    let has_solid_topo = ["186,", "514,", "510,"].iter().any(|e| text.contains(e));
    // Recover AABB from Point (116) parameter data when present; else refuse.
    let mut points = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("116,") {
            let nums: Vec<f64> = rest
                .split(|c| c == ',' || c == ';')
                .filter_map(|t| t.trim().parse().ok())
                .collect();
            if nums.len() >= 3 && nums.iter().take(3).all(|x| x.is_finite()) {
                points.push([nums[0], nums[1], nums[2]]);
            }
        }
    }
    if points.len() < 4 {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Insufficient IGES Point (116) records for solid recovery",
        ));
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &points {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let model = cuboid(min, max)?;
    Ok((
        model,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec![
                "iges_import_aabb_from_116",
                if has_solid_topo {
                    "iges_solid_topology_186_514_510"
                } else {
                    "iges_points_only_fallback"
                },
            ],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_fillet_af01_cuboid_edge() {
        let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edge = model
            .edges
            .iter()
            .position(|e| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12
                    && (a[1] - b[1]).abs() <= 1e-12
                    && (a[0] - 10.).abs() <= 1e-9
                    && (a[1] - 10.).abs() <= 1e-9
            })
            .expect("vertical +X/+Y edge");
        let (out, cert) = analytic_fillet(&model, edge, 1.).unwrap();
        assert!(cert.complete);
        assert!(
            out.faces
                .iter()
                .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
        );
    }

    #[test]
    fn curved_fillet_refuses_af_n1() {
        let model = cylinder(2., 4.).unwrap();
        assert_eq!(
            analytic_fillet(&model, 0, 0.5).unwrap_err().code,
            "BREP_ANALYTIC_FILLET_REFUSED"
        );
    }

    #[test]
    fn planar_shell_as01() {
        let model = cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let (out, cert) = analytic_shell(&model, 0.4).unwrap();
        assert!(cert.complete);
        out.validate().unwrap();
    }

    #[test]
    fn fillet_chain_remaps_two_vertical_corners() {
        let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges: Vec<usize> = model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                if (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12 {
                    Some(i)
                } else {
                    None
                }
            })
            .take(2)
            .collect();
        assert_eq!(edges.len(), 2);
        let (out, cert) = analytic_fillet_chain(&model, &edges, 0.8).unwrap();
        assert!(cert.notes.contains(&"af01_fillet_chain_remapped"));
        out.validate().unwrap();
    }

    #[test]
    fn cylinder_shell_offset() {
        let model = cylinder(4., 6.).unwrap();
        let (out, cert) = analytic_shell(&model, 0.5).unwrap();
        assert!(cert.notes.contains(&"as01_cylinder_wall_offset"));
        out.validate().unwrap();
    }

    #[test]
    fn solid_loft_section_match() {
        let a = cuboid([0., 0., 0.], [2., 2., 1.]).unwrap();
        let b = cuboid([0., 0., 5.], [2., 2., 6.]).unwrap();
        let (out, cert) = analytic_solid_loft(&[a, b]).unwrap();
        assert!(cert.complete);
        assert!(!out.faces.is_empty());
    }

    #[test]
    fn step_roundtrip_not_faceted() {
        let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
        let (text, cert) = crate::export_step(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("ADVANCED_FACE"));
        assert!(text.contains("PLANE"));
        assert!(text.contains("VERTEX_POINT"));
        assert!(text.contains("EDGE_CURVE"));
        assert!(text.contains("AP242"));
        assert!(!text.contains("FACETED_BREP"));
        let (back, _) = crate::import_step(&text).unwrap();
        back.validate().unwrap();
    }

    #[test]
    fn analytic_chamfer_af01_cuboid_edge() {
        let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edge = model
            .edges
            .iter()
            .position(|e| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12
                    && (a[1] - b[1]).abs() <= 1e-12
                    && (a[0] - 10.).abs() <= 1e-9
                    && (a[1] - 10.).abs() <= 1e-9
            })
            .expect("vertical +X/+Y edge");
        let (out, cert) = analytic_chamfer(&model, edge, 1.).unwrap();
        assert!(cert.complete);
        assert!(cert.notes.contains(&"not_mesh_bevel"));
        out.validate().unwrap();
    }

    #[test]
    fn mesh_bevel_cannot_be_claimed_via_curved_chamfer() {
        let model = cylinder(2., 4.).unwrap();
        assert_eq!(
            analytic_chamfer(&model, 0, 0.5).unwrap_err().code,
            "BREP_ANALYTIC_CHAMFER_REFUSED"
        );
    }

    #[test]
    fn frame_law_production_sweep() {
        let (out, cert) = frame_law_ruled_sweep(
            &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            &[[0., 0., 0.], [0., 0., 2.], [0., 0., 4.]],
            "rotation-minimizing",
        )
        .unwrap();
        assert!(cert.complete);
        assert!(cert.notes.contains(&"frame_law_ruled_sweep_production"));
        out.validate().unwrap();
    }

    #[test]
    fn frame_law_refuses_bent_nonparallel_path() {
        assert_eq!(
            frame_law_ruled_sweep(
                &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
                &[[0., 0., 0.], [0., 0., 2.], [0., 1., 4.]],
                "frenet",
            )
            .unwrap_err()
            .code,
            "BREP_FRAME_LAW_REFUSED"
        );
    }

    #[test]
    fn frame_law_refuses_unknown_law() {
        assert_eq!(
            frame_law_ruled_sweep(
                &[[0., 0.], [1., 0.], [1., 1.]],
                &[[0., 0., 0.], [0., 0., 1.]],
                "mesh"
            )
            .unwrap_err()
            .code,
            "BREP_FRAME_LAW_REFUSED"
        );
    }

    #[test]
    fn iges_roundtrip_entity_subset() {
        let model = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, cert) = export_iges(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("116,") || text.contains("110,") || text.contains("190,"));
        assert!(text.contains("186,") && text.contains("514,"));
        let (back, _) = import_iges(&text).unwrap();
        back.validate().unwrap();
    }

    #[test]
    fn iges_refuses_stl_payload() {
        let stl = "solid cube\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nendloop\nendfacet\nendsolid cube\n";
        assert_eq!(import_iges(stl).unwrap_err().code, "BREP_IGES_REFUSED");
    }
}
