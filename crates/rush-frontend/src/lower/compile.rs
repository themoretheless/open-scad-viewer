use super::*;

pub(super) fn has_assertion(value: &J) -> bool {
    match value {
        J::Object(fields) => {
            ["assert_value", "geometry_effects"].contains(&s(value, "op"))
                || fields.values().any(has_assertion)
        }
        J::Array(items) => items.iter().any(has_assertion),
        _ => false,
    }
}
pub(super) fn gather(v: &V, out: &mut Vec<J>) {
    match v {
        V::Void(checks) => out.extend(checks.iter().cloned()),
        V::Array(a) => a.iter().for_each(|v| gather(v, out)),
        V::Record(r, _, _, _) => r.values().for_each(|v| gather(v, out)),
        V::Json(j) if ["typed", "typed_value", "checked", "memo"].contains(&s(j, "op")) => {
            out.push(j.clone())
        }
        V::Json(j) if j.get("sequence").is_some() => gather(&V::Json(j["sequence"].clone()), out),
        _ => {}
    }
}
pub(super) fn infer(expected: &J, actual: &J, g: &Set<String>, bs: &mut Types, explicit: bool) -> R<()> {
    let name = s(expected, "name");
    if g.contains(name) {
        if let Some(old) = bs.get(name) {
            if !explicit && typename(old) != typename(actual) {
                return Err(crate::error(format!(
                    "Conflicting inference for {name}: {} and {}",
                    typename(old),
                    typename(actual)
                )));
            }
        } else {
            bs.insert(name.into(), actual.clone());
        }
    } else if name == s(actual, "name") {
        for (e, a) in arr(expected, "args").iter().zip(arr(actual, "args")) {
            infer(e, a, g, bs, explicit)?
        }
    }
    Ok(())
}
pub(super) fn binding_names(pattern: &J) -> Vec<&str> {
    if s(pattern, "kind") == "name" {
        vec![s(pattern, "value")]
    } else {
        arr(pattern, "items")
            .iter()
            .map(|item| s(item, "value"))
            .collect()
    }
}
impl Compiler {
    pub(super) fn bind(&mut self, p: &J, v: V, e: &mut Env) -> R<Vec<J>> {
        if matches!(&v, V::Void(_)) {
            return Err(crate::error("Function has no return value"));
        }
        if s(p, "kind") == "name" {
            let name = s(p, "value");
            if e.contains_key(name) {
                return Err(crate::error(format!("Duplicate name {name}")));
            }
            e.insert(name.into(), v);
            return Ok(Vec::new());
        }
        if s(p, "kind") == "list_pattern" {
            let items = arr(p, "items");
            for item in items {
                if e.contains_key(s(item, "value")) {
                    return Err(crate::error(format!("Duplicate name {}", s(item, "value"))));
                }
            }
            if let V::Array(values) = v {
                if values.len() != items.len() {
                    return Err(crate::error(format!(
                        "List destructuring requires exactly {} elements",
                        items.len()
                    )));
                }
                for (item, value) in items.iter().zip(values) {
                    e.insert(s(item, "value").into(), value);
                }
                return Ok(Vec::new());
            }
            let input = json!({"op":"memo","id":self.check_id("m"),"value":list(v)?});
            let constraint = json!({"id":self.check_id("v"),"left":{"op":"length","input":&input},"relation":"eq","right":items.len(),"message":format!("List destructuring requires exactly {} elements",items.len())});
            let check = json!({"op":"memo","id":self.check_id("m"),"value":{"op":"assert_value","constraints":[constraint],"value":1}});
            for (index, item) in items.iter().enumerate() {
                e.insert(s(item,"value").into(),V::Json(json!({"op":"at","input":{"op":"checked","checks":[&check],"value":&input},"index":index})));
            }
            return Ok(vec![check]);
        }
        let V::Record(r, _, _, _) = v else {
            return Err(crate::error("Destructuring requires a record"));
        };
        for item in arr(p, "items") {
            let name = s(item, "value");
            if !r.contains_key(name) {
                return Err(crate::error(format!("Unknown result field {name}")));
            }
            if e.contains_key(name) {
                return Err(crate::error(format!("Duplicate name {name}")));
            }
        }
        for item in arr(p, "items") {
            let name = s(item, "value");
            e.insert(name.into(), r[name].clone());
        }
        Ok(Vec::new())
    }
}
pub fn compile(statements: Vec<Statement>) -> R<J> { compile_with_ids(statements, false) }
pub fn compile_with_ids(statements: Vec<Statement>, stable_ids: bool) -> R<J> {
    let mut c = Compiler {
        nodes: vec![],
        parameters: vec![],
        controls: vec![],
        constraints: vec![],
        checks: vec![],
        collections: Set::new(),
        structs: Map::new(),
        traits: Map::new(),
        trait_envs: Map::new(),
        implementations: Map::new(),

        active: Types::new(),
        serial: 0,
        stable_ids,
        owner: String::new(),
        owner_serial: 0,
        show_serial: 0,
        author_ids: Set::new(),
        check_serial: 0,
        expansions: 0,
    };
    let mut e = Env::new();
    let mut root = String::new();
    let mut segments = None;
    for statement in statements {
        let a = statement.node;
        c.owner.clear();
        c.owner_serial = 0;
        let result: R<()> = try {
            let name = s(&a, "name");
            match s(&a, "kind") {
                "bind" => {
                    if e.contains_key(name) {
                        do yeet crate::error(format!("Duplicate name {name}"));
                    }
                    c.owner = if let Some(id) = a.get("author_id").and_then(J::as_str) {
                        if !c.author_ids.insert(id.to_owned()) { do yeet crate::error(format!("Duplicate node ID {id}")); }
                        format!("ri_{id}")
                    } else { format!("rb_{name}") };
                    c.owner_serial = 0;
                    let v = c.eval(&a["value"], &e, 0)?;
                    if matches!(&v, V::Void(_)) {
                        do yeet crate::error("Function has no return value");
                    }
                    if c.needs_assertion(&v) {
                        c.statement_checks(&v);
                    }
                    if let V::Json(j) = &v
                        && j.get("geometry").is_some()
                    {
                        root = s(j, "geometry").into()
                    }
                    e.insert(name.into(), v);
                }
                "trait" => {
                    if c.traits.contains_key(name)
                        || c.structs.contains_key(name)
                        || [
                            "int", "f32", "f64", "str", "length", "angle", "Geometry", "Vec",
                        ]
                        .contains(&name)
                    {
                        do yeet crate::error(format!("Duplicate or reserved type {name}"));
                    }
                    c.validate_trait(&a)?;
                    c.traits.insert(name.into(), a.clone());
                    c.trait_envs.insert(name.into(), e.clone());
                }
                "impl" => c.register_impl(&a, &e)?,
                "struct" => {
                    if c.structs.contains_key(name)
                        || c.traits.contains_key(name)
                        || [
                            "int", "f32", "f64", "str", "length", "angle", "Geometry", "Vec",
                        ]
                        .contains(&name)
                    {
                        do yeet crate::error(format!("Duplicate or reserved type {name}"));
                    }
                    c.structs.insert(name.into(), a.clone());
                    let g = arr(&a, "generics")
                        .iter()
                        .map(|g| g.as_str().unwrap().into())
                        .collect();
                    for f in arr(&a, "fields") {
                        c.validate(&f["type"], &g, &Set::from([name.into()]))?
                    }
                }
                "destructure" => {
                    let v = c.eval(&a["value"], &e, 0)?;
                    if c.needs_assertion(&v) {
                        c.statement_checks(&v);
                    }
                    let checks = c.bind(&a["pattern"], v, &mut e)?;
                    if !checks.is_empty() {
                        c.statement_checks(&V::Void(checks));
                    }
                }
                "validate" | "assert" => c.check(s(&a, "kind"), &a["value"], &e)?,
                "segments" => {
                    if segments.is_some() {
                        do yeet crate::error("Duplicate segments declaration");
                    }
                    segments = Some(a["value"].clone())
                }
                "param" => {
                    if e.contains_key(name) {
                        do yeet crate::error(format!("Duplicate name {name}"));
                    }
                    c.parameters.push(a["parameter"].clone());
                    c.controls.push(a["control"].clone());
                    e.insert(name.into(), V::Json(json!({"param":name})));
                }
                "show" => {
                    c.show_serial += 1;
                    c.owner = format!("rs_{}", c.show_serial);
                    c.owner_serial = 0;
                    let v = c.eval(&a["value"], &e, 0)?;
                    root = c.geometry(v)?
                }
                "statement" => {
                    let value = c.eval(&a["value"], &e, 0)?;
                    c.statement_checks(&value);
                }
                _ => do yeet crate::error("Unknown statement"),
            }
        };
        result
            .map_err(|e| crate::error(format!("Rush line {}: {}", statement.line, e)))?;
    }
    if root.is_empty() {
        return Err(crate::error("No geometry to show"));
    }
    let mut result = json!({"nodes":c.nodes,"parameters":c.parameters,"customizer":c.controls,"root":root,"constraints":c.constraints,"checks":c.checks});
    if let Some(s) = segments {
        result["segments"] = s
    }
    Ok(result)
}
