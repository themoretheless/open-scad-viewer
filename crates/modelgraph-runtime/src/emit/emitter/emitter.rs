use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn value(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
    ) -> Result<f64> {
        self.eval
            .field(expr, scope, &format!("{current}/{field}"), dimension)
    }
    pub(super) fn vector(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
    ) -> Result<Vec<f64>> {
        if !expr.is_array() {
            let path = format!("{current}/{field}");
            let value = self.eval.resolve(expr, scope, &path, 0)?;
            return sequence(&value, &path)?
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let path = format!("{path}/{index}");
                    crate::units::field(
                        numeric_value(value, &path)?,
                        dimension,
                        &path,
                        self.eval.strict,
                    )
                })
                .collect();
        }
        array(expr)
            .iter()
            .enumerate()
            .map(|(i, v)| self.value(v, scope, current, &format!("{field}/{i}"), dimension))
            .collect()
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn geometry_vector(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
        size: usize,
    ) -> Result<Vec<f64>> {
        let values = self.vector(expr, scope, current, field, dimension)?;
        if values.len() != size {
            return Err(Error::new(
                "type_error",
                format!("{current}/{field}"),
                format!(
                    "Expected vector with {size} components, received {}.",
                    values.len()
                ),
            ));
        }
        Ok(values)
    }
    pub(super) fn frame(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
    ) -> Result<Value> {
        Ok(
            json!({"origin":self.vector(&expr["origin"],scope,current,&format!("{field}/origin"),LENGTH)?,"rotation":self.vector(&expr["rotation"],scope,current,&format!("{field}/rotation"),ANGLE)?}),
        )
    }
    pub(super) fn child(
        &mut self,
        item: &'a Value,
        nodes: &Rc<Nodes<'a>>,
        scope: &Scope<'a>,
        current: &str,
        depth: usize,
        expected: GeometryType,
    ) -> Result<()> {
        self.emit(
            str_at(item, "input"),
            nodes,
            scope,
            current,
            depth + 1,
            expected,
            None,
            false,
        )
    }
    // Traversal carries distinct lexical, geometric and assembly contexts.
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn emit(
        &mut self,
        key: &str,
        nodes: &Rc<Nodes<'a>>,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
        expected: GeometryType,
        parent: Option<&Parent>,
        allow_assembly: bool,
    ) -> Result<()> {
        self.expanded += 1;
        if self.expanded > 4096 || depth > 32 {
            return Err(Error::new(
                "graph_limit",
                path,
                "Maximum depth 32 and expanded node uses 4096.",
            ));
        }
        let item = *nodes.get(key).unwrap();
        let source_start = self.lines.len();
        let current = format!("{path}/{key}");
        let op = str_at(item, "op");
        let fail = |code: &str, message: &str| Error::new(code, &current, message);
        let produced = if [
            "sketch",
            "rectangle",
            "circle",
            "polygon",
            "offset",
            "projection",
            "section",
        ]
        .contains(&op)
        {
            GeometryType::Profile
        } else if [
            "box",
            "sphere",
            "cylinder",
            "extrude",
            "revolve",
            "loft",
            "advanced_extrude",
            "cone",
            "torus",
            "gear",
            "thread",
            "planetary_gears",
            "planetary_spinner",
        ]
        .contains(&op)
        {
            GeometryType::Solid
        } else {
            expected
        };
        if expected != GeometryType::Any && produced != expected {
            return Err(fail(
                "geometry_type_mismatch",
                &format!(
                    "Expected {}, received {}. Extrude or revolve a profile before using it as a solid.",
                    expected.name(),
                    produced.name()
                ),
            ));
        }
        let expected = if expected == GeometryType::Any {
            produced
        } else {
            expected
        };
        self.source_map
            .push(json!({"node_id":key,"line":self.lines.len()+1,"instance_path":current}));
        let cx = Ctx {
            item,
            key,
            op,
            nodes,
            scope,
            current: &current,
            depth,
            expected,
            parent,
            allow_assembly,
        };
        match op {
            "planetary_spinner" => self.emit_planetary_spinner(&cx)?,
            "gear" | "thread" | "planetary_gears" => self.emit_mechanical(&cx)?,
            "assembly" => self.emit_assembly(&cx)?,
            "assert" => self.emit_assert(&cx)?,
            "evaluate" => self.emit_evaluate(&cx)?,
            "call" => self.emit_call(&cx)?,
            "match" => self.emit_match(&cx)?,
            "if" => self.emit_if(&cx)?,
            "group" => self.emit_group(&cx)?,
            "collect" => self.emit_collect(&cx)?,
            "map" => self.emit_map(&cx)?,
            "affine" => self.emit_affine(&cx)?,
            "mirror" => self.emit_mirror(&cx)?,
            "projection" | "section" => self.emit_projection(&cx)?,
            "offset" => self.emit_offset(&cx)?,
            "advanced_extrude" => self.emit_advanced_extrude(&cx)?,
            "cone" => self.emit_cone(&cx)?,
            "torus" => self.emit_torus(&cx)?,
            "linear_pattern" | "circular_pattern" => self.emit_pattern(&cx)?,
            "loft" => self.emit_loft(&cx)?,
            "sketch" => self.emit_sketch(&cx)?,
            "rectangle" | "box" => self.emit_rectangle_box(&cx)?,
            "circle" | "sphere" | "cylinder" => self.emit_round(&cx)?,
            "polygon" => self.emit_polygon(&cx)?,
            "extrude" | "revolve" => self.emit_extrude_revolve(&cx)?,
            "translate" | "rotate" | "scale" => self.emit_transform(&cx)?,
            "union" | "intersection" | "difference" | "hull" => self.emit_boolean(&cx)?,
            _ => return Err(fail("invalid_document", "Unsupported geometry operation.")),
        }
        if self.assertion_targets.contains(key) {
            let snapshot_key = (item as *const Value as usize, Rc::as_ptr(scope) as usize);
            if !self.assertion_snapshots.contains_key(&snapshot_key) {
                let source = format!(
                    "$fn = {};\n{}",
                    self.segments,
                    self.lines[source_start..].join("\n")
                );
                let source = if self.assertion_snapshot_bytes + source.len() <= 1_000_000 {
                    self.assertion_snapshot_bytes += source.len();
                    Some(source)
                } else {
                    None
                };
                self.assertion_snapshots.insert(
                    snapshot_key,
                    AssertionSnapshot {
                        geometry_type: expected,
                        _scope: scope.clone(),
                        source,
                    },
                );
            }
        }
        Ok(())
    }
}
