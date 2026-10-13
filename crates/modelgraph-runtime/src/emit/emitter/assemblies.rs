use super::*;

impl<'a> Emitter<'a> {
    pub(super) fn emit_assembly(&mut self, cx: &Ctx<'a, '_>) -> Result<()> {
        let Ctx {
            item,
            nodes,
            scope,
            current,
            depth,
            expected,
            parent,
            allow_assembly,
            ..
        } = *cx;
        let fail = |code: &str, message: &str| Error::new(code, current.as_str(), message);
        if !allow_assembly || expected != GeometryType::Solid {
            return Err(fail(
                "assembly_scope",
                "Assembly must be the root or a direct assembly component; boolean/transform wrappers are not allowed.",
            ));
        }
        let mut components = Vec::with_capacity(array(&item["components"]).len());
        for (i, c) in array(&item["components"]).iter().enumerate() {
            let prefix = format!("components/{i}");
            let mut anchors = Vec::with_capacity(array(&c["anchors"]).len());
            for (j, a) in array(&c["anchors"]).iter().enumerate() {
                let mut v = self.frame(a, scope, &current, &format!("{prefix}/anchors/{j}"))?;
                v["id"] = a["id"].clone();
                anchors.push(v);
            }
            let mut component = json!({"id":c["id"],"input":c["input"],"anchors":anchors});
            if let Some(p) = c.get("placement") {
                component["placement"] =
                    self.frame(p, scope, &current, &format!("{prefix}/placement"))?;
            }
            if let Some(m) = c.get("mate") {
                let mut mate = json!({"component":m["component"],"anchor":m["anchor"],"own_anchor":m["own_anchor"],"gap":self.value(&m["gap"],scope,&current,&format!("{prefix}/mate/gap"),LENGTH)?,"rotation":self.vector(&m["rotation"],scope,&current,&format!("{prefix}/mate/rotation"),ANGLE)?});
                if let Some(j) = m.get("joint") {
                    let dim = if str_at(j, "kind") == "revolute" {
                        ANGLE
                    } else {
                        LENGTH
                    };
                    let mut joint = json!({"kind":j["kind"]});
                    for k in ["position", "min", "max"] {
                        joint[k] = json!(self.value(
                            &j[k],
                            scope,
                            &current,
                            &format!("{prefix}/joint/{k}"),
                            dim
                        )?);
                    }
                    mate["joint"] = joint;
                }
                component["mate"] = mate;
            }
            components.push(component);
        }
        let placed = assembly::place(&components, &current)?;
        for mut component in placed {
            if self.assembly_components.len() >= 64 {
                return Err(fail(
                    "assembly_limit",
                    "Maximum 64 expanded assembly components.",
                ));
            }
            let m = assembly::read_matrix(&component["matrix"]);
            self.lines.push(format!(
                "multmatrix({}){{",
                json_string(&assembly::matrix_json(m))
            ));
            let world = parent.map_or(m, |p| assembly::multiply(p.matrix, m));
            if world.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
                return Err(fail(
                    "assembly_limit",
                    "World transform exceeds numeric limits.",
                ));
            }
            let component_path = format!("{current}/components/{}", str_at(&component, "id"));
            let input = str_at(&component, "input").to_owned();
            let is_assembly = nodes
                .get(input.as_str())
                .is_some_and(|n| str_at(n, "op") == "assembly");
            component["matrix"] = json!(world);
            for a in component["anchors"].as_array_mut().unwrap() {
                if let Some(p) = parent {
                    a["matrix"] = json!(assembly::multiply(
                        p.matrix,
                        assembly::read_matrix(&a["matrix"])
                    ));
                }
            }
            component["instance_path"] = json!(component_path);
            component["parent_path"] = parent.map_or(Value::Null, |p| json!(p.path));
            component["is_assembly"] = json!(is_assembly);
            let record_index = self.assembly_components.len();
            self.assembly_components.push(component);
            let start = self.lines.len();
            let child_parent = Parent {
                path: component_path.clone(),
                matrix: world,
            };
            self.emit(
                &input,
                nodes,
                scope,
                &component_path,
                depth + 1,
                GeometryType::Solid,
                Some(&child_parent),
                true,
            )?;
            if !is_assembly {
                self.assembly_components[record_index]["source"] = json!(format!(
                    "$fn={};\nmultmatrix({}){{\n{}\n}}",
                    self.segments,
                    json_string(&assembly::matrix_json(world)),
                    self.lines[start..].join("\n")
                ));
            }
            self.lines.push("}".into());
        }
        Ok(())
    }
}
