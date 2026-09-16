//! Freeform NURBS STEP for closed solids (elevated bicubic faces).
//!
//! Capability `nurbs-step-solid/1`. Constructor analytic STEP stays on
//! `step_interchange` / `step-interchange/1`. No Parasolid claims.
//! `general-nurbs-step` is lifted here for freeform cuboid / bump / cavity solids.

use crate::Model;
use crate::analytic_features::FeatureCertificate;
use crate::nurbs_step_shared::{
    StepWriter, as_uniform_bicubic, emit_b_spline_surface, fmt_refs, is_planar_surface,
    is_uniform_bicubic_positive, parse_b_spline_surfaces, parse_entities, refuse,
    refuse_mesh_payloads_common, step_header, surface_aabb,
};
use nurbs_core::{Result, surface::Surface};

pub const NURBS_STEP_SOLID_CAPABILITY: &str = "nurbs-step-solid/1";

/// Cuboid with each face elevated to uniform bicubic (degree 3×3, w≡1) for STEP honesty.
pub fn freeform_cuboid_solid(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    let mut model = crate::cuboid(min, max)?;
    for face in &mut model.faces {
        let elev = as_uniform_bicubic(&face.surface).ok_or_else(|| {
            refuse("freeform_cuboid_solid requires elevatable planar bilinear faces")
        })?;
        face.surface = elev;
    }
    model.validate()?;
    Ok(model)
}

fn bump_top_surface(min: [f64; 3], max: [f64; 3], bump: f64) -> Surface {
    // Match cuboid +Z face control layout: corners [4,5,6,7] → bilinear then elevate + bump.
    let p00 = [min[0], min[1], max[2]];
    let p10 = [max[0], min[1], max[2]];
    let p01 = [min[0], max[1], max[2]];
    let p11 = [max[0], max[1], max[2]];
    let bilinear = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![p00.to_vec(), p01.to_vec()],
            vec![p10.to_vec(), p11.to_vec()],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let mut s = as_uniform_bicubic(&bilinear).expect("bilinear elevates");
    for i in 1..3 {
        for j in 1..3 {
            s.control_points[i][j][2] += bump;
        }
    }
    s
}

/// Cuboid with +Z face replaced by a native bicubic bump (corners match).
pub fn freeform_cuboid_with_bump_face(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    let mut model = freeform_cuboid_solid(min, max)?;
    // cuboid face 1 is +Z ([4,5,6,7])
    model.faces[1].surface = bump_top_surface(min, max, 0.15 * (max[2] - min[2]).max(0.05));
    if !is_uniform_bicubic_positive(&model.faces[1].surface) {
        return Err(refuse("bump face must be uniform bicubic w≡1"));
    }
    model.validate()?;
    Ok(model)
}

fn admit_solid_for_export(model: &Model) -> Result<()> {
    model.validate()?;
    if model.bodies.is_empty() || model.shells.is_empty() {
        return Err(refuse(
            "nurbs-step-solid/1 admits closed solids with bodies only",
        ));
    }
    if !model.shells.iter().all(|s| s.closed) {
        return Err(refuse("nurbs-step-solid/1 requires closed shells"));
    }
    if model.edges.iter().any(|e| e.curve.degree != 1) {
        return Err(refuse("nurbs-step-solid/1 boundary edges must be LINE"));
    }
    for face in &model.faces {
        if as_uniform_bicubic(&face.surface).is_none() {
            return Err(refuse(
                "nurbs-step-solid/1 faces must be elevatable to bicubic w≡1",
            ));
        }
    }
    Ok(())
}

