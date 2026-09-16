//! Freeform NURBS STEP for a single trimmed bicubic open face (outer + one hole).
//!
//! Capability `nurbs-step-trimmed-bicubic/1`. Peer to `nurbs-step-bicubic-face/1`;
//! constructor solids stay on `step_interchange`. No MANIFOLD_SOLID_BREP.

use crate::analytic_features::FeatureCertificate;
use crate::nurbs_step_shared::{
    StepWriter, corner_xyz, emit_b_spline_surface, fmt_refs, invert_uv,
    is_uniform_bicubic_positive, parse_b_spline_surface, parse_entities, refuse,
    refuse_mesh_payloads_common, resolve_cartesian, split_top_args, step_header,
};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Result, surface::Surface};

pub const NURBS_STEP_TRIMMED_BICUBIC_CAPABILITY: &str = "nurbs-step-trimmed-bicubic/1";

fn admit_hole_uv(hole_uv: [[f64; 2]; 4]) -> Result<()> {
    for p in &hole_uv {
        if !(p[0].is_finite() && p[1].is_finite()) {
            return Err(refuse("hole_uv must be finite"));
        }
        if p[0] <= 1e-9 || p[0] >= 1. - 1e-9 || p[1] <= 1e-9 || p[1] >= 1. - 1e-9 {
            return Err(refuse(
                "nurbs-step-trimmed-bicubic/1 hole_uv must lie strictly inside (0,1)²",
            ));
        }
    }
    Ok(())
}

/// One bicubic face with outer 4 verts/edges + hole 4 verts/edges; `face.holes = [1]`.
pub fn bicubic_trimmed_face(surface: Surface, hole_uv: [[f64; 2]; 4]) -> Result<Model> {
    if !is_uniform_bicubic_positive(&surface) {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 admits uniform bicubic w≡1 non-periodic only",
        ));
    }
    admit_hole_uv(hole_uv)?;
    let outer_uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut corners = Vec::with_capacity(8);
    for uv in &outer_uv {
        corners.push(corner_xyz(&surface, uv[0], uv[1])?);
    }
    for uv in &hole_uv {
        corners.push(corner_xyz(&surface, uv[0], uv[1])?);
    }
    let mut edges = Vec::new();
    let mut outer_coedges = Vec::new();
    for i in 0..4 {
        let j = (i + 1) % 4;
        edges.push(Edge {
            degenerate: false,
            vertices: [i, j],
            curve: crate::line(corners[i].to_vec(), corners[j].to_vec()),
        });
        outer_coedges.push(Coedge {
            edge: i,
            reversed: false,
            pcurve: crate::line(outer_uv[i].to_vec(), outer_uv[j].to_vec()),
        });
    }
    let mut hole_coedges = Vec::new();
    // Hole UV loop must be CW (outer is CCW).
    let hole_order = [0usize, 3, 2, 1];
    for k in 0..4 {
        let i = hole_order[k];
        let j = hole_order[(k + 1) % 4];
        let a = 4 + i;
        let b = 4 + j;
        let ei = edges.len();
        edges.push(Edge {
            degenerate: false,
            vertices: [a, b],
            curve: crate::line(corners[a].to_vec(), corners[b].to_vec()),
        });
        hole_coedges.push(Coedge {
            edge: ei,
            reversed: false,
            pcurve: crate::line(hole_uv[i].to_vec(), hole_uv[j].to_vec()),
        });
    }
    let mut model = Model(
        brep_topology::Model {
            vertices: corners.into_iter().map(|point| Vertex { point }).collect(),
            edges,
            loops: vec![
                Loop {
                    coedges: outer_coedges,
                },
                Loop {
                    coedges: hole_coedges,
                },
            ],
            faces: vec![Face {
                surface,
                outer: 0,
                holes: vec![1],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

fn admit_trimmed_face(model: &Model) -> Result<&Surface> {
    model.validate()?;
    if !model.bodies.is_empty() {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 refuses solids / MANIFOLD bodies",
        ));
    }
    if model.shells.len() != 1
        || model.faces.len() != 1
        || model.edges.len() != 8
        || model.vertices.len() != 8
        || model.loops.len() != 2
    {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 admits one open trimmed face (8V/8E/2 loops) only",
        ));
    }
    let face = &model.faces[0];
    if face.holes != [1] {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 requires face.holes = [1]",
        ));
    }
    if !is_uniform_bicubic_positive(&face.surface) {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 surface must be uniform bicubic w≡1",
        ));
    }
    if model.edges.iter().any(|e| e.curve.degree != 1) {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 boundary edges must be LINE (degree 1)",
        ));
    }
    Ok(&face.surface)
}

