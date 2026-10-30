//! STEP `/3`–`/9` writer, periodic carrier reconstruction, and `/8` regularity certification.

use crate::analytic_features::FeatureCertificate;
use crate::step_interchange_v3::{
    MAX_OUTPUT_BYTES, STEP_INTERCHANGE_V3_CAPABILITY, STEP_INTERCHANGE_V4_CAPABILITY,
    STEP_INTERCHANGE_V5_CAPABILITY, STEP_INTERCHANGE_V6_CAPABILITY,
    STEP_INTERCHANGE_V7_CAPABILITY, STEP_INTERCHANGE_V8_CAPABILITY,
    STEP_INTERCHANGE_V9_CAPABILITY, StepRegularityEvidence, StepV3Report, StepV8Certificate,
    entity_digest, refuse,
};
use crate::step_part21::{Value, parse, render_entity_value};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, TopoId, TopoKind};
use nurbs_core::{Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Writer {
    next: usize,
    pub(crate) rows: BTreeMap<usize, String>,
}
impl Writer {
    pub(crate) fn new() -> Self {
        Self {
            next: 1,
            rows: BTreeMap::new(),
        }
    }
    fn reserve(&mut self) -> usize {
        let id = self.next;
        self.next += 1;
        id
    }
    fn set(&mut self, id: usize, body: String) {
        self.rows.insert(id, body);
    }
    pub(crate) fn emit(&mut self, body: String) -> usize {
        let id = self.reserve();
        self.set(id, body);
        id
    }
    pub(crate) fn point(&mut self, p: &[f64]) -> usize {
        self.emit(format!("CARTESIAN_POINT('',({}))", floats(p)))
    }
}
fn floats(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:.17e}").replace('e', "E"))
        .collect::<Vec<_>>()
        .join(",")
}
pub(crate) fn enforce_output_limit(text: &str) -> Result<()> {
    if text.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP output exceeds 16 MiB"));
    }
    Ok(())
}
fn quadratic_patch_knots(segments: usize) -> Vec<f64> {
    let mut knots = vec![0.; 3];
    for knot in 1..segments {
        knots.extend([knot as f64, knot as f64])
    }
    knots.extend([segments as f64; 3]);
    knots
}


// Face counts and degrees do not establish a periodic carrier: partial
// revolutions can have the same shape of patch records as a complete band.



fn constant_boundary(surface: &Surface, column: usize) -> bool {
    surface
        .control_points
        .iter()
        .all(|row| row[column] == surface.control_points[0][column])
}