/// Export freeform solid: MANIFOLD_SOLID_BREP / BREP_WITH_VOIDS + CLOSED_SHELL + B_SPLINE faces.
pub fn export_nurbs_step_solid(model: &Model) -> Result<(String, FeatureCertificate)> {
    admit_solid_for_export(model)?;
    let mut w = StepWriter::new();
    let mut out = step_header(
        "nurbs-freeform-solid.step",
        "OpenSCAD Viewer freeform NURBS solid",
    );

    let mut vid = Vec::with_capacity(model.vertices.len());
    let mut pid = Vec::with_capacity(model.vertices.len());
    for v in &model.vertices {
        let p = w.cartesian(v.point);
        pid.push(p);
        vid.push(w.vertex_point(p));
    }

    let mut edge_ids = Vec::with_capacity(model.edges.len());
    for e in &model.edges {
        let a = e.vertices[0];
        let b = e.vertices[1];
        let pa = model.vertices[a].point;
        let pb = model.vertices[b].point;
        let dir = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        edge_ids.push(w.line_edge(vid[a], vid[b], pid[a], dir));
    }

    let mut face_ids = Vec::with_capacity(model.faces.len());
    for face in &model.faces {
        let surface = as_uniform_bicubic(&face.surface)
            .ok_or_else(|| refuse("face elevation failed during solid export"))?;
        let surf_id = emit_b_spline_surface(&mut w, &surface);
        let outer_loop = {
            let loop_ = &model.loops[face.outer];
            let oriented: Vec<usize> = loop_
                .coedges
                .iter()
                .map(|c| {
                    let sense = if c.reversed { ".F." } else { ".T." };
                    w.emit(format!(
                        "ORIENTED_EDGE('',*,*,#{},{})",
                        edge_ids[c.edge], sense
                    ))
                })
                .collect();
            w.emit(format!("EDGE_LOOP('',({}))", fmt_refs(&oriented)))
        };
        let mut bounds = vec![w.emit(format!("FACE_OUTER_BOUND('',#{outer_loop},.T.)"))];
        for &hid in &face.holes {
            let loop_ = &model.loops[hid];
            let oriented: Vec<usize> = loop_
                .coedges
                .iter()
                .map(|c| {
                    let sense = if c.reversed { ".F." } else { ".T." };
                    w.emit(format!(
                        "ORIENTED_EDGE('',*,*,#{},{})",
                        edge_ids[c.edge], sense
                    ))
                })
                .collect();
            let lid = w.emit(format!("EDGE_LOOP('',({}))", fmt_refs(&oriented)));
            bounds.push(w.emit(format!("FACE_BOUND('',#{lid},.T.)")));
        }
        face_ids.push(w.emit(format!(
            "ADVANCED_FACE('',({}),#{surf_id},.T.)",
            fmt_refs(&bounds)
        )));
    }

    let mut shell_ids = Vec::with_capacity(model.shells.len());
    for shell in &model.shells {
        let refs: Vec<usize> = shell.faces.iter().map(|fu| face_ids[fu.face]).collect();
        shell_ids.push(w.emit(format!("CLOSED_SHELL('',({}))", fmt_refs(&refs))));
    }

    let body = &model.bodies[0];
    if body.inner_shells.is_empty() {
        let _ = w.emit(format!(
            "MANIFOLD_SOLID_BREP('body',#{})",
            shell_ids[body.outer_shell]
        ));
    } else {
        let voids: Vec<usize> = body.inner_shells.iter().map(|&i| shell_ids[i]).collect();
        let _ = w.emit(format!(
            "BREP_WITH_VOIDS('',#{},({}))",
            shell_ids[body.outer_shell],
            fmt_refs(&voids)
        ));
    }

    out.extend(w.lines);
    out.push("ENDSEC;".into());
    out.push("END-ISO-10303-21;".into());
    Ok((
        out.join("\n") + "\n",
        FeatureCertificate {
            capability: NURBS_STEP_SOLID_CAPABILITY,
            complete: true,
            notes: vec![
                "nurbs_step_solid",
                "manifold_solid_brep_b_spline_faces",
                "line_edges_vertex_point",
                "no_aabb_oscad_solid",
                "general-nurbs-step-lifted-to-freeform-caps",
            ],
        },
    ))
}