/// Export trimmed bicubic open face: B_SPLINE + FACE_OUTER_BOUND + FACE_BOUND + OPEN_SHELL.
pub fn export_nurbs_step_trimmed(model: &Model) -> Result<(String, FeatureCertificate)> {
    let surface = admit_trimmed_face(model)?.clone();
    let mut w = StepWriter::new();
    let mut out = step_header(
        "nurbs-trimmed-bicubic.step",
        "OpenSCAD Viewer freeform NURBS trimmed face",
    );
    let surf_id = emit_b_spline_surface(&mut w, &surface);

    let mut vid = Vec::with_capacity(8);
    let mut pid = Vec::with_capacity(8);
    for v in &model.vertices {
        let p = w.cartesian(v.point);
        pid.push(p);
        vid.push(w.vertex_point(p));
    }

    let mut emit_loop = |loop_index: usize| -> usize {
        let mut edge_ids = Vec::with_capacity(4);
        for coedge in &model.loops[loop_index].coedges {
            let edge = &model.edges[coedge.edge];
            let (a, b) = if coedge.reversed {
                (edge.vertices[1], edge.vertices[0])
            } else {
                (edge.vertices[0], edge.vertices[1])
            };
            let pa = model.vertices[a].point;
            let pb = model.vertices[b].point;
            let dir = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let uv_a = [
                coedge.pcurve.control_points[0][0],
                coedge.pcurve.control_points[0][1],
            ];
            let uv_b = [
                coedge.pcurve.control_points[1][0],
                coedge.pcurve.control_points[1][1],
            ];
            edge_ids.push(w.pcurve_line_edge(vid[a], vid[b], pid[a], dir, surf_id, uv_a, uv_b));
        }
        let oriented: Vec<usize> = edge_ids
            .iter()
            .map(|&e| w.emit(format!("ORIENTED_EDGE('',*,*,#{e},.T.)")))
            .collect();
        w.emit(format!("EDGE_LOOP('',({}))", fmt_refs(&oriented)))
    };

    let outer_loop = emit_loop(model.faces[0].outer);
    let hole_loop = emit_loop(model.faces[0].holes[0]);
    let outer_bound = w.emit(format!("FACE_OUTER_BOUND('',#{outer_loop},.T.)"));
    let hole_bound = w.emit(format!("FACE_BOUND('',#{hole_loop},.T.)"));
    let face = w.emit(format!(
        "ADVANCED_FACE('',(#{outer_bound},#{hole_bound}),#{surf_id},.T.)"
    ));
    let open_shell = w.emit(format!("OPEN_SHELL('',(#{face}))"));
    let _ = w.emit(format!("SHELL_BASED_SURFACE_MODEL('',(#{open_shell}))"));

    out.extend(w.lines);
    out.push("ENDSEC;".into());
    out.push("END-ISO-10303-21;".into());
    Ok((
        out.join("\n") + "\n",
        FeatureCertificate {
            capability: NURBS_STEP_TRIMMED_BICUBIC_CAPABILITY,
            complete: true,
            notes: vec![
                "nurbs_step_trimmed_bicubic",
                "face_outer_bound_plus_face_bound",
                "open_shell_no_manifold_solid",
                "general-nurbs-step-lifted-to-freeform-caps",
            ],
        },
    ))
}

fn refuse_trimmed_payloads(text: &str) -> Result<()> {
    refuse_mesh_payloads_common(text)?;
    if text.contains("MANIFOLD_SOLID_BREP") || text.contains("BREP_WITH_VOIDS") {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 refuses MANIFOLD_SOLID_BREP solids",
        ));
    }
    if !text.contains("FACE_OUTER_BOUND") || !text.contains("FACE_BOUND") {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 requires FACE_OUTER_BOUND + FACE_BOUND",
        ));
    }
    if !text.contains("ADVANCED_FACE") || !text.contains("OPEN_SHELL") {
        return Err(refuse(
            "Missing ADVANCED_FACE / OPEN_SHELL for trimmed face",
        ));
    }
    Ok(())
}

