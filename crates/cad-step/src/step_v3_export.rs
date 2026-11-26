//! STEP `/3`–`/9` writer, periodic carrier reconstruction, and `/8` regularity certification.

use brep_core::analytic_features::FeatureCertificate;
use crate::step_interchange_v3::{
    MAX_OUTPUT_BYTES, STEP_INTERCHANGE_V3_CAPABILITY, STEP_INTERCHANGE_V4_CAPABILITY,
    STEP_INTERCHANGE_V5_CAPABILITY, STEP_INTERCHANGE_V6_CAPABILITY,
    STEP_INTERCHANGE_V7_CAPABILITY, STEP_INTERCHANGE_V8_CAPABILITY,
    STEP_INTERCHANGE_V9_CAPABILITY, StepRegularityEvidence, StepSemanticMetadata, StepV3Report,
    StepV8Certificate, entity_digest, refuse,
};
use crate::step_part21::{Value, parse, render_entity_value};
use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Model, TopoId, TopoKind};
use nurbs_core::{Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const STEP_V6_AUTHORED_CHART_MARKER: &str = "authored-chart-v1";

mod encoding;
pub(crate) use encoding::{
    Writer, canonical_face_senses, emit_curve_source, emit_curve_v5, emit_surface_source,
    enforce_output_limit, refs_text,
};
use encoding::{
    constant_boundary, emit_curve, emit_surface, emit_surface_v5, floats, quadratic_patch_knots,
};

/// Display material bound to an assembly occurrence. RGB channels are in 0..=1.
pub struct StepOccurrenceColour<'a> {
    pub name: &'a str,
    pub rgb: [f64; 3],
}