fn refuse_solid_payloads(text: &str) -> Result<()> {
    refuse_mesh_payloads_common(text)?;
    if text.contains("OSCAD_SOLID") || text.contains("AABB") {
        return Err(refuse("OSCAD_SOLID / AABB descriptor refused"));
    }
    if !(text.contains("MANIFOLD_SOLID_BREP") || text.contains("BREP_WITH_VOIDS")) {
        return Err(refuse(
            "nurbs-step-solid/1 requires MANIFOLD_SOLID_BREP or BREP_WITH_VOIDS",
        ));
    }
    if !text.contains("CLOSED_SHELL") || !text.contains("ADVANCED_FACE") {
        return Err(refuse("Missing CLOSED_SHELL / ADVANCED_FACE"));
    }
    if text.contains("OPEN_SHELL") && !text.contains("CLOSED_SHELL") {
        return Err(refuse("Open-shell STEP is not nurbs-step-solid/1"));
    }
    Ok(())
}

fn entity_refs(token: &str) -> Vec<usize> {
    token
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn validate_linked_solid_graph(
    entities: &std::collections::BTreeMap<usize, (String, String)>,
) -> Result<()> {
    let surface_ids: std::collections::BTreeSet<usize> = entities
        .iter()
        .filter_map(|(id, (ty, _))| (ty == "B_SPLINE_SURFACE_WITH_KNOTS").then_some(*id))
        .collect();
    let mut face_ids = std::collections::BTreeSet::new();
    let mut face_surface_ids = std::collections::BTreeSet::new();
    for (id, (ty, args)) in entities {
        if ty != "ADVANCED_FACE" {
            continue;
        }
        let parts = crate::nurbs_step_shared::split_top_args(args);
        if parts.len() < 4 {
            return Err(refuse("ADVANCED_FACE argument graph incomplete"));
        }
        let bound_refs = entity_refs(&parts[1]);
        if bound_refs.is_empty()
            || !bound_refs.iter().any(|bound| {
                entities
                    .get(bound)
                    .is_some_and(|(bound_ty, _)| bound_ty == "FACE_OUTER_BOUND")
            })
        {
            return Err(refuse("ADVANCED_FACE missing linked FACE_OUTER_BOUND"));
        }
        for bound in bound_refs {
            let Some((bound_ty, bound_args)) = entities.get(&bound) else {
                return Err(refuse("ADVANCED_FACE bound reference broken"));
            };
            if bound_ty != "FACE_OUTER_BOUND" && bound_ty != "FACE_BOUND" {
                return Err(refuse("ADVANCED_FACE references non-bound entity"));
            }
            let Some(loop_id) = entity_refs(bound_args).first().copied() else {
                return Err(refuse("FACE_BOUND missing EDGE_LOOP reference"));
            };
            if !entities
                .get(&loop_id)
                .is_some_and(|(loop_ty, _)| loop_ty == "EDGE_LOOP")
            {
                return Err(refuse("FACE_BOUND EDGE_LOOP reference broken"));
            }
        }
        let Some(surface_id) = entity_refs(&parts[2]).first().copied() else {
            return Err(refuse("ADVANCED_FACE missing surface reference"));
        };
        if !surface_ids.contains(&surface_id) {
            return Err(refuse("ADVANCED_FACE B-spline surface reference broken"));
        }
        face_ids.insert(*id);
        face_surface_ids.insert(surface_id);
    }
    if face_ids.len() != surface_ids.len() || face_surface_ids != surface_ids {
        return Err(refuse(
            "Every B-spline surface must be linked by exactly one ADVANCED_FACE",
        ));
    }

    let mut shell_face_ids = std::collections::BTreeSet::new();
    let mut closed_shell_ids = std::collections::BTreeSet::new();
    for (id, (ty, args)) in entities {
        if ty == "CLOSED_SHELL" {
            closed_shell_ids.insert(*id);
            for face_id in entity_refs(args) {
                if !face_ids.contains(&face_id) {
                    return Err(refuse("CLOSED_SHELL face reference broken"));
                }
                shell_face_ids.insert(face_id);
            }
        }
    }
    if shell_face_ids != face_ids {
        return Err(refuse(
            "CLOSED_SHELL must link every ADVANCED_FACE in the solid",
        ));
    }
    let body_shell_refs: std::collections::BTreeSet<usize> = entities
        .values()
        .filter(|(ty, _)| ty == "MANIFOLD_SOLID_BREP" || ty == "BREP_WITH_VOIDS")
        .flat_map(|(_, args)| entity_refs(args))
        .filter(|id| closed_shell_ids.contains(id))
        .collect();
    if body_shell_refs != closed_shell_ids {
        return Err(refuse(
            "Solid body must link every CLOSED_SHELL in the exchange graph",
        ));
    }
    Ok(())
}

fn match_bump_face_index(model: &Model, bump: &Surface) -> Result<usize> {
    let (bmin, bmax) = surface_aabb(bump);
    let bctr = [
        0.5 * (bmin[0] + bmax[0]),
        0.5 * (bmin[1] + bmax[1]),
        0.5 * (bmin[2] + bmax[2]),
    ];
    let mut best = None;
    let mut best_d = f64::INFINITY;
    for (i, face) in model.faces.iter().enumerate() {
        let (fmin, fmax) = surface_aabb(&face.surface);
        let fctr = [
            0.5 * (fmin[0] + fmax[0]),
            0.5 * (fmin[1] + fmax[1]),
            0.5 * (fmin[2] + fmax[2]),
        ];
        let d = (fctr[0] - bctr[0])
            .hypot(fctr[1] - bctr[1])
            .hypot(fctr[2] - bctr[2]);
        if d < best_d {
            best_d = d;
            best = Some(i);
        }
    }
    best.ok_or_else(|| refuse("No face matched bump surface"))
}

fn rebuild_from_surfaces(surfaces: &[Surface]) -> Result<Model> {
    let planar: Vec<_> = surfaces.iter().map(is_planar_surface).collect();
    let nonplanar: Vec<usize> = planar
        .iter()
        .enumerate()
        .filter_map(|(i, p)| if !*p { Some(i) } else { None })
        .collect();

    if surfaces.len() == 6 && nonplanar.is_empty() {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for s in surfaces {
            let (a, b) = surface_aabb(s);
            for i in 0..3 {
                min[i] = min[i].min(a[i]);
                max[i] = max[i].max(b[i]);
            }
        }
        return freeform_cuboid_solid(min, max);
    }

    if surfaces.len() == 6 && nonplanar.len() == 1 {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for (i, s) in surfaces.iter().enumerate() {
            if nonplanar[0] == i {
                continue;
            }
            let (a, b) = surface_aabb(s);
            for j in 0..3 {
                min[j] = min[j].min(a[j]);
                max[j] = max[j].max(b[j]);
            }
        }
        let bump = &surfaces[nonplanar[0]];
        // Corners of the bump lie on the cuboid face.
        for p in [
            &bump.control_points[0][0],
            &bump.control_points[0][3],
            &bump.control_points[3][0],
            &bump.control_points[3][3],
        ] {
            for j in 0..3 {
                min[j] = min[j].min(p[j]);
                max[j] = max[j].max(p[j]);
            }
        }
        let mut model = freeform_cuboid_solid(min, max)?;
        let idx = match_bump_face_index(&model, bump).unwrap_or(1);
        model.faces[idx].surface = bump.clone();
        model.validate()?;
        return Ok(model);
    }

    if surfaces.len() == 12 && nonplanar.is_empty() {
        // Cavity: partition into two AABB cuboids by clustering face AABB centers along volume.
        let mut face_boxes: Vec<([f64; 3], [f64; 3])> = surfaces.iter().map(surface_aabb).collect();
        // Overall AABB = outer; find smaller cluster as inner by taking faces whose centers
        // are strictly inside overall AABB inset.
        let mut omin = [f64::INFINITY; 3];
        let mut omax = [f64::NEG_INFINITY; 3];
        for (a, b) in &face_boxes {
            for i in 0..3 {
                omin[i] = omin[i].min(a[i]);
                omax[i] = omax[i].max(b[i]);
            }
        }
        // Inner cuboid: faces whose midpoints are not on the outer boundary planes
        let mut imin = [f64::INFINITY; 3];
        let mut imax = [f64::NEG_INFINITY; 3];
        let mut inner_count = 0usize;
        for (a, b) in &face_boxes {
            let mid = [
                0.5 * (a[0] + b[0]),
                0.5 * (a[1] + b[1]),
                0.5 * (a[2] + b[2]),
            ];
            let on_outer =
                (0..3).any(|i| (mid[i] - omin[i]).abs() < 1e-6 || (mid[i] - omax[i]).abs() < 1e-6);
            if !on_outer {
                inner_count += 1;
                for i in 0..3 {
                    imin[i] = imin[i].min(a[i]);
                    imax[i] = imax[i].max(b[i]);
                }
            }
        }
        if inner_count < 3 || !imin.iter().all(|v| v.is_finite()) {
            // Fallback: half the faces by sorting mid Z / volume — use median split of face volumes
            face_boxes.sort_by(|a, b| {
                let va = (a.1[0] - a.0[0]) * (a.1[1] - a.0[1]) * (a.1[2] - a.0[2]);
                let vb = (b.1[0] - b.0[0]) * (b.1[1] - b.0[1]) * (b.1[2] - b.0[2]);
                va.partial_cmp(&vb).unwrap()
            });
            // Smaller 6 faces form inner
            imin = [f64::INFINITY; 3];
            imax = [f64::NEG_INFINITY; 3];
            for (a, b) in face_boxes.iter().take(6) {
                for i in 0..3 {
                    imin[i] = imin[i].min(a[i]);
                    imax[i] = imax[i].max(b[i]);
                }
            }
        }
        let outer = crate::cuboid(omin, omax)?;
        let inner = crate::cuboid(imin, imax)?;
        let mut result = crate::imprint_pipeline::cavity(
            &outer,
            &inner,
            outer.tolerance_mm.max(inner.tolerance_mm),
        )?;
        for face in &mut result.faces {
            if let Some(elev) = as_uniform_bicubic(&face.surface) {
                face.surface = elev;
            }
        }
        result.validate()?;
        return Ok(result);
    }

    Err(refuse(
        "nurbs-step-solid/1 import admits 6-face cuboid/bump or 12-face planar cavity only",
    ))
}

/// Import freeform solid STEP → Model.
pub fn import_nurbs_step_solid(text: &str) -> Result<(Model, FeatureCertificate)> {
    refuse_solid_payloads(text)?;
    let entities = parse_entities(text);
    validate_linked_solid_graph(&entities)?;
    let surfs = parse_b_spline_surfaces(&entities)?;
    let surfaces: Vec<Surface> = surfs
        .into_iter()
        .map(|(_, s)| {
            as_uniform_bicubic(&s)
                .ok_or_else(|| refuse("Imported solid face outside bicubic/elevatable matrix"))
        })
        .collect::<Result<Vec<_>>>()?;
    for s in &surfaces {
        if !is_uniform_bicubic_positive(s) {
            return Err(refuse(
                "nurbs-step-solid/1 imported surfaces must be uniform bicubic w≡1",
            ));
        }
    }
    let model = rebuild_from_surfaces(&surfaces)?;
    Ok((
        model,
        FeatureCertificate {
            capability: NURBS_STEP_SOLID_CAPABILITY,
            complete: true,
            notes: vec![
                "nurbs_step_solid_import",
                "manifold_solid_brep_b_spline_faces",
                "no_aabb_oscad_solid",
            ],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeform_cuboid_topology_roundtrip() {
        let model = freeform_cuboid_solid([0., 0., 0.], [3., 2., 1.]).unwrap();
        let r = model.validate().unwrap();
        assert_eq!(
            (r.vertex_count, r.edge_count, r.face_count, r.body_count),
            (8, 12, 6, 1)
        );
        assert!(
            model
                .faces
                .iter()
                .all(|f| is_uniform_bicubic_positive(&f.surface))
        );
        let (text, cert) = export_nurbs_step_solid(&model).unwrap();
        assert!(cert.complete);
        assert_eq!(cert.capability, NURBS_STEP_SOLID_CAPABILITY);
        assert!(text.contains("MANIFOLD_SOLID_BREP"));
        assert!(text.contains("CLOSED_SHELL"));
        assert!(text.contains("B_SPLINE_SURFACE_WITH_KNOTS"));
        assert!(text.contains("VERTEX_POINT"));
        assert!(text.contains("LINE("));
        assert!(!text.contains("OSCAD_SOLID"));
        assert!(!text.contains("AABB"));
        let (back, icert) = import_nurbs_step_solid(&text).unwrap();
        assert!(icert.complete);
        let r2 = back.validate().unwrap();
        assert_eq!(
            (r2.vertex_count, r2.edge_count, r2.face_count, r2.body_count),
            (8, 12, 6, 1)
        );
    }

    #[test]
    fn freeform_cuboid_bump_roundtrip() {
        let model = freeform_cuboid_with_bump_face([0., 0., 0.], [2., 2., 2.]).unwrap();
        assert_eq!(model.faces.len(), 6);
        assert_eq!(
            model
                .faces
                .iter()
                .filter(|f| !is_planar_surface(&f.surface))
                .count(),
            1
        );
        let (text, _) = export_nurbs_step_solid(&model).unwrap();
        let (back, _) = import_nurbs_step_solid(&text).unwrap();
        back.validate().unwrap();
        assert_eq!(back.faces.len(), 6);
        assert_eq!(back.vertices.len(), 8);
        assert_eq!(back.edges.len(), 12);
        assert_eq!(
            back.faces
                .iter()
                .filter(|f| !is_planar_surface(&f.surface))
                .count(),
            1
        );
    }

    #[test]
    fn a4_boolean_difference_step_roundtrip() {
        let a = freeform_cuboid_solid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let b0 = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 1.],
                [0., 0., 1., 1.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (diff, cert) = crate::nurbs_boolean_imprint_solids(&a, &b, "difference").unwrap();
        assert!(cert.boolean_imprint);
        let (text, ecert) = export_nurbs_step_solid(&diff).unwrap();
        assert!(ecert.complete);
        assert!(!text.contains("OSCAD_SOLID"));
        assert!(!text.contains("AABB"));
        assert!(text.contains("B_SPLINE_SURFACE_WITH_KNOTS"));
        assert!(text.contains("BREP_WITH_VOIDS") || text.contains("MANIFOLD_SOLID_BREP"));
        let (back, icert) = import_nurbs_step_solid(&text).unwrap();
        assert!(icert.complete);
        back.validate().unwrap();
        assert!(!back.bodies.is_empty());
    }

    #[test]
    fn refuses_unlinked_freeform_solid_graph() {
        let model = freeform_cuboid_solid([0., 0., 0.], [3., 2., 1.]).unwrap();
        let (text, _) = export_nurbs_step_solid(&model).unwrap();
        let broken = text.replacen("CLOSED_SHELL('',(#", "CLOSED_SHELL('',(#999,#", 1);
        assert_eq!(
            import_nurbs_step_solid(&broken).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
    }

    #[test]
    fn refuses_missing_surface_and_out_of_matrix_face() {
        let model = freeform_cuboid_solid([0., 0., 0.], [3., 2., 1.]).unwrap();
        let (text, _) = export_nurbs_step_solid(&model).unwrap();
        let broken = text.replacen(
            "B_SPLINE_SURFACE_WITH_KNOTS",
            "UNSUPPORTED_SPLINE_SURFACE",
            1,
        );
        assert_eq!(
            import_nurbs_step_solid(&broken).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );

        let mut high_degree = model;
        high_degree.faces[0].surface = high_degree.faces[0]
            .surface
            .edit_axis(nurbs_core::surface::Axis::U, |curve| curve.elevate(4))
            .unwrap();
        high_degree.validate().unwrap();
        assert_eq!(
            export_nurbs_step_solid(&high_degree).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
    }
}