fn parse_hole_uv_from_vertices(
    entities: &std::collections::BTreeMap<usize, (String, String)>,
    surface: &Surface,
) -> Result<[[f64; 2]; 4]> {
    // Collect VERTEX_POINT → CARTESIAN, skip outer corners by distance to S(0|1,0|1).
    let mut pts = Vec::new();
    for (_id, (ty, args)) in entities {
        if ty != "VERTEX_POINT" {
            continue;
        }
        let parts = split_top_args(args);
        if parts.len() < 2 {
            continue;
        }
        let cid = parts[1]
            .trim()
            .trim_start_matches('#')
            .parse::<usize>()
            .ok();
        let Some(cid) = cid else { continue };
        if let Some(p) = resolve_cartesian(entities, cid) {
            pts.push(p);
        }
    }
    let outer = [
        corner_xyz(surface, 0., 0.)?,
        corner_xyz(surface, 1., 0.)?,
        corner_xyz(surface, 1., 1.)?,
        corner_xyz(surface, 0., 1.)?,
    ];
    let near_outer = |p: [f64; 3]| {
        outer
            .iter()
            .any(|o| (o[0] - p[0]).hypot(o[1] - p[1]).hypot(o[2] - p[2]) < 1e-6)
    };
    let mut hole_pts: Vec<[f64; 3]> = pts.into_iter().filter(|p| !near_outer(*p)).collect();
    // Dedup near-duplicates
    hole_pts.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap()
            .then(a[1].partial_cmp(&b[1]).unwrap())
            .then(a[2].partial_cmp(&b[2]).unwrap())
    });
    hole_pts.dedup_by(|a, b| {
        (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9 && (a[2] - b[2]).abs() < 1e-9
    });
    if hole_pts.len() < 4 {
        return Err(refuse("Trimmed STEP missing four hole vertices"));
    }
    let hole_pts = &hole_pts[..4];
    let mut uvs = Vec::with_capacity(4);
    for p in hole_pts {
        uvs.push(invert_uv(surface, *p)?);
    }
    // Recover CCW UV order; bicubic_trimmed_face reverses the hole wire to CW.
    let cx = uvs.iter().map(|u| u[0]).sum::<f64>() / 4.;
    let cy = uvs.iter().map(|u| u[1]).sum::<f64>() / 4.;
    uvs.sort_by(|a, b| {
        let aa = (a[1] - cy).atan2(a[0] - cx);
        let bb = (b[1] - cy).atan2(b[0] - cx);
        aa.partial_cmp(&bb).unwrap()
    });
    Ok([uvs[0], uvs[1], uvs[2], uvs[3]])
}

/// Import trimmed bicubic open-face STEP → Model (restores outer + holes).
pub fn import_nurbs_step_trimmed(text: &str) -> Result<(Model, FeatureCertificate)> {
    refuse_trimmed_payloads(text)?;
    let entities = parse_entities(text);
    let surface = parse_b_spline_surface(&entities)?;
    let hole_uv = parse_hole_uv_from_vertices(&entities, &surface)?;
    let model = bicubic_trimmed_face(surface, hole_uv)?;
    Ok((
        model,
        FeatureCertificate {
            capability: NURBS_STEP_TRIMMED_BICUBIC_CAPABILITY,
            complete: true,
            notes: vec![
                "nurbs_step_trimmed_bicubic_import",
                "face_outer_bound_plus_face_bound",
                "open_shell_no_manifold_solid",
            ],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bicubic() -> Surface {
        let row = |y: f64| {
            (0..4)
                .map(|i| vec![i as f64, y, 0.05 * (i as f64 + y)])
                .collect::<Vec<_>>()
        };
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![row(0.), row(1.), row(2.), row(3.)],
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn roundtrip_trimmed_hole_count() {
        let hole = [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
        let model = bicubic_trimmed_face(sample_bicubic(), hole).unwrap();
        assert_eq!(model.faces[0].holes.len(), 1);
        assert_eq!(model.vertices.len(), 8);
        assert_eq!(model.edges.len(), 8);
        assert!(model.bodies.is_empty());
        let (text, cert) = export_nurbs_step_trimmed(&model).unwrap();
        assert!(cert.complete);
        assert_eq!(cert.capability, NURBS_STEP_TRIMMED_BICUBIC_CAPABILITY);
        assert!(text.contains("FACE_OUTER_BOUND"));
        assert!(text.contains("FACE_BOUND"));
        assert_eq!(text.matches("PCURVE(").count(), 8);
        assert_eq!(text.matches("SURFACE_CURVE(").count(), 8);
        assert!(text.contains("OPEN_SHELL"));
        assert!(!text.contains("MANIFOLD_SOLID_BREP"));
        let (back, icert) = import_nurbs_step_trimmed(&text).unwrap();
        assert!(icert.complete);
        back.validate().unwrap();
        assert_eq!(back.faces[0].holes.len(), 1);
        assert_eq!(back.loops.len(), 2);
        assert!(back.bodies.is_empty());
    }

    #[test]
    fn refuses_solid_export_and_import() {
        let solid = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        assert_eq!(
            export_nurbs_step_trimmed(&solid).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
        let text = r#"ISO-10303-21;
HEADER;
ENDSEC;
DATA;
#1=B_SPLINE_SURFACE_WITH_KNOTS('',3,3,((#2,#3,#4,#5),(#6,#7,#8,#9),(#10,#11,#12,#13),(#14,#15,#16,#17)),.UNSPECIFIED.,.UNSPECIFIED.,.F.,.F.,.F.,(1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.,1.),(4,4),(4,4),(0.,1.),(0.,1.),.UNSPECIFIED.);
#18=MANIFOLD_SOLID_BREP('body',#19);
#19=CLOSED_SHELL('',(#20));
#20=ADVANCED_FACE('',(#21),#1,.T.);
ENDSEC;
END-ISO-10303-21;
"#;
        assert_eq!(
            import_nurbs_step_trimmed(text).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
    }
}
