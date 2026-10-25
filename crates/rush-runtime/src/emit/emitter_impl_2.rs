use super::*;

impl<'a> Emitter<'a> {
    // Traversal carries distinct lexical, geometric and assembly contexts.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit(
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
        let cx = NativeEmitCtx { item, key, op, nodes, scope, current: &current, depth, expected, parent, allow_assembly, source_start };
        match op {
            "planetary_spinner" | "gear" | "thread" | "planetary_gears" | "assembly" | "assert" | "evaluate" | "call" | "match" | "if" | "group" | "collect" | "map" | "affine" | "mirror" | "projection" | "section" | "offset" => self.emit_cases_1(cx),
            "advanced_extrude" | "cone" | "torus" | "linear_pattern" | "circular_pattern" | "loft" | "sketch" | "rectangle" | "box" | "circle" | "sphere" | "cylinder" | "polygon" | "extrude" | "revolve" | "translate" | "rotate" | "scale" | "union" | "intersection" | "difference" | "hull" => self.emit_cases_2(cx),
            _ => Err(fail("invalid_document", "Unsupported geometry operation.")),
        }
    }
}
