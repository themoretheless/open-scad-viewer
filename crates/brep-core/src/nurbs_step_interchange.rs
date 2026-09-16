//! Freeform NURBS STEP for a single untrimmed positive-weight bicubic open face.
//!
//! Capability `nurbs-step-bicubic-face/1`. Constructor solids stay on
//! `step_interchange` / `step-interchange/1`. No MANIFOLD_SOLID_BREP, no holes,
//! no rational weights ≠ 1, no non-bicubic surfaces.

use crate::analytic_features::FeatureCertificate;
use crate::nurbs_step_shared::{
    StepWriter, corner_xyz, emit_b_spline_surface, fmt_refs, is_uniform_bicubic_positive,
    parse_b_spline_surface, parse_entities, refuse, refuse_mesh_payloads_common, step_header,
};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Result, surface::Surface};

pub const NURBS_STEP_BICUBIC_FACE_CAPABILITY: &str = "nurbs-step-bicubic-face/1";

/// Build the admitted open-face model from a uniform bicubic surface.
pub fn bicubic_open_face(surface: Surface) -> Result<Model> {
    if !is_uniform_bicubic_positive(&surface) {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 admits uniform bicubic w≡1 non-periodic only",
        ));
    }
    let corners = [
        corner_xyz(&surface, 0., 0.)?,
        corner_xyz(&surface, 1., 0.)?,
        corner_xyz(&surface, 1., 1.)?,
        corner_xyz(&surface, 0., 1.)?,
    ];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut edges = Vec::new();
    let mut coedges = Vec::new();
    for i in 0..4 {
        let j = (i + 1) % 4;
        edges.push(Edge {
            degenerate: false,
            vertices: [i, j],
            curve: crate::line(corners[i].to_vec(), corners[j].to_vec()),
        });
        coedges.push(Coedge {
            edge: i,
            reversed: false,
            pcurve: crate::line(uv[i].to_vec(), uv[j].to_vec()),
        });
    }
    let mut model = Model(
        brep_topology::Model {
            vertices: corners.map(|point| Vertex { point }).to_vec(),
            edges,
            loops: vec![Loop { coedges }],
            faces: vec![Face {
                surface,
                outer: 0,
                holes: vec![],
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

fn admit_open_face(model: &Model) -> Result<&Surface> {
    model.validate()?;
    if !model.bodies.is_empty() {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 refuses solids / MANIFOLD bodies",
        ));
    }
    if model.shells.len() != 1
        || model.faces.len() != 1
        || model.edges.len() != 4
        || model.vertices.len() != 4
        || model.loops.len() != 1
    {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 admits one open face (4V/4E/1 loop) only",
        ));
    }
    let face = &model.faces[0];
    if !face.holes.is_empty() {
        return Err(refuse("nurbs-step-bicubic-face/1 refuses holes / trims"));
    }
    if !is_uniform_bicubic_positive(&face.surface) {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 surface must be uniform bicubic w≡1",
        ));
    }
    if model.edges.iter().any(|e| e.curve.degree != 1) {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 boundary edges must be LINE (degree 1)",
        ));
    }
    Ok(&face.surface)
}

/// Export admitted bicubic open face as AP214/AP242 Part 21 with B_SPLINE entities.
pub fn export_nurbs_step(model: &Model) -> Result<(String, FeatureCertificate)> {
    let surface = admit_open_face(model)?.clone();
    let mut w = StepWriter::new();
    let mut out = step_header(
        "nurbs-bicubic-face.step",
        "OpenSCAD Viewer freeform NURBS face",
    );

    let surf_id = emit_b_spline_surface(&mut w, &surface);

    let mut vid = Vec::with_capacity(4);
    let mut pid = Vec::with_capacity(4);
    for v in &model.vertices {
        let p = w.cartesian(v.point);
        pid.push(p);
        vid.push(w.vertex_point(p));
    }
    let mut edge_ids = Vec::with_capacity(4);
    for i in 0..4 {
        let j = (i + 1) % 4;
        let a = model.vertices[i].point;
        let b = model.vertices[j].point;
        let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        edge_ids.push(w.line_edge(vid[i], vid[j], pid[i], dir));
    }
    let oriented: Vec<usize> = edge_ids
        .iter()
        .map(|&e| w.emit(format!("ORIENTED_EDGE('',*,*,#{e},.T.)")))
        .collect();
    let loop_id = w.emit(format!("EDGE_LOOP('',({}))", fmt_refs(&oriented)));
    let bound = w.emit(format!("FACE_OUTER_BOUND('',#{loop_id},.T.)"));
    let face = w.emit(format!("ADVANCED_FACE('',(#{bound}),#{surf_id},.T.)"));
    let open_shell = w.emit(format!("OPEN_SHELL('',(#{face}))"));
    let _ = w.emit(format!("SHELL_BASED_SURFACE_MODEL('',(#{open_shell}))"));

    out.extend(w.lines);
    out.push("ENDSEC;".into());
    out.push("END-ISO-10303-21;".into());
    Ok((
        out.join("\n") + "\n",
        FeatureCertificate {
            capability: NURBS_STEP_BICUBIC_FACE_CAPABILITY,
            complete: true,
            notes: vec![
                "nurbs_step_bicubic_open_face",
                "b_spline_surface_with_knots",
                "open_shell_no_manifold_solid",
                "no_aabb_oscad_solid",
            ],
        },
    ))
}