pub(crate) fn canonical_face_senses(model: &Model) -> Result<Model> {
    let mut result = model.clone();
    let reversed = result
        .shells
        .iter()
        .flat_map(|shell| &shell.faces)
        .filter(|usage| usage.reversed)
        .map(|usage| usage.face)
        .collect::<BTreeSet<_>>();
    for face_index in reversed {
        let face = &mut result.0.faces[face_index];
        let sum = face.surface.knots_u[face.surface.degree_u]
            + face.surface.knots_u[face.surface.knots_u.len() - face.surface.degree_u - 1];
        face.surface.control_points.reverse();
        face.surface.weights.reverse();
        face.surface.knots_u.reverse();
        for knot in &mut face.surface.knots_u {
            *knot = sum - *knot
        }
        let mut loops = vec![face.outer];
        loops.extend(face.holes.iter().copied());
        for loop_index in loops {
            for coedge in &mut result.0.loops[loop_index].coedges {
                for point in &mut coedge.pcurve.control_points {
                    point[0] = sum - point[0]
                }
                coedge.pcurve = coedge.pcurve.reverse()?;
                coedge.reversed = !coedge.reversed;
            }
            result.0.loops[loop_index].coedges.reverse();
        }
    }
    for shell in &mut result.0.shells {
        for usage in &mut shell.faces {
            usage.reversed = false
        }
    }
    result.validate()?;
    Ok(result)
}
pub(crate) fn refs_text(v: &[usize]) -> String {
    v.iter()
        .map(|x| format!("#{x}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn knot_form(knots: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut m = Vec::new();
    let mut k = Vec::new();
    for &x in knots {
        if k.last()
            .is_some_and(|v: &f64| (*v - x).abs() <= f64::EPSILON)
        {
            *m.last_mut().unwrap() += 1;
        } else {
            k.push(x);
            m.push(1);
        }
    }
    (m, k)
}
fn emit_curve(w: &mut Writer, c: &Curve) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form(&c.knots);
    w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.F.,.{closed}.,.F.,({}),({}),({}),.UNSPECIFIED.)",
        c.degree, refs_text(&cps), floats(&c.weights), m.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&k),
        closed = if c.periodic { "T" } else { "F" }))
}
fn emit_surface(w: &mut Writer, s: &Surface) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form(&s.knots_u);
    let (mv, kv) = knot_form(&s.knots_v);
    w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.F.,.{pu}.,.{pv}.,.F.,.F.,({}),({}),({}),({}),({}),.UNSPECIFIED.)",
        s.degree_u, s.degree_v, floats(&s.weights.iter().flatten().copied().collect::<Vec<_>>()),
        mu.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        mv.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&ku), floats(&kv),
        pu=if s.periodic_u {"T"} else {"F"}, pv=if s.periodic_v {"T"} else {"F"}))
}
fn all_unit_weights(mut weights: impl Iterator<Item = f64>) -> bool {
    weights.all(|weight| (weight - 1.).abs() <= f64::EPSILON)
}
fn knot_form_exchange(knots: &[f64], exact: bool) -> (Vec<usize>, Vec<f64>) {
    if !exact {
        return knot_form(knots);
    }
    let mut counts = Vec::new();
    let mut values = Vec::new();
    for &knot in knots {
        if values.last() == Some(&knot) {
            *counts.last_mut().unwrap() += 1;
        } else {
            values.push(knot);
            counts.push(1);
        }
    }
    (counts, values)
}
pub(crate) fn emit_curve_v5(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, false)
}
pub(crate) fn emit_curve_source(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, true)
}
fn emit_curve_exchange(w: &mut Writer, c: &Curve, exact: bool) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form_exchange(&c.knots, exact);
    let mult = m
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let closed = if c.periodic { "T" } else { "F" };
    let unit = if exact {
        c.weights.iter().all(|&v| v == 1.)
    } else {
        all_unit_weights(c.weights.iter().copied())
    };
    if unit {
        w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.{closed}.,.F.,({mult}),({}),.UNSPECIFIED.)",
            c.degree,refs_text(&cps),floats(&k)))
    } else {
        w.emit(format!("(BOUNDED_CURVE()B_SPLINE_CURVE({},({}),.UNSPECIFIED.,.{closed}.,.F.)B_SPLINE_CURVE_WITH_KNOTS(({mult}),({}),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE(({}))REPRESENTATION_ITEM(''))",
            c.degree,refs_text(&cps),floats(&k),floats(&c.weights)))
    }
}
fn emit_surface_v5(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, false)
}
pub(crate) fn emit_surface_source(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, true)
}
fn emit_surface_exchange(w: &mut Writer, s: &Surface, exact: bool) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let weights = s
        .weights
        .iter()
        .map(|row| format!("({})", floats(row)))
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form_exchange(&s.knots_u, exact);
    let (mv, kv) = knot_form_exchange(&s.knots_v, exact);
    let mu = mu
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mv = mv
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let pu = if s.periodic_u { "T" } else { "F" };
    let pv = if s.periodic_v { "T" } else { "F" };
    let unit = if exact {
        s.weights.iter().flatten().all(|&v| v == 1.)
    } else {
        all_unit_weights(s.weights.iter().flatten().copied())
    };
    if unit {
        w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.,({mu}),({mv}),({}),({}),.UNSPECIFIED.)",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    } else {
        w.emit(format!("(BOUNDED_SURFACE()B_SPLINE_SURFACE({},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.)B_SPLINE_SURFACE_WITH_KNOTS(({mu}),({mv}),({}),({}),.UNSPECIFIED.)GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_SURFACE(({weights}))REPRESENTATION_ITEM('')SURFACE())",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    }
}

