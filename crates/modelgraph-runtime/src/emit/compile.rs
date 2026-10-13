use super::*;

fn collect_assertion_targets<'a>(value: &'a Value, targets: &mut HashSet<&'a str>) {
    match value {
        Value::Object(fields) => {
            if fields.get("op").and_then(Value::as_str) == Some("geometry_effects")
                && let Some(target) = fields.get("input").and_then(Value::as_str)
            {
                targets.insert(target);
            }
            if let Some(checks) = fields.get("geometry_assertions").and_then(Value::as_array) {
                targets.extend(checks.iter().filter_map(|check| check["target"].as_str()));
            }
            for child in fields.values() {
                collect_assertion_targets(child, targets);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_assertion_targets(child, targets);
            }
        }
        _ => (),
    }
}
impl<'a> Emitter<'a> {
    fn assertion_source(
        &mut self,
        target: &str,
        nodes: &Rc<Nodes<'a>>,
        scope: &Scope<'a>,
        effects_only: bool,
    ) -> Result<String> {
        if !nodes.contains_key(target) {
            return Err(Error::new(
                "unknown_node",
                "/geometry_assertions",
                format!("Unknown assertion target {target}."),
            ));
        }
        let snapshot_key = (
            *nodes.get(target).unwrap() as *const Value as usize,
            Rc::as_ptr(scope) as usize,
        );
        let previously_emitted = self.assertion_snapshots.contains_key(&snapshot_key);
        if !effects_only
            && self
                .assertion_snapshots
                .get(&snapshot_key)
                .is_some_and(|snapshot| snapshot.geometry_type == GeometryType::Profile)
        {
            return Err(Error::new(
                "geometry_type_mismatch",
                "/geometry_assertions",
                "Geometry assertions require a solid; extrude or revolve profiles first.",
            ));
        }
        if let Some(source) = self
            .assertion_snapshots
            .get(&snapshot_key)
            .filter(|snapshot| effects_only || snapshot.geometry_type == GeometryType::Solid)
            .and_then(|snapshot| snapshot.source.as_ref())
        {
            return Ok(source.clone());
        }
        let lines = std::mem::replace(&mut self.lines, vec![format!("$fn = {};", self.segments)]);
        let source_map = std::mem::take(&mut self.source_map);
        let lengths = (
            self.sketch_solutions.len(),
            self.assembly_components.len(),
            self.mechanical_reports.len(),
            self.mechanical_parts.len(),
        );
        // Hidden targets execute their embedded checks on first use. A previously
        // evaluated target is only rendered again when the bounded cache was full.
        self.eval.suppress_checks = previously_emitted;
        let result = self.emit(
            target,
            nodes,
            scope,
            "/assertion_target",
            1,
            if effects_only {
                GeometryType::Any
            } else {
                GeometryType::Solid
            },
            None,
            true,
        );
        self.eval.suppress_checks = false;
        let source = self.lines.join("\n");
        self.lines = lines;
        self.source_map = source_map;
        self.sketch_solutions.truncate(lengths.0);
        self.assembly_components.truncate(lengths.1);
        self.mechanical_reports.truncate(lengths.2);
        self.mechanical_parts.truncate(lengths.3);
        result?;
        Ok(source)
    }
}

