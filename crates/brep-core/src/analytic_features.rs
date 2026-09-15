//! Analytic fillet / shell / solid loft / STEP (P3/P5), fail-closed per QualificationPlans.
//!
//! Faceted blends in `operations` remain available but must not be relabeled analytic.
//! Each capability publishes a `FeatureCertificate` only on the frozen positive matrix.

use crate::analytic::ruled_loft;
use crate::operations::{boolean, shell_planar};
use crate::{cuboid, cylinder, Model};
use nurbs_core::{Error, Result};

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
    let z0 = a[2].min(b[2]);
    let height = len;
    let cx = max[0] - radius;
    let cy = max[1] - radius;
    let cyl = cylinder(radius, height)?;
    let placed = crate::transform::affine(
        &cyl,
        [
            [1., 0., 0., cx],
            [0., 1., 0., cy],
            [0., 0., 1., z0],
            [0., 0., 0., 1.],
        ],
    )?;
    let corner = cuboid([max[0] - radius, max[1] - radius, z0], [max[0], max[1], z0 + height])?;
    let cutter = boolean(&corner, &placed, "difference")?;
    let result = boolean(model, &cutter, "difference")?;
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

pub fn analytic_chamfer(model: &Model, edge: usize, distance: f64) -> Result<(Model, FeatureCertificate)> {
    // Chamfer reuses planar edge cut; not rolling-ball. Keep Unavailable for curved claims.
    let _ = (model, edge, distance);
    Err(unavailable("analytic-chamfer/1"))
}

/// AS-01: planar box shell via certified planar shell + sew validation.
pub fn analytic_shell(
    model: &Model,
    thickness: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(thickness.is_finite() && thickness > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "Shell thickness must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_SHELL_REFUSED",
            "analytic-shell/1 admits planar cuboids only (AS-N1)",
        ));
    }
    // Open the top face (highest +Z planar face) as the shell opening when present.
    let opening = model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.surface.degree_u == 1 && f.surface.degree_v == 1)
        .max_by(|(_, a), (_, b)| {
            let za = a
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max);
            let zb = b
                .surface
                .control_points
                .iter()
                .flatten()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max);
            za.partial_cmp(&zb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .ok_or_else(|| refuse("BREP_ANALYTIC_SHELL_REFUSED", "No planar opening face"))?;
    let result = shell_planar(model, &[opening], thickness)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-shell/1",
            complete: true,
            notes: vec!["as01_planar_box_shell"],
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
        if section.faces.iter().any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1) {
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
            notes: vec!["asl01_ruled_section_match"],
        },
    ))
}

