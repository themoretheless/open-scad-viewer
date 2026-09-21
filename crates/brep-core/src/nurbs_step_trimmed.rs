//! Freeform NURBS STEP for a single trimmed bicubic open face (outer + one hole).
//!
//! Capability `nurbs-step-trimmed-bicubic/1`. Peer to `nurbs-step-bicubic-face/1`;
//! constructor solids stay on `step_interchange`. No MANIFOLD_SOLID_BREP.

use crate::analytic_features::FeatureCertificate;
use crate::nurbs_step_shared::{
    PcurveLineEdge, StepGraphRoot, StepWriter, corner_xyz, emit_b_spline_surface, fmt_refs,
    invert_uv, is_uniform_bicubic_positive, parse_entities, refuse, refuse_mesh_payloads_common,
    resolve_cartesian, split_top_args, step_header, surface_from_b_spline_args,
    validate_linked_step_graph,
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
            edge_ids.push(w.pcurve_line_edge(PcurveLineEdge {
                va: vid[a],
                vb: vid[b],
                point: pid[a],
                direction: dir,
                surface: surf_id,
                uv_a,
                uv_b,
            }));
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
    vertex_ids: &[usize],
) -> Result<[[f64; 2]; 4]> {
    if vertex_ids.len() != 4 {
        return Err(refuse(
            "Trimmed STEP hole loop must link exactly four vertices",
        ));
    }
    let mut hole_pts = Vec::with_capacity(4);
    for vertex_id in vertex_ids {
        let (ty, args) = entities
            .get(vertex_id)
            .ok_or_else(|| refuse("FACE_BOUND hole vertex reference broken"))?;
        if ty != "VERTEX_POINT" {
            return Err(refuse("FACE_BOUND hole vertex has wrong type"));
        }
        let parts = split_top_args(args);
        if parts.len() != 2 {
            return Err(refuse("FACE_BOUND hole VERTEX_POINT malformed"));
        }
        let cid = parts[1]
            .trim()
            .trim_start_matches('#')
            .parse::<usize>()
            .map_err(|_| refuse("FACE_BOUND hole point reference malformed"))?;
        hole_pts.push(
            resolve_cartesian(entities, cid)
                .ok_or_else(|| refuse("FACE_BOUND hole point reference broken"))?,
        );
    }
    let mut uvs = Vec::with_capacity(4);
    for p in &hole_pts {
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
    let graph = validate_linked_step_graph(&entities, StepGraphRoot::OpenShell)?;
    if graph.faces.len() != 1
        || graph.faces[0].outer_vertex_ids.len() != 4
        || graph.faces[0].hole_vertex_ids.len() != 1
    {
        return Err(refuse(
            "nurbs-step-trimmed-bicubic/1 requires one linked outer loop and one hole loop",
        ));
    }
    let linked_face = &graph.faces[0];
    let surface_args = &entities
        .get(&linked_face.surface_id)
        .ok_or_else(|| refuse("Linked face surface disappeared"))?
        .1;
    let surface = surface_from_b_spline_args(&entities, surface_args)?;
    if !is_uniform_bicubic_positive(&surface) {
        return Err(refuse(
            "Imported surface outside freeform NURBS STEP (bicubic w≡1)",
        ));
    }
    let hole_uv =
        parse_hole_uv_from_vertices(&entities, &surface, &linked_face.hole_vertex_ids[0])?;
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

    #[test]
    fn refuses_orphan_vertex_and_broken_hole_loop_type() {
        let hole = [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
        let model = bicubic_trimmed_face(sample_bicubic(), hole).unwrap();
        let (text, _) = export_nurbs_step_trimmed(&model).unwrap();

        let with_orphan = text.replace(
            "ENDSEC;\nEND-ISO-10303-21;",
            "#999999=VERTEX_POINT('',#1);\nENDSEC;\nEND-ISO-10303-21;",
        );
        assert_eq!(
            import_nurbs_step_trimmed(&with_orphan).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );

        let bound_line = text
            .lines()
            .find(|line| line.contains("=FACE_BOUND("))
            .unwrap();
        let loop_id = bound_line
            .split_once("',#")
            .unwrap()
            .1
            .split(',')
            .next()
            .unwrap();
        let broken = text.replacen(
            &format!("#{loop_id}=EDGE_LOOP("),
            &format!("#{loop_id}=DIRECTION("),
            1,
        );
        assert_eq!(
            import_nurbs_step_trimmed(&broken).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
    }

    #[test]
    fn refuses_broken_independent_pcurve_correspondence() {
        let hole = [[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]];
        let model = bicubic_trimmed_face(sample_bicubic(), hole).unwrap();
        let (text, _) = export_nurbs_step_trimmed(&model).unwrap();
        let broken = text.replacen(
            "CARTESIAN_POINT('',(0.000000000000000,0.000000000000000))",
            "CARTESIAN_POINT('',(0.200000000000000,0.000000000000000))",
            1,
        );
        let error = import_nurbs_step_trimmed(&broken).unwrap_err();
        assert_eq!(error.code, "BREP_NURBS_STEP_REFUSED");
        assert!(error.message.contains("correspondence") || error.message.contains("endpoints"));
    }
}
