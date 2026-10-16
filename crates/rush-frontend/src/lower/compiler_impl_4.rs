use super::*;

impl Compiler {
    pub(super) fn validate(&self, t: &J, g: &Set<String>, visiting: &Set<String>) -> R<()> {
        let name = s(t, "name");
        let args = arr(t, "args");
        if name == "$record" {
            for field in t["fields"]
                .as_object()
                .ok_or_else(|| crate::error("Invalid record type"))?
                .values()
            {
                self.validate(field, g, visiting)?;
            }
            return Ok(());
        }
        if g.contains(name) {
            if !args.is_empty() {
                return Err(crate::error("Generic parameter cannot have type arguments"));
            }
            return Ok(());
        }
        if ["int", "f32", "f64", "str", "length", "angle", "Geometry"].contains(&name) {
            if !args.is_empty() {
                return Err(crate::error(format!("Type {name} takes no arguments")));
            }
            return Ok(());
        }
        if name == "Vec" {
            if args.len() != 1 {
                return Err(crate::error("Vec requires one element type"));
            }
            return self.validate(&args[0], g, visiting);
        }
        let def = self
            .structs
            .get(name)
            .ok_or_else(|| crate::error(format!("Unknown type {name}")))?;
        if visiting.contains(name) {
            return Err(crate::error("Recursive value structures are not supported"));
        }
        if args.len() != arr(def, "generics").len() {
            return Err(crate::error(format!(
                "Wrong type argument count for {name}"
            )));
        }
        for a in args {
            self.validate(a, g, visiting)?
        }
        Ok(())
    }
    pub(super) fn infer_type(&self, v: &V) -> R<J> {
        match v {
            V::Record(fields, t, _, _) => {
                if let Some(t) = t {
                    return Ok(t.clone());
                }
                let mut types = value_codec::Map::new();
                for (name, value) in fields {
                    types.insert(name.clone(), self.infer_type(value)?);
                }
                return Ok(json!({"name":"$record","args":[],"fields":J::Object(types)}));
            }
            V::Array(a) => {
                let first = a.first().ok_or_else(|| {
                    crate::error(
                        "Cannot infer element type of an empty vector; specify generic arguments",
                    )
                })?;
                let element = self.infer_type(first)?;
                for v in a {
                    if typename(&self.infer_type(v)?) != typename(&element) {
                        return Err(crate::error("Generic vector requires one element type"));
                    }
                }
                return Ok(ty("Vec", vec![element]));
            }
            V::Json(j) => {
                if let Some(sequence) = j.get("sequence")
                    && ["typed_value", "checked", "memo"].contains(&s(sequence, "op"))
                {
                    return self.infer_type(&V::Json(sequence.clone()));
                }
                if s(j, "op") == "negate" {
                    return self.infer_type(&V::Json(j["value"].clone()));
                }
                if ["add", "subtract", "multiply", "divide", "mod"].contains(&s(j, "op"))
                    && arr(j, "args").len() == 2
                {
                    let left = self.infer_type(&V::Json(j["args"][0].clone()))?;
                    let right = self.infer_type(&V::Json(j["args"][1].clone()))?;
                    let physical = |t: &J| ["length", "angle"].contains(&s(t, "name"));
                    let op = s(j, "op");
                    if ["add", "subtract", "mod"].contains(&op)
                        && typename(&left) == typename(&right)
                    {
                        return Ok(left);
                    }
                    if op == "multiply" && physical(&left) && !physical(&right) {
                        return Ok(left);
                    }
                    if op == "multiply" && !physical(&left) && physical(&right) {
                        return Ok(right);
                    }
                    if op == "divide" && physical(&left) && !physical(&right) {
                        return Ok(left);
                    }
                    if !physical(&left)
                        && !physical(&right)
                        && op != "divide"
                        && s(&left, "name") == "int"
                        && s(&right, "name") == "int"
                    {
                        return Ok(left);
                    }
                    return Ok(ty("f64", vec![]));
                }
                let name = if j.get("text").is_some() || s(j, "op") == "text" {
                    "str"
                } else if j.get("geometry").is_some() {
                    "Geometry"
                } else if ["checked", "memo"].contains(&s(j, "op")) {
                    return self.infer_type(&V::Json(j["value"].clone()));
                } else if s(j, "op") == "typed_value" {
                    return Ok(j["type"].clone());
                } else if s(j, "op") == "typed" {
                    s(j, "type")
                } else if s(j, "op") == "quantity" {
                    if ["deg", "rad"].contains(&s(j, "unit")) {
                        "angle"
                    } else {
                        "length"
                    }
                } else if let Some(id) = j["param"].as_str() {
                    let p = self
                        .parameters
                        .iter()
                        .find(|p| s(p, "id") == id)
                        .ok_or_else(|| crate::error("Unknown parameter"))?;
                    if p.get("unit").is_some() {
                        if ["deg", "rad"].contains(&s(p, "unit")) {
                            "angle"
                        } else {
                            "length"
                        }
                    } else if flag(p, "integer") {
                        "int"
                    } else {
                        "f64"
                    }
                } else if j.as_f64().is_some_and(|n| n.fract() == 0.0) {
                    "int"
                } else {
                    "f64"
                };
                return Ok(ty(name, vec![]));
            }
            _ => {}
        }
        Ok(ty("f64", vec![]))
    }
    pub(super) fn check_type(&mut self, v: V, t: &J, bs: &Types, label: &str, level: usize) -> R<V> {
        if level > 16 {
            return Err(crate::error("Record type nesting exceeds 16"));
        }
        let t = substitute(t, bs, 0)?;
        let name = s(&t, "name");
        if name == "$record" {
            let V::Record(mut fields, nominal, marker, bounds) = v else {
                return Err(crate::error(format!("{label}: expected record")));
            };
            let expected = t["fields"]
                .as_object()
                .ok_or_else(|| crate::error("Invalid structural record type"))?;
            if fields.len() != expected.len() {
                return Err(crate::error(format!("{label}: record fields must match")));
            }
            for (key, field_type) in expected {
                let value = fields
                    .shift_remove(key.as_str())
                    .ok_or_else(|| crate::error(format!("{label}: missing field {key}")))?;
                fields.insert(
                    key.clone(),
                    self.check_type(value, field_type, bs, &format!("{label}.{key}"), level + 1)?,
                );
            }
            return Ok(V::Record(fields, nominal, marker, bounds));
        }
        if (name == "str" || name == "Vec" || self.structs.contains_key(name))
            && matches!(&v,V::Json(j) if j.get("text").is_none() && j.get("geometry").is_none())
        {
            let descriptor = self.runtime_type_descriptor(&t, level)?;
            let checked = json!({"op":"typed_value","type":descriptor,"value":expression(v)?});
            return Ok(V::Json(if name == "Vec" {
                json!({"sequence":checked})
            } else {
                checked
            }));
        }
        if name == "str" {
            if !matches!(&v,V::Json(j) if j.get("text").is_some()) {
                return Err(crate::error(format!("{label}: expected str")));
            }
            return Ok(v);
        }
        if name == "Geometry" {
            self.geometry(v.clone())?;
            return Ok(v);
        }
        if ["int", "f32", "f64", "length", "angle"].contains(&name) {
            let v = scalar(v)?;
            if let Some(n) = v.as_f64() {
                if name == "int" && n.fract() != 0.0 {
                    return Err(crate::error(format!("{label}: expected int")));
                }
                if name == "length" || name == "angle" {
                    return Err(crate::error(format!("{label}: expected {name} units")));
                }
            }
            if s(&v, "op") == "quantity" && name != "length" && name != "angle" {
                return Err(crate::error(format!(
                    "{label}: expected dimensionless {name}"
                )));
            }
            return Ok(V::Json(json!({"op":"typed","type":name,"value":v})));
        }
        if name == "Vec" {
            if let V::Array(a) = v
                && arr(&t, "args").len() == 1
            {
                return a
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        self.check_type(v, &t["args"][0], bs, &format!("{label}[{i}]"), level + 1)
                    })
                    .collect::<R<Vec<_>>>()
                    .map(V::Array);
            }
            return Err(crate::error(format!(
                "{label}: expected {} literal vector",
                typename(&t)
            )));
        }
        let def = self.structs.get(name).cloned().ok_or_else(|| {
            crate::error(format!(
                "{label}: unknown or incomplete type {}",
                typename(&t)
            ))
        })?;
        if arr(&t, "args").len() != arr(&def, "generics").len() {
            return Err(crate::error(format!(
                "{label}: unknown or incomplete type {}",
                typename(&t)
            )));
        }
        let V::Record(mut r, old, _, bounds) = v else {
            return Err(crate::error(format!(
                "{label}: expected {} record",
                typename(&t)
            )));
        };
        if let Some(old) = old
            && typename(&old) != typename(&t)
        {
            return Err(crate::error(format!(
                "{label}: expected {}, got {}",
                typename(&t),
                typename(&old)
            )));
        }
        if r.len() != arr(&def, "fields").len()
            || arr(&def, "fields")
                .iter()
                .any(|f| !r.contains_key(s(f, "name")))
        {
            return Err(crate::error(format!(
                "{label}: fields must exactly match {name}"
            )));
        }
        let types = arr(&def, "generics")
            .iter()
            .zip(arr(&t, "args"))
            .map(|(g, t)| (g.as_str().unwrap().into(), t.clone()))
            .collect();
        let mut out = Map::new();
        for f in arr(&def, "fields") {
            let key = s(f, "name");
            let value = r.shift_remove(key).unwrap();
            out.insert(
                key.into(),
                self.check_type(
                    value,
                    &substitute(&f["type"], &types, 0)?,
                    bs,
                    &format!("{label}.{key}"),
                    level + 1,
                )?,
            );
        }
        Ok(V::Record(out, Some(t), false, bounds))
    }
    pub(super) fn type_fields(&self, t: &J) -> R<Types> {
        if s(t, "name") == "$record" {
            return Ok(t["fields"]
                .as_object()
                .ok_or_else(|| crate::error("Invalid structural type"))?
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect());
        }
        let def = self
            .structs
            .get(s(t, "name"))
            .ok_or_else(|| crate::error("Expected a record type"))?;
        let bs: Types = arr(def, "generics")
            .iter()
            .zip(arr(t, "args"))
            .map(|(g, t)| (g.as_str().unwrap().into(), t.clone()))
            .collect();
        arr(def, "fields")
            .iter()
            .map(|f| Ok((s(f, "name").into(), substitute(&f["type"], &bs, 0)?)))
            .collect()
    }
    pub(super) fn invoke(&mut self, f: &Function, call: &J, caller: &Env, b: usize) -> R<V> {
        if b > 64 {
            return Err(crate::error("Function expansion exceeds 64"));
        }
        self.expansions += 1;
        if self.expansions > 256 {
            return Err(crate::error("Block function call expansion exceeds 256"));
        }
        let def = &f.ast;
        let mut bs = f.types.clone();
        let generic: Set<String> = arr(def, "generics")
            .iter()
            .map(|g| g.as_str().unwrap().into())
            .collect();
        let mut allowed = bs.keys().cloned().chain(generic.iter().cloned()).collect();
        self.associated_names(def, &mut allowed)?;
        for field in arr(def, "inputs").iter().chain(arr(def, "outputs")) {
            self.validate(&field["type"], &allowed, &Set::new())?
        }
        let explicit = call.get("types").is_some();
        if explicit {
            if arr(call, "types").len() != arr(def, "generics").len() {
                return Err(crate::error("Wrong generic argument count"));
            }
            for (t, g) in arr(call, "types").iter().zip(arr(def, "generics")) {
                let resolved = substitute(t, &self.active, 0)?;
                self.validate(&resolved, &Set::new(), &Set::new())?;
                bs.insert(g.as_str().unwrap().into(), resolved);
            }
        }
        let args = arr(call, "args");
        let inputs = arr(def, "inputs");
        if args.len() > inputs.len() {
            return Err(crate::error("Function argument count mismatch"));
        }
        let named = args.iter().any(|a| a.get("name").is_some());
        let mut names = Set::new();
        if named
            && args.iter().any(|a| {
                a.get("name").is_none()
                    || !names.insert(s(a, "name"))
                    || !inputs.iter().any(|f| s(f, "name") == s(a, "name"))
            })
        {
            return Err(crate::error(
                "Named arguments must exactly match parameters",
            ));
        }
        // Defaults see the declaration environment and already-bound parameters,
        // while explicitly supplied arguments see the caller's environment.
        let mut scope = f.env.clone();
        for (i, field) in inputs.iter().enumerate() {
            let name = s(field, "name");
            let arg = if named {
                args.iter().find(|a| s(a, "name") == name)
            } else {
                args.get(i)
            };
            let value = if let Some(arg) = arg {
                self.eval(&arg["value"], caller, b + 1)?
            } else if let Some(default) = field.get("default") {
                let previous = std::mem::replace(&mut self.active, bs.clone());
                let value = self.eval(default, &scope, b + 1);
                self.active = previous;
                value?
            } else {
                return Err(crate::error(format!(
                    "Function argument count mismatch: missing required argument {name}"
                )));
            };
            if !generic.is_empty() && !explicit {
                infer(
                    &field["type"],
                    &self.infer_type(&value)?,
                    &generic,
                    &mut bs,
                    explicit,
                )?;
            }
            self.resolve_function_bounds(def, &mut bs)?;
            let value =
                self.check_type(value, &field["type"], &bs, &format!("argument {name}"), 0)?;
            let mut value = self.constrain_receiver(value, &field["type"], &def["bounds"], 0)?;
            if name == "self"
                && let (Some(owner), V::Record(_, _, _, bounds)) =
                    (def["traitOwner"].as_str(), &mut value)
            {
                *bounds = vec![owner.into()];
            }
            scope.insert(name.into(), value);
        }
        for g in &generic {
            if !bs.contains_key(g) {
                return Err(crate::error(format!(
                    "Cannot infer {g}; provide explicit type arguments"
                )));
            }
        }
        self.resolve_function_bounds(def, &mut bs)?;
        let previous = std::mem::replace(&mut self.active, bs.clone());
        let result: R<V> = try {
            let mut locals: Set<String> = inputs.iter().map(|f| s(f, "name").into()).collect();
            let mut statement_checks = Vec::new();
            for st in arr(def, "items") {
                if ["assert", "validate"].contains(&s(st, "kind")) {
                    statement_checks.push(self.local_check(s(st, "kind"), &st["left"], &scope)?);
                    continue;
                }
                if s(st, "kind") == "statement" {
                    let value = self.eval(&st["left"], &scope, b + 1)?;
                    self.gather_effects(&value, &mut statement_checks);
                    continue;
                }
                let pattern = &st["left"];
                let names = if s(pattern, "kind") == "name" {
                    vec![s(pattern, "value")]
                } else {
                    arr(pattern, "items")
                        .iter()
                        .map(|i| s(i, "value"))
                        .collect()
                };
                for name in names {
                    if !locals.insert(name.into()) {
                        do yeet crate::error(format!("Duplicate local {name}"));
                    }
                    scope.shift_remove(name);
                }
                let v = self.eval(&st["right"], &scope, b + 1)?;
                self.gather_effects(&v, &mut statement_checks);
                statement_checks.extend(self.bind(pattern, v, &mut scope)?);
            }
            let result = self.eval(&def["left"], &scope, b + 1)?;
            let output = if flag(def, "inferResult") {
                if matches!(result, V::Void(_)) {
                    do yeet crate::error("A result expression cannot return a no-result function");
                }
                result
            } else if flag(def, "singleResult") {
                self.check_type(result, &def["outputs"][0]["type"], &bs, "result", 0)?
            } else {
                let V::Record(mut r, _, _, _) = result else {
                    do yeet crate::error("ret must return named fields");
                };
                if r.len() != arr(def, "outputs").len()
                    || arr(def, "outputs")
                        .iter()
                        .any(|f| !r.contains_key(s(f, "name")))
                {
                    do yeet crate::error("ret fields must exactly match named results");
                }
                let mut out = Map::new();
                for field in arr(def, "outputs") {
                    let name = s(field, "name");
                    out.insert(
                        name.into(),
                        self.check_type(
                            r.shift_remove(name).unwrap(),
                            &field["type"],
                            &bs,
                            &format!("result {name}"),
                            0,
                        )?,
                    );
                }
                V::Record(out, None, false, Vec::new())
            };
            let mut checks = Vec::new();
            for field in inputs {
                self.gather_effects(scope.get(s(field, "name")).unwrap(), &mut checks)
            }
            checks.extend(statement_checks);
            gather(&output, &mut checks);
            if checks.len() > 256 {
                do yeet crate::error("Function type checks exceed 256");
            }
            if flag(def, "voidResult") {
                V::Void(checks)
            } else {
                let output = self.protect(output, &checks)?;
                self.memo_value(output)?
            }
        };
        self.active = previous;
        result
    }
}