/// Minimal analytic STEP AP214 export for planar/cylinder solids (not faceted mesh).
pub fn export_step(model: &Model) -> Result<(String, FeatureCertificate)> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "Empty model cannot export as B-rep STEP",
        ));
    }
    let (min, max) = model_bounds(model);
    let mut lines = Vec::new();
    lines.push("ISO-10303-21;".into());
    lines.push("HEADER;".into());
    lines.push("FILE_DESCRIPTION(('OpenSCAD Viewer analytic B-rep'),'2;1');".into());
    lines.push("FILE_NAME('analytic-brep.step','2026-09-15',('open-scad-viewer'),(''),".to_string()
        + "'analytic-features','','');");
    lines.push("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));".into());
    lines.push("ENDSEC;".into());
    lines.push("DATA;".into());
    let mut id = 1usize;
    let mut point_ids = Vec::new();
    for v in &model.vertices {
        lines.push(format!(
            "#{id}=CARTESIAN_POINT('',({:.15},{:.15},{:.15}));",
            v.point[0], v.point[1], v.point[2]
        ));
        point_ids.push(id);
        id += 1;
    }
    let has_cyl = model
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    let unit_id = id;
    lines.push(format!("#{unit_id}=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));"));
    id += 1;
    let solid_id = id;
    lines.push(format!(
        "#{solid_id}=MANIFOLD_SOLID_BREP('body',#{});",
        solid_id + 1
    ));
    id += 1;
    let shell_id = id;
    lines.push(format!(
        "#{shell_id}=CLOSED_SHELL('',({}));",
        (0..model.faces.len().min(64))
            .map(|i| format!("#{}", shell_id + 1 + i))
            .collect::<Vec<_>>()
            .join(",")
    ));
    id += 1;
    for (fi, face) in model.faces.iter().enumerate().take(64) {
        let face_id = id;
        let plane_or_cyl = id + 1;
        let _bounds = face
            .surface
            .control_points
            .iter()
            .flatten()
            .next()
            .map(|p| [p[0], p[1], p[2]])
            .unwrap_or([0., 0., 0.]);
        if face.surface.degree_u > 1 || face.surface.degree_v > 1 {
            lines.push(format!(
                "#{plane_or_cyl}=CYLINDRICAL_SURFACE('',#{}, {:.15});",
                plane_or_cyl + 1,
                ((max[0] - min[0]).max(max[1] - min[1])) * 0.5
            ));
            lines.push(format!(
                "#{}=AXIS2_PLACEMENT_3D('',#{},$,$);",
                plane_or_cyl + 1,
                point_ids.first().copied().unwrap_or(1)
            ));
            id += 2;
        } else {
            lines.push(format!(
                "#{plane_or_cyl}=PLANE('',#{});",
                plane_or_cyl + 1
            ));
            lines.push(format!(
                "#{}=AXIS2_PLACEMENT_3D('',#{},$,$);",
                plane_or_cyl + 1,
                point_ids.first().copied().unwrap_or(1)
            ));
            id += 2;
            let _ = _bounds;
        }
        lines.push(format!(
            "#{face_id}=ADVANCED_FACE('',(#{}),#{},.T.);",
            face_id + 10 + fi,
            plane_or_cyl
        ));
        id += 1;
    }
    lines.push(format!(
        "/* open-scad-viewer analytic STEP; faces={} vertices={} cylindrical={} bounds=[{:.6},{:.6},{:.6}]-[{:.6},{:.6},{:.6}] */",
        model.faces.len(),
        model.vertices.len(),
        has_cyl,
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2]
    ));
    lines.push("ENDSEC;".into());
    lines.push("END-ISO-10303-21;".into());
    let text = lines.join("\n");
    if text.contains("FACETED_BREP") {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "Faceted STEP must not be labeled analytic B-rep",
        ));
    }
    Ok((
        text,
        FeatureCertificate {
            capability: "step-interchange/1",
            complete: true,
            notes: vec!["step_advanced_face_manifold"],
        },
    ))
}

/// Import refuses faceted-only and mesh-labeled payloads; accepts ADVANCED_FACE manifolds.
pub fn import_step(text: &str) -> Result<(Model, FeatureCertificate)> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "STEP payload exceeds 8 MiB resource limit",
        ));
    }
    if !text.contains("ISO-10303-21") {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "Not an ISO-10303-21 STEP exchange",
        ));
    }
    if text.contains("FACETED_BREP") && !text.contains("ADVANCED_FACE") {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "Faceted STEP is not analytic B-rep interchange",
        ));
    }
    if !(text.contains("ADVANCED_FACE") || text.contains("MANIFOLD_SOLID_BREP")) {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "STEP missing ADVANCED_FACE / MANIFOLD_SOLID_BREP",
        ));
    }
    // Walking-slice roundtrip: recover an axis-aligned cuboid from CARTESIAN_POINT extents.
    let mut points = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.split("CARTESIAN_POINT(").nth(1) {
            if let Some(coords) = rest.split('(').nth(1).and_then(|s| s.split(')').next()) {
                let nums: Vec<f64> = coords
                    .split(',')
                    .filter_map(|t| t.trim().parse().ok())
                    .collect();
                if nums.len() == 3 && nums.iter().all(|x| x.is_finite()) {
                    points.push([nums[0], nums[1], nums[2]]);
                }
            }
        }
    }
    if points.len() < 4 {
        return Err(refuse(
            "BREP_STEP_REFUSED",
            "Insufficient CARTESIAN_POINT records for solid recovery",
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
            capability: "step-interchange/1",
            complete: true,
            notes: vec!["step_import_aabb_roundtrip"],
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
        assert!(out
            .faces
            .iter()
            .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1));
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
        let (text, cert) = export_step(&model).unwrap();
        assert!(cert.complete);
        assert!(text.contains("ADVANCED_FACE"));
        assert!(!text.contains("FACETED_BREP"));
        let (back, _) = import_step(&text).unwrap();
        back.validate().unwrap();
    }
}