pub(crate) fn export_step_direct(
    model: &Model,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let mut canonical = canonical_face_senses(model)?;
    if capability == STEP_INTERCHANGE_V6_CAPABILITY {
        // /6 preserves the exact authored chart: retain the original face uses
        // so the self-authored import route applies the inverse mirror. Other
        // routes exchange the canonical forward chart directly.
        for (target, source) in canonical.shells.iter_mut().zip(&model.shells) {
            for (target, source) in target.faces.iter_mut().zip(&source.faces) {
                target.reversed = source.reversed;
            }
        }
    }
    let model = &canonical;
    let allow_open_shells = capability == STEP_INTERCHANGE_V9_CAPABILITY;
    if (!allow_open_shells && model.bodies.is_empty())
        || (!allow_open_shells && model.shells.iter().any(|s| !s.closed))
    {
        return Err(refuse(
            "STEP /3 through /8 export closed manifold bodies only",
        ));
    }
    if model.bodies.is_empty() && !model.shells.iter().any(|shell| !shell.closed) {
        return Err(refuse("STEP /9 export requires a solid body or open shell"));
    }
    let mut w = Writer::new();
    let mut vertex_entities = Vec::new();
    for v in &model.vertices {
        let p = w.point(&v.point);
        vertex_entities.push(w.emit(format!("VERTEX_POINT('',#{p})")));
    }
    let mut surface_cache = BTreeMap::<String, usize>::new();
    let mut surfaces = Vec::new();
    for face in &model.faces {
        let key = format!("{:?}", face.surface);
        let id = if let Some(id) = surface_cache.get(&key) {
            *id
        } else {
            let id = if ap242_composition {
                emit_surface_v5(&mut w, &face.surface)
            } else {
                emit_surface(&mut w, &face.surface)
            };
            surface_cache.insert(key, id);
            id
        };
        surfaces.push(id);
    }
    let curves3: Vec<_> = model
        .edges
        .iter()
        .map(|e| {
            if ap242_composition {
                emit_curve_v5(&mut w, &e.curve)
            } else {
                emit_curve(&mut w, &e.curve)
            }
        })
        .collect();
    let pcurve_context = ap242_composition.then(|| {
        w.emit("(GEOMETRIC_REPRESENTATION_CONTEXT(2)REPRESENTATION_CONTEXT('','2D'))".into())
    });
    let mut pcurves_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut pcurve_surfaces_by_edge = vec![Vec::<usize>::new(); model.edges.len()];
    let mut coedge_pcurve = BTreeMap::new();
    for (li, loop_) in model.loops.iter().enumerate() {
        for (ci, c) in loop_.coedges.iter().enumerate() {
            let face = model
                .faces
                .iter()
                .position(|f| f.outer == li || f.holes.contains(&li))
                .ok_or_else(|| refuse("Loop has no owning face"))?;
            let curve2 = if ap242_composition {
                emit_curve_v5(&mut w, &c.pcurve)
            } else {
                emit_curve(&mut w, &c.pcurve)
            };
            let reference = if let Some(context) = pcurve_context {
                w.emit(format!(
                    "DEFINITIONAL_REPRESENTATION('',(#{curve2}),#{context})"
                ))
            } else {
                curve2
            };
            let pc = w.emit(format!("PCURVE('',#{},#{reference})", surfaces[face]));
            pcurves_by_edge[c.edge].push(pc);
            pcurve_surfaces_by_edge[c.edge].push(surfaces[face]);
            coedge_pcurve.insert((li, ci), pc);
        }
    }
    let mut edge_entities = Vec::new();
    for (i, e) in model.edges.iter().enumerate() {
        if e.degenerate && !allow_degenerate {
            return Err(refuse(
                "Pole/degenerate edges are typed-refused before STEP /6",
            ));
        }
        let usages = model
            .loops
            .iter()
            .flat_map(|loop_| &loop_.coedges)
            .filter(|coedge| coedge.edge == i)
            .filter_map(|coedge| {
                let face = model.faces.iter().position(|face| {
                    face.outer < model.loops.len()
                        && (model.loops[face.outer]
                            .coedges
                            .iter()
                            .any(|candidate| std::ptr::eq(candidate, coedge))
                            || face.holes.iter().any(|hole| {
                                model.loops[*hole]
                                    .coedges
                                    .iter()
                                    .any(|candidate| std::ptr::eq(candidate, coedge))
                            }))
                })?;
                Some((surfaces[face], face, coedge))
            })
            .collect::<Vec<_>>();
        let mut seam = false;
        for a in 0..usages.len() {
            for b in a + 1..usages.len() {
                if usages[a].0 != usages[b].0 {
                    continue;
                }
                let da = usages[a].2.pcurve.domain();
                let db = usages[b].2.pcurve.domain();
                let pa = usages[a].2.pcurve.evaluate((da[0] + da[1]) / 2.)?.point;
                let pb = usages[b].2.pcurve.evaluate((db[0] + db[1]) / 2.)?.point;
                let surface = &model.faces[usages[a].1].surface;
                let periods = [
                    surface.knots_u[surface.knots_u.len() - surface.degree_u - 1]
                        - surface.knots_u[surface.degree_u],
                    surface.knots_v[surface.knots_v.len() - surface.degree_v - 1]
                        - surface.knots_v[surface.degree_v],
                ];
                seam |= (pa[0] - pb[0]).abs() >= (periods[0] - 1e-8)
                    || (pa[1] - pb[1]).abs() >= (periods[1] - 1e-8);
            }
        }
        let sc = w.emit(format!(
            "{}('',#{},({}),.PCURVE_S1.)",
            if ap242_composition && seam {
                "SEAM_CURVE"
            } else {
                "SURFACE_CURVE"
            },
            curves3[i],
            refs_text(&pcurves_by_edge[i])
        ));
        edge_entities.push(w.emit(format!(
            "EDGE_CURVE('',#{},#{},#{sc},.T.)",
            vertex_entities[e.vertices[0]], vertex_entities[e.vertices[1]]
        )));
    }
    let mut loop_entities = Vec::new();
    for loop_ in &model.loops {
        let oriented: Vec<_> = loop_
            .coedges
            .iter()
            .map(|c| {
                w.emit(format!(
                    "ORIENTED_EDGE('',*,*,#{},.{}.)",
                    edge_entities[c.edge],
                    if c.reversed { "F" } else { "T" }
                ))
            })
            .collect();
        loop_entities.push(w.emit(format!("EDGE_LOOP('',({}))", refs_text(&oriented))));
    }
    let mut face_reversed = vec![None; model.faces.len()];
    for shell in &model.shells {
        for usage in &shell.faces {
            if face_reversed[usage.face]
                .replace(usage.reversed)
                .is_some_and(|prior| prior != usage.reversed)
            {
                return Err(refuse("A shared face has inconsistent shell-use senses"));
            }
        }
    }
    let mut face_entities = Vec::new();
    for (i, f) in model.faces.iter().enumerate() {
        let mut bounds = vec![w.emit(format!(
            "FACE_OUTER_BOUND('',#{},.T.)",
            loop_entities[f.outer]
        ))];
        bounds.extend(
            f.holes
                .iter()
                .map(|h| w.emit(format!("FACE_BOUND('',#{},.T.)", loop_entities[*h]))),
        );
        face_entities.push(w.emit(format!(
            "ADVANCED_FACE('',({}),#{},.{})",
            refs_text(&bounds),
            surfaces[i],
            if face_reversed[i].unwrap_or(false) {
                "F."
            } else {
                "T."
            }
        )));
    }
    let mut shell_entities = Vec::new();
    for s in &model.shells {
        shell_entities.push(w.emit(format!(
            "{}('',({}))",
            if s.closed {
                "CLOSED_SHELL"
            } else {
                "OPEN_SHELL"
            },
            refs_text(
                &s.faces
                    .iter()
                    .map(|u| face_entities[u.face])
                    .collect::<Vec<_>>()
            )
        )));
    }
    let mut body_entities = Vec::new();
    for b in &model.bodies {
        body_entities.push(if b.inner_shells.is_empty() {
            w.emit(format!(
                "MANIFOLD_SOLID_BREP('',#{})",
                shell_entities[b.outer_shell]
            ))
        } else {
            if ap242_composition {
                let voids = b
                    .inner_shells
                    .iter()
                    .map(|s| {
                        w.emit(format!(
                            "ORIENTED_CLOSED_SHELL('',*,#{},.F.)",
                            shell_entities[*s]
                        ))
                    })
                    .collect::<Vec<_>>();
                w.emit(format!(
                    "BREP_WITH_VOIDS('',#{},({}))",
                    shell_entities[b.outer_shell],
                    refs_text(&voids)
                ))
            } else {
                w.emit(format!(
                    "BREP_WITH_VOIDS('',#{},({}))",
                    shell_entities[b.outer_shell],
                    refs_text(
                        &b.inner_shells
                            .iter()
                            .map(|s| shell_entities[*s])
                            .collect::<Vec<_>>()
                    )
                ))
            }
        });
    }
    let surface_models = model
        .shells
        .iter()
        .enumerate()
        .filter(|(_, shell)| !shell.closed)
        .map(|(index, _)| {
            w.emit(format!(
                "SHELL_BASED_SURFACE_MODEL('',(#{s}))",
                s = shell_entities[index]
            ))
        })
        .collect::<Vec<_>>();
    let unit = w.emit("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))".into());
    if ap242_composition {
        let uncertainty = w.emit(format!(
            "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{unit},'distance_accuracy_value','')",
            floats(std::slice::from_ref(&model.tolerance_mm))
        ));
        let context = w.emit(format!(
            "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))REPRESENTATION_CONTEXT('','3D'))"
        ));
        let mut items = body_entities.clone();
        items.extend(surface_models.iter().copied());
        let representation_type = if body_entities.is_empty() {
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION"
        } else {
            "ADVANCED_BREP_SHAPE_REPRESENTATION"
        };
        let representation = w.emit(format!(
            "{representation_type}('',({}),#{context})",
            refs_text(&items)
        ));
        let application =
            w.emit("APPLICATION_CONTEXT('managed model based 3d engineering')".into());
        let product_context = w.emit(format!("PRODUCT_CONTEXT('',#{application},'mechanical')"));
        let product = w.emit(format!("PRODUCT('model','model','',(#{product_context}))"));
        let formation = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
        let definition_context = w.emit(format!(
            "PRODUCT_DEFINITION_CONTEXT('part definition',#{application},'design')"
        ));
        let definition = w.emit(format!(
            "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
        ));
        let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
        w.emit(format!(
            "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
        ));
    }
    let header = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('OpenSCAD Viewer retained AP242 B-rep'),'2;1');\nFILE_NAME('model.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n";
    let mut text = String::from(header);
    for (id, row) in &w.rows {
        text.push_str(&format!("#{id}={row};\n"));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    enforce_output_limit(&text)?;
    let mut parsed = parse(&text)?;
    let groups: [(TopoKind, &[TopoId], &[usize]); 6] = [
        (TopoKind::Vertex, &model.1.vertices, &vertex_entities),
        (TopoKind::Edge, &model.1.edges, &edge_entities),
        (TopoKind::Loop, &model.1.loops, &loop_entities),
        (TopoKind::Face, &model.1.faces, &face_entities),
        (TopoKind::Shell, &model.1.shells, &shell_entities),
        (TopoKind::Body, &model.1.bodies, &body_entities),
    ];
    for (kind, ids, entities_for_kind) in groups {
        for (&topo, &entity_id) in ids.iter().zip(entities_for_kind) {
            let digest = entity_digest(&parsed, entity_id)?;
            let entity = parsed.get_mut(&entity_id).unwrap();
            if let Value::Call(_, args) = &mut entity.value {
                args[0] =
                    Value::String(format!("OSCAD_TOPO/3|{}|{}|{digest}", kind.as_str(), topo));
            }
        }
    }
    text = String::from(header);
    for (id, entity) in &parsed {
        text.push_str(&format!("#{id}={};\n", render_entity_value(&entity.value)));
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    enforce_output_limit(&text)?;
    let count = model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len();
    Ok((
        text,
        FeatureCertificate {
            capability,
            complete: true,
            notes: vec![
                "direct_model_export",
                "no_constructor_recognition",
                "sha256_canonical_graph_identity",
            ],
        },
        StepV3Report {
            identity: crate::step_interchange::StepIdentityReport {
                preserved: true,
                source: "internal-metadata",
                preserved_count: count,
                created_count: 0,
                lost_count: 0,
            },
            ignored_entities: vec![],
            metadata_loss: vec![],
            instance_count: parsed.len(),
            reachable_count: parsed.len(),
            definition_identities: vec![],
            occurrence_identities: vec![],
            product_hierarchy: vec![],
            external_references: vec![],
        },
    ))
}

pub fn export_step_v3(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(model, false, false, STEP_INTERCHANGE_V3_CAPABILITY)
}
pub fn export_step_v4(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let (text, mut certificate, report) = export_step_v3(model)?;
    certificate.capability = STEP_INTERCHANGE_V4_CAPABILITY;
    certificate.notes.extend([
        "analytic_periodic_carriers_import",
        "exact_endpoint_point_selectors",
    ]);
    Ok((text, certificate, report))
}
pub fn export_step_v5(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(model, true, false, STEP_INTERCHANGE_V5_CAPABILITY)
}
pub fn export_step_v6(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(
        &periodicized_step_v6(model)?,
        true,
        true,
        STEP_INTERCHANGE_V6_CAPABILITY,
    )
}
pub fn export_step_v7(model: &Model) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct(
        &periodicized_step_v6(model)?,
        true,
        true,
        STEP_INTERCHANGE_V7_CAPABILITY,
    )
}
pub fn export_step_v8(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    let periodic = periodicized_step_v6(model)?;
    let certificate = certify_step_v8_topology(&periodic)?;
    let (text, _, report) =
        export_step_direct(&periodic, true, true, STEP_INTERCHANGE_V8_CAPABILITY)?;
    Ok((text, certificate, report))
}
pub fn export_step_v9(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    // Compatibility API follows STEP /9 source-preserving chart semantics.
    let mut certificate = certify_step_v8_topology(model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
        "authored_nurbs_charts_preserved",
    ]);
    let (text, _, report) =
        export_step_direct(model, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    Ok((text, certificate, report))
}

#[path="step_v3_export/algorithm_merge_periodic_patch_grid.rs"]
mod algorithm_merge_periodic_patch_grid;
use algorithm_merge_periodic_patch_grid::merge_periodic_patch_grid;
#[path="step_v3_export/algorithm_periodicized_sphere.rs"]
mod algorithm_periodicized_sphere;
use algorithm_periodicized_sphere::periodicized_sphere;
#[path="step_v3_export/algorithm_periodic_patch_grid_compatible.rs"]
mod algorithm_periodic_patch_grid_compatible;
pub(crate) use algorithm_periodic_patch_grid_compatible::periodic_patch_grid_compatible;
#[path="step_v3_export/algorithm_periodicized_step_v6.rs"]
mod algorithm_periodicized_step_v6;
pub(crate) use algorithm_periodicized_step_v6::periodicized_step_v6;
#[path="step_v3_export/algorithm_certify_periodic_surface.rs"]
mod algorithm_certify_periodic_surface;
pub(crate) use algorithm_certify_periodic_surface::certify_periodic_surface;
#[path="step_v3_export/algorithm_certify_step_v8_topology.rs"]
mod algorithm_certify_step_v8_topology;
pub(crate) use algorithm_certify_step_v8_topology::certify_step_v8_topology;
