use super::*;
pub(crate) fn certify_step_v8_topology(model: &Model) -> Result<StepV8Certificate> {
    model.validate()?;
    let mut seen = BTreeSet::new();
    let mut regularity = Vec::new();
    for face in &model.faces {
        if !seen.insert(format!("{:?}", face.surface)) {
            continue;
        }
        if let Some(evidence) = certify_periodic_surface(&face.surface)? {
            regularity.push(evidence)
        }
    }
    for evidence in &regularity {
        for boundary in &evidence.collapsed_boundaries {
            let boundary_value = if *boundary == "v-min" {
                evidence.parameter_v[0]
            } else {
                evidence.parameter_v[1]
            };
            let mut intervals = Vec::<[f64; 2]>::new();
            for face in &model.faces {
                if !certify_periodic_surface(&face.surface)?
                    .is_some_and(|candidate| candidate.carrier == evidence.carrier)
                {
                    continue;
                }
                for loop_id in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for coedge in &model.loops[loop_id].coedges {
                        if !model.edges[coedge.edge].degenerate {
                            continue;
                        }
                        let domain = coedge.pcurve.domain();
                        let a = coedge.pcurve.evaluate(domain[0])?.point;
                        let b = coedge.pcurve.evaluate(domain[1])?.point;
                        if (a[1] - boundary_value).abs() <= 1e-12
                            && (b[1] - boundary_value).abs() <= 1e-12
                        {
                            intervals.push([a[0].min(b[0]), a[0].max(b[0])])
                        }
                    }
                }
            }
            intervals.sort_by(|left, right| left[0].total_cmp(&right[0]));
            let mut covered = evidence.parameter_u[0];
            for interval in intervals {
                if interval[0] > covered + 1e-12 {
                    break;
                }
                covered = covered.max(interval[1]);
            }
            if covered < evidence.parameter_u[1] - 1e-12 {
                return Err(refuse(
                    "STEP /8 pole topology does not cover the complete collapsed parameter interval",
                ));
            }
        }
    }
    Ok(StepV8Certificate {
        capability: STEP_INTERCHANGE_V8_CAPABILITY,
        complete: true,
        regularity,
        sense_layers: [
            "seam/pole/hole",
            "EDGE_CURVE.same_sense",
            "ORIENTED_EDGE.orientation",
            "FACE_BOUND.orientation",
            "ADVANCED_FACE.same_sense",
            "CLOSED_SHELL face use",
            "ORIENTED_CLOSED_SHELL.orientation",
        ],
        coupled_sense_cases: 128,
        notes: vec![
            "whole_domain_homogeneous_interval_certificate",
            "collapsed_boundaries_bound_to_pole_topology",
            "lifted_periods_bound_to_seam_topology",
            "exhaustive_coupled_sense_matrix",
        ],
    })
}

fn export_step_direct(
    model: &Model,
    ap242_composition: bool,
    allow_degenerate: bool,
    capability: &'static str,
) -> Result<(String, FeatureCertificate, StepV3Report)> {
    let canonical = brep_core::face_senses::canonical_face_senses(model)?;
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
        shell_entities.push(w.emit(
            format!("{}('',({}))",if s.closed{"CLOSED_SHELL"}else{"OPEN_SHELL"},
            refs_text(&s.faces.iter().map(|u| face_entities[u.face]).collect::<Vec<_>>())),
        ));
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
        let uncertainty=w.emit(format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-7),#{unit},'distance_accuracy_value','')"));
        let context=w.emit(format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))REPRESENTATION_CONTEXT('','3D'))"));
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
    let periodic = periodicized_step_v6(model)?;
    let mut certificate = certify_step_v8_topology(&periodic)?;
    certificate.capability = STEP_INTERCHANGE_V9_CAPABILITY;
    certificate.notes.extend([
        "certified_interior_point_selectors",
        "bounded_freeform_poles",
        "general_affine_occurrences",
        "retained_open_shell_products",
    ]);
    let (text, _, report) =
        export_step_direct(&periodic, true, true, STEP_INTERCHANGE_V9_CAPABILITY)?;
    Ok((text, certificate, report))
}