struct AssemblyExport<'a> {
    colours: Option<&'a [Option<StepOccurrenceColour<'a>>]>,
    groups: Option<(&'a [&'a str], &'a [Option<usize>])>,
    placements: &'a [(usize, [[f64; 4]; 4])],
    definition_tolerances: &'a [f64],
    names: Option<(&'a str, &'a [&'a str])>,
}

fn export_step_direct_impl(
    model: &Model,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
    semantic: Option<&StepSemanticMetadata>,
    assembly: Option<AssemblyExport<'_>>,
    share_points: bool,
) -> Result<(String, FeatureCertificate, StepV3Report)> {
    if let Some(semantic) = semantic {
        semantic.presentation.validate_for_model(model)?;
        semantic.xde.validate_for_model(model)?;
    }
    // AP242 reverses each cavity in ORIENTED_CLOSED_SHELL. Emit the
    // underlying shell in its own sense before canonicalizing its charts.
    let shell_geometry = (ap242_composition && model.bodies.iter().any(|b| !b.inner_shells.is_empty())).then(|| {
        let mut geometry = model.clone();
        for shell in model.bodies.iter().flat_map(|body| &body.inner_shells) {
            for face in &mut geometry.0.shells[*shell].faces {
                face.reversed = !face.reversed;
            }
        }
        geometry
    });
    let model = shell_geometry.as_ref().unwrap_or(model);
    // /6 exchanges the authored chart directly, including its original face
    // senses. Other routes mirror reversed faces into canonical forward charts.
    let canonical = (capability != STEP_INTERCHANGE_V6_CAPABILITY)
        .then(|| canonical_face_senses(model))
        .transpose()?;
    let model = canonical.as_ref().unwrap_or(model);
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
    if share_points || assembly.is_some() {
        w.enable_point_sharing();
    }
    let mut vertex_entities = Vec::new();
    for v in &model.vertices {
        let p = w.point(&v.point);
        vertex_entities.push(w.emit(format!("VERTEX_POINT('',#{p})")));
    }
    // At a shared pole several UV intervals map to one spatial vertex.
    // Give each incident face its own carrier for unambiguous PCURVE selection.
    let mut pole_uses = vec![0usize; model.edges.len()];
    for coedge in model.loops.iter().flat_map(|wire| &wire.coedges) {
        if model.edges[coedge.edge].degenerate {
            pole_uses[coedge.edge] += 1;
        }
    }
    let mut surface_cache = BTreeMap::<String, usize>::new();
    let mut surfaces = Vec::new();
    for face in &model.faces {
        if ap242_composition && std::iter::once(&face.outer).chain(&face.holes)
            .flat_map(|&wire| &model.loops[wire].coedges)
            .any(|coedge| pole_uses[coedge.edge] > 2)
        {
            surfaces.push(emit_surface_v5(&mut w, &face.surface));
            continue;
        }
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
    let mut uv_curves = BTreeMap::new();
    for (li, loop_) in model.loops.iter().enumerate() {
        for (ci, c) in loop_.coedges.iter().enumerate() {
            let face = model
                .faces
                .iter()
                .position(|f| f.outer == li || f.holes.contains(&li))
                .ok_or_else(|| refuse("Loop has no owning face"))?;
            // Share the immutable UV spline while retaining each use's carrier and orientation.
            let curve2 = if capability == STEP_INTERCHANGE_V9_CAPABILITY {
                *uv_curves.entry(format!("{:?}", c.pcurve)).or_insert_with(|| emit_curve_v5(&mut w, &c.pcurve))
            } else if ap242_composition {
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
        let mut contexts = BTreeMap::new();
        let mut context_for = |writer: &mut Writer, tolerance: f64| {
            *contexts.entry(tolerance.to_bits()).or_insert_with(|| {
                let uncertainty = writer.emit(format!(
                    "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{unit},'distance_accuracy_value','')",
                    floats(&[tolerance])
                ));
                writer.emit(format!(
                    "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))REPRESENTATION_CONTEXT('','3D'))"
                ))
            })
        };
        let context = context_for(&mut w, model.tolerance_mm);
        let definition_context_id = assembly.as_ref().map_or(context, |assembly| {
            context_for(&mut w, assembly.definition_tolerances[0])
        });
        let mut items = body_entities.clone();
        if assembly.is_some() {
            items.truncate(1);
        } else {
            items.extend(surface_models.iter().copied());
        }
        let representation_type = if body_entities.is_empty() {
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION"
        } else {
            "ADVANCED_BREP_SHAPE_REPRESENTATION"
        };
        let representation = w.emit(format!(
            "{representation_type}('',({}),#{definition_context_id})",
            refs_text(&items)
        ));
        let application =
            w.emit("APPLICATION_CONTEXT('managed model based 3d engineering')".into());
        let product_context = w.emit(format!("PRODUCT_CONTEXT('',#{application},'mechanical')"));
        let authored = semantic.and_then(|semantic| {
            (semantic.xde.products.len() == 1)
                .then(|| semantic.xde.products.values().next().unwrap())
        });
        let quote = |value: &str| format!(
            "'{}'",
            crate::step_interchange_v3::step_xde::encode_step_string(value).replace('\'', "''")
        );
        let product = w.emit(format!(
            "PRODUCT({},{},{},(#{product_context}))",
            quote(authored.map_or("model", |product| product.id.as_str())),
            quote(assembly.as_ref().and_then(|a| a.names.map(|(_, names)| names[0])).unwrap_or_else(|| authored.map_or("model", |product| product.name.as_str()))),
            quote(authored.map_or("", |product| product.description.as_str())),
        ));
        let formation = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
        let definition_context = w.emit(format!(
            "PRODUCT_DEFINITION_CONTEXT('part definition',#{application},'design')"
        ));
        let definition = w.emit(format!(
            "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
        ));
        let shape_name = authored.and_then(|product| product.shape_name.as_deref()).unwrap_or("");
        let shape = w.emit(format!(
            "PRODUCT_DEFINITION_SHAPE({},'',#{definition})",
            quote(shape_name)
        ));
        w.emit(format!(
            "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
        ));
        if let Some(assembly) = assembly {
            let assembly_product = w.emit(format!(
                "PRODUCT('planet-set',{},'',(#{product_context}))", quote(assembly.names.map_or("Planet gear set", |(name,_)| name))
            ));
            let assembly_formation = w.emit(format!(
                "PRODUCT_DEFINITION_FORMATION('1','',#{assembly_product})"
            ));
            let assembly_definition = w.emit(format!(
                "PRODUCT_DEFINITION('design','',#{assembly_formation},#{definition_context})"
            ));
            let assembly_shape = w.emit(format!(
                "PRODUCT_DEFINITION_SHAPE('','',#{assembly_definition})"
            ));
            let origin = w.point(&[0., 0., 0.]);
            let frame = w.emit(format!("AXIS2_PLACEMENT_3D('',#{origin},$,$)"));
            let assembly_rep = w.emit(format!("SHAPE_REPRESENTATION('',(#{frame}),#{context})"));
            w.emit(format!("SHAPE_DEFINITION_REPRESENTATION(#{assembly_shape},#{assembly_rep})"));
            let mut definitions = vec![(representation, definition)];
            for (index, &body) in body_entities.iter().enumerate().skip(1) {
                let product = w.emit(format!(
                    "PRODUCT('gear-{index}',{},'',(#{product_context}))",
                    assembly.names.map_or_else(|| quote(&format!("Gear {index}")), |(_,names)| quote(names[index]))
                ));
                let formation = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
                let definition = w.emit(format!(
                    "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
                ));
                let context = context_for(&mut w, assembly.definition_tolerances[index]);
                let representation = w.emit(format!(
                    "ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context})"
                ));
                let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
                w.emit(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"));
                definitions.push((representation, definition));
            }
            let mut group_nodes = Vec::new();
            if let Some((groups, _)) = assembly.groups {
                for (index, name) in groups.iter().enumerate() {
                    let product = w.emit(format!("PRODUCT('group-{index}',{},'',(#{product_context}))", quote(name)));
                    let formation = w.emit(format!("PRODUCT_DEFINITION_FORMATION('1','',#{product})"));
                    let definition = w.emit(format!("PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"));
                    let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
                    let rep = w.emit(format!("SHAPE_REPRESENTATION('',(#{frame}),#{context})"));
                    w.emit(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{rep})"));
                    let identity = w.emit(format!("ITEM_DEFINED_TRANSFORMATION('','',#{frame},#{frame})"));
                    let relation = w.emit(format!("(REPRESENTATION_RELATIONSHIP('','',#{rep},#{assembly_rep})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{identity})SHAPE_REPRESENTATION_RELATIONSHIP())"));
                    let usage = w.emit(format!("NEXT_ASSEMBLY_USAGE_OCCURRENCE('group-{index}',{},'',#{assembly_definition},#{definition},$)", quote(name)));
                    let usage_shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{usage})"));
                    w.emit(format!("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{relation},#{usage_shape})"));
                    group_nodes.push((definition, rep));
                }
            }
            let mut directions = BTreeMap::new();
            for (index, (body, matrix)) in assembly.placements.iter().enumerate() {
                let (representation, definition) = definitions[*body];
                let (assembly_definition, assembly_rep) = assembly.groups
                    .and_then(|(_, parents)| parents[index])
                    .map_or((assembly_definition, assembly_rep), |group| group_nodes[group]);
                let rigid = crate::step_interchange_v3::is_proper_rigid_placement(*matrix);
                let scales = if rigid {
                    [1.; 3]
                } else {
                    crate::step_interchange_v3::orthogonal_placement_scales(*matrix)
                        .ok_or_else(|| refuse("STEP assembly export requires nonsingular orthogonal placements"))?
                };
                let origin = w.point(&[matrix[0][3], matrix[1][3], matrix[2][3]]);
                let mut direction_id = |w: &mut Writer, column: usize| {
                    let direction = [0, 1, 2].map(|row| matrix[row][column] / scales[column]);
                    *directions.entry(direction.map(f64::to_bits)).or_insert_with(|| {
                        w.emit(format!("DIRECTION('',({}))", floats(&direction)))
                    })
                };
                let x = direction_id(&mut w, 0);
                let z = direction_id(&mut w, 2);
                if assembly.colours.is_some() && !rigid {
                    return Err(refuse("Styled STEP assemblies require rigid placements"));
                }
                let placement_axis = rigid.then(|| w.emit(format!("AXIS2_PLACEMENT_3D('',#{origin},#{z},#{x})")));
                let operator = if let Some(axis) = placement_axis {
                    w.emit(format!("ITEM_DEFINED_TRANSFORMATION('','',#{frame},#{axis})"))
                } else {
                    let y = direction_id(&mut w, 1);
                    let [sx, sy, sz] = scales;
                    let (sx, sy, sz) = (floats(&[sx]), floats(&[sy]), floats(&[sz]));
                    w.emit(format!(
                        "(CARTESIAN_TRANSFORMATION_OPERATOR('',#{x},#{y},#{origin},{sx})CARTESIAN_TRANSFORMATION_OPERATOR_3D(#{z})CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM({sy},{sz})GEOMETRIC_REPRESENTATION_ITEM()REPRESENTATION_ITEM(''))"
                    ))
                };
                let relationship = w.emit(format!(
                    "(REPRESENTATION_RELATIONSHIP('','',#{representation},#{assembly_rep})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{operator})SHAPE_REPRESENTATION_RELATIONSHIP())"
                ));
                let occurrence = w.emit(format!(
                    "NEXT_ASSEMBLY_USAGE_OCCURRENCE('gear-occurrence-{index}','Gear occurrence {index}','',#{assembly_definition},#{definition},$)"
                ));
                let shape = w.emit(format!("PRODUCT_DEFINITION_SHAPE('','',#{occurrence})"));
                w.emit(format!("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{relationship},#{shape})"));
                if let (Some(axis), Some(style)) = (placement_axis, assembly.colours.and_then(|styles| styles[index].as_ref())) {
                    let placement_rep = w.emit(format!("SHAPE_REPRESENTATION('',(#{axis}),#{context})"));
                    w.emit(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{placement_rep})"));
                    let rgb = w.emit(format!("COLOUR_RGB({},{})", quote(style.name), floats(&style.rgb)));
                    let colour = w.emit(format!("FILL_AREA_STYLE_COLOUR('',#{rgb})"));
                    let fill = w.emit(format!("FILL_AREA_STYLE('',(#{colour}))"));
                    let surface = w.emit(format!("SURFACE_STYLE_FILL_AREA(#{fill})"));
                    let side = w.emit(format!("SURFACE_SIDE_STYLE('',(#{surface}))"));
                    let usage = w.emit(format!("SURFACE_STYLE_USAGE(.BOTH.,#{side})"));
                    let assignment = w.emit(format!("PRESENTATION_STYLE_BY_CONTEXT((#{usage}),#{placement_rep})"));
                    let solid = body_entities[*body];
                    let item = w.emit(format!("STYLED_ITEM('',(#{assignment}),#{solid})"));
                    w.emit(format!("MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#{item}),#{context})"));
                }
            }
        }
        if let Some(semantic) = semantic {
            crate::step_interchange_v3::step_presentation::emit(
                &mut w,
                &semantic.presentation,
                &face_entities,
                &body_entities,
                representation,
            )?;
            crate::step_interchange_v3::step_xde::emit(
                &mut w,
                &semantic.xde,
                definition,
                context,
            )?;
        }
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
                let authored_chart = if capability == STEP_INTERCHANGE_V6_CAPABILITY
                    && kind == TopoKind::Face
                {
                    format!("|{STEP_V6_AUTHORED_CHART_MARKER}")
                } else {
                    String::new()
                };
                args[0] = Value::String(format!(
                    "OSCAD_TOPO/3|{}|{}|{digest}{authored_chart}",
                    kind.as_str(),
                    topo
                ));
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

pub(crate) fn export_step_direct(
    model: &Model,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(String, FeatureCertificate, StepV3Report)> {
    export_step_direct_impl(model, ap242_composition, allow_degenerate, capability, None, None, false)
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
    export_v9_points(model, false)
}

/// AP242 export sharing Cartesian point entities with bit-identical coordinates.
/// Topological vertices, parameter-space dimensions and authored charts remain distinct.
pub fn export_step_v9_shared_points(model: &Model) -> Result<(String, StepV8Certificate, StepV3Report)> {
    export_v9_points(model, true)
}

fn export_v9_points(model: &Model, share_points: bool) -> Result<(String, StepV8Certificate, StepV3Report)> {
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
        export_step_direct_impl(model, true, true, STEP_INTERCHANGE_V9_CAPABILITY, None, None, share_points)?;
    Ok((text, certificate, report))
}

mod assemblies;
pub use assemblies::{
    export_grouped_step_assembly, export_named_step_assembly, export_named_styled_step_assembly,
    export_planet_gear_occurrences_step, export_planetary_gear_set_step, export_step_shared_assembly,
    export_styled_step_assembly,
};

pub fn export_step_v9_with_metadata(
    model: &Model,
    semantic: &StepSemanticMetadata,
) -> Result<(String, StepV8Certificate, StepV3Report)> {
    export_v9_metadata_points(model, semantic, false)
}

/// Write typed product and face attributes together with shared Cartesian points.
pub fn export_step_v9_with_metadata_shared_points(
    model: &Model, semantic: &StepSemanticMetadata,
) -> Result<(String, StepV8Certificate, StepV3Report)> {
    export_v9_metadata_points(model, semantic, true)
}

fn export_v9_metadata_points(
    model: &Model, semantic: &StepSemanticMetadata, share_points: bool,
) -> Result<(String, StepV8Certificate, StepV3Report)> {
    let mut certificate = certify_step_v8_topology(model)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "ap214_face_body_product_presentation",
        "typed_xde_product_attributes",
        "validation_properties_preserved",
    ]);
    let (text, _, report) = export_step_direct_impl(
        model,
        true,
        true,
        STEP_INTERCHANGE_V9_CAPABILITY,
        Some(semantic),
        None,
        share_points,
    )?;
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