fn refuse_open_face_payloads(text: &str) -> Result<()> {
    refuse_mesh_payloads_common(text)?;
    if text.contains("MANIFOLD_SOLID_BREP") || text.contains("BREP_WITH_VOIDS") {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 refuses MANIFOLD_SOLID_BREP solids",
        ));
    }
    if text.contains("FACE_BOUND") && !text.contains("FACE_OUTER_BOUND") {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 refuses hole-only FACE_BOUND",
        ));
    }
    if text.contains("FACE_BOUND(") {
        return Err(refuse(
            "nurbs-step-bicubic-face/1 refuses trimmed faces; use nurbs-step-trimmed-bicubic/1",
        ));
    }
    if !text.contains("ADVANCED_FACE") || !text.contains("OPEN_SHELL") {
        return Err(refuse("Missing ADVANCED_FACE / OPEN_SHELL for open face"));
    }
    Ok(())
}

/// Import bicubic open-face STEP → Model.
pub fn import_nurbs_step(text: &str) -> Result<(Model, FeatureCertificate)> {
    refuse_open_face_payloads(text)?;
    let entities = parse_entities(text);
    let surface = parse_b_spline_surface(&entities)?;
    let model = bicubic_open_face(surface)?;
    Ok((
        model,
        FeatureCertificate {
            capability: NURBS_STEP_BICUBIC_FACE_CAPABILITY,
            complete: true,
            notes: vec![
                "nurbs_step_bicubic_open_face_import",
                "b_spline_surface_with_knots",
                "open_shell_no_manifold_solid",
                "no_aabb_oscad_solid",
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
                .map(|i| vec![i as f64, y, 0.1 * (i as f64 + y)])
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
    fn roundtrip_bicubic_open_face() {
        let model = bicubic_open_face(sample_bicubic()).unwrap();
        let (text, cert) = export_nurbs_step(&model).unwrap();
        assert!(cert.complete);
        assert_eq!(cert.capability, NURBS_STEP_BICUBIC_FACE_CAPABILITY);
        assert!(text.contains("B_SPLINE_SURFACE_WITH_KNOTS"));
        assert!(text.contains("OPEN_SHELL"));
        assert!(text.contains("SHELL_BASED_SURFACE_MODEL"));
        assert!(text.contains("ADVANCED_FACE"));
        assert!(!text.contains("MANIFOLD_SOLID_BREP"));
        assert!(!text.contains("OSCAD_SOLID"));
        assert!(!text.contains("FACETED_BREP"));
        let (back, icert) = import_nurbs_step(&text).unwrap();
        assert!(icert.complete);
        back.validate().unwrap();
        assert_eq!(back.faces.len(), 1);
        assert_eq!(back.edges.len(), 4);
        assert_eq!(back.vertices.len(), 4);
        assert!(back.bodies.is_empty());
        let a = &model.faces[0].surface;
        let b = &back.faces[0].surface;
        assert_eq!(a.degree_u, b.degree_u);
        assert_eq!(a.degree_v, b.degree_v);
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..3 {
                    assert!(
                        (a.control_points[i][j][k] - b.control_points[i][j][k]).abs() < 1e-9,
                        "cp[{i}][{j}][{k}]"
                    );
                }
                assert!((a.weights[i][j] - b.weights[i][j]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn refuses_constructor_solid_export() {
        let solid = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        assert_eq!(
            export_nurbs_step(&solid).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
    }

    #[test]
    fn refuses_rational_surface() {
        let mut s = sample_bicubic();
        s.weights[0][0] = 2.;
        assert!(bicubic_open_face(s).is_err());
    }

    #[test]
    fn refuses_manifold_solid_import() {
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
            import_nurbs_step(text).unwrap_err().code,
            "BREP_NURBS_STEP_REFUSED"
        );
    }

    #[test]
    fn refuses_faceted() {
        assert!(
            import_nurbs_step(
                "ISO-10303-21;\nDATA;\n#1=FACETED_BREP('',#2);\nENDSEC;\nEND-ISO-10303-21;\n"
            )
            .is_err()
        );
    }
}
