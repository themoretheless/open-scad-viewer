//! Compilation contract: preserves the original arithmetic and proof budgets.
use super::*;

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