pub fn compile(document: &Value) -> Result<Value> {
    let eval = Evaluator::new(document)?;
    let mut assertion_targets = HashSet::new();
    collect_assertion_targets(document, &mut assertion_targets);
    let main = index_nodes(
        &document["nodes"],
        str_at(document, "root"),
        "/nodes",
        &assertion_targets,
    )?;
    let mut bodies = HashMap::new();
    for fn_value in document["functions"].as_array().into_iter().flatten() {
        if str_at(fn_value, "kind") == "geometry" {
            let id = str_at(fn_value, "id");
            bodies.insert(
                id,
                index_nodes(
                    &fn_value["nodes"],
                    str_at(fn_value, "root"),
                    &format!("/functions/{id}/nodes"),
                    &assertion_targets,
                )?,
            );
        }
    }
    let segments = document["segments"].as_f64().unwrap() as u64;
    let mut emitter = Emitter {
        assertion_targets,
        assertion_snapshots: HashMap::new(),
        assertion_snapshot_bytes: 0,
        eval,
        bodies,
        lines: vec![
            "// Generated from modelgraph/1; execution target: legacy/current + Manifold".into(),
            format!("$fn = {segments};"),
        ],
        source_map: vec![],
        sketch_solutions: vec![],
        assembly_components: vec![],
        mechanical_reports: vec![],
        mechanical_parts: vec![],
        expanded: 0,
        profile_work: 0,
        mechanical_characters: 0,
        segments,
    };
    let checks = emitter.eval.validate_checks()?;
    let global_scope = emitter.eval.global_scope.clone();
    emitter.emit(
        str_at(document, "root"),
        &main,
        &global_scope,
        "",
        1,
        GeometryType::Solid,
        None,
        true,
    )?;
    let mut geometry_assertions = checks.geometry_assertions.as_array().unwrap().clone();
    let mut constraint_report = checks.constraint_report.as_array().unwrap().clone();
    if geometry_assertions.len()
        + emitter
            .eval
            .pending_geometry_checks
            .iter()
            .filter(|pending| pending.check["check"] != "__effects")
            .count()
        > 256
    {
        return Err(Error::new(
            "assertion_limit",
            "/geometry_assertions",
            "Maximum 256 expanded geometry checks.",
        ));
    }
    let mut source_characters = 0;
    let mut source_cache: HashMap<(String, usize, usize, bool), String> = HashMap::new();
    for check in &mut geometry_assertions {
        let target = str_at(check, "target").to_owned();
        if !main.contains_key(target.as_str()) {
            return Err(Error::new(
                "unknown_node",
                "/geometry_assertions",
                format!("Unknown assertion target {target}."),
            ));
        }
        if target != str_at(document, "root") {
            let cache_key = (
                target.clone(),
                Rc::as_ptr(&global_scope) as usize,
                Rc::as_ptr(&main) as usize,
                false,
            );
            let source = if let Some(source) = source_cache.get(&cache_key) {
                source.clone()
            } else {
                let source = emitter.assertion_source(&target, &main, &global_scope, false)?;
                source_cache.insert(cache_key, source.clone());
                source
            };
            source_characters += source.len();
            if source_characters > 1_000_000 {
                return Err(Error::new(
                    "assertion_limit",
                    "/geometry_assertions",
                    "Assertion sources exceed 1000000 bytes.",
                ));
            }
            check["source"] = json!(source);
        }
    }
    let mut pending_index = 0;
    while pending_index < emitter.eval.pending_geometry_checks.len() {
        let queued = &emitter.eval.pending_geometry_checks[pending_index];
        let mut pending = crate::eval::PendingGeometryCheck {
            check: queued.check.clone(),
            scope: queued.scope.clone(),
        };
        pending_index += 1;
        let effects_only = pending.check["check"] == "__effects";
        if !effects_only && geometry_assertions.len() >= 256 {
            return Err(Error::new(
                "assertion_limit",
                "/geometry_assertions",
                "Maximum 256 expanded geometry checks.",
            ));
        }
        let target = str_at(&pending.check, "target").to_owned();
        let candidates: Vec<_> = std::iter::once(&main)
            .chain(emitter.bodies.values())
            .filter(|nodes| nodes.contains_key(target.as_str()))
            .cloned()
            .collect();
        let nodes = match candidates.as_slice() {
            [nodes] => nodes.clone(),
            [] => {
                return Err(Error::new(
                    "unknown_node",
                    "/geometry_assertions",
                    format!("Unknown assertion target {target}."),
                ));
            }
            _ => {
                return Err(Error::new(
                    "ambiguous_assertion_target",
                    "/geometry_assertions",
                    format!("Assertion target {target} occurs in multiple geometry graphs."),
                ));
            }
        };
        let cache_key = (
            target.clone(),
            Rc::as_ptr(&pending.scope) as usize,
            Rc::as_ptr(&nodes) as usize,
            effects_only,
        );
        let source = if let Some(source) = source_cache.get(&cache_key) {
            source.clone()
        } else {
            let source = emitter.assertion_source(&target, &nodes, &pending.scope, effects_only)?;
            source_cache.insert(cache_key, source.clone());
            source
        };
        if effects_only {
            continue;
        }
        source_characters += source.len();
        if source_characters > 1_000_000 {
            return Err(Error::new(
                "assertion_limit",
                "/geometry_assertions",
                "Assertion sources exceed 1000000 bytes.",
            ));
        }
        pending.check["source"] = json!(source);
        geometry_assertions.push(pending.check);
    }
    constraint_report.append(&mut emitter.eval.constraint_report);
    Ok(
        json!({"geometry_assertions":geometry_assertions,"constraint_report":constraint_report,"sketch_solutions":emitter.sketch_solutions,"assembly_components":emitter.assembly_components,"mechanical_reports":emitter.mechanical_reports,"mechanical_parts":emitter.mechanical_parts,"source":emitter.lines.join("\n"),"source_map":emitter.source_map,"execution_target":"legacy/current+own-rust-cad"}),
    )
}
