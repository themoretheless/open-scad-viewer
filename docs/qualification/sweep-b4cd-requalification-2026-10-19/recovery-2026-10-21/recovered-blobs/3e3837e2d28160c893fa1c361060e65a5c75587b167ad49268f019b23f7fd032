// Fluent sequence lowering. Callbacks are compiled once with lexical locals;
// parameter-dependent lists execute in the bounded Rust evaluator.
impl Compiler {
    fn callback(&mut self, ast: &J, e: &Env, b: usize, arity: usize) -> R<(J, V)> {
        let callable = self.eval(ast, e, b)?;
        if let V::Function(f) = &callable {
            let inputs = arr(&f.ast, "inputs");
            let count = if arity == 1 && inputs.get(1).is_some_and(|input| input.get("default").is_none()) { 2 } else { arity };
            if inputs.len() < count || inputs[count..].iter().any(|input| input.get("default").is_none()) {
                return Err(crate::error(format!("Callback requires {arity} parameters (an optional index is allowed for single-item callbacks)")));
            }
            let mut scope = e.clone();
            let mut names = Vec::new();
            let mut args = Vec::new();
            for _ in 0..count {
                self.serial += 1;
                let local = format!("q{}", self.serial);
                scope.insert(local.clone(), V::Json(json!({"local":&local})));
                args.push(json!({"value":{"kind":"name","value":&local}}));
                names.push(local);
            }
            let body = self.invoke(f, &json!({"args":args}), &scope, b)?;
            return Ok((json!({"op":"lambda","parameters":names}), body));
        }
        let V::Lambda(f) = callable else {
            return Err(crate::error("Sequence callback requires a lambda or function"));
        };
        if f.parameters.len() != arity && !(arity == 1 && f.parameters.len() == 2) {
            return Err(crate::error(format!("Callback requires {arity} parameters")));
        }
        let mut scope = f.env.clone();
        let mut names = Vec::new();
        for name in &f.parameters {
            self.serial += 1;
            let local = format!("q{}", self.serial);
            scope.insert(name.clone(), V::Json(json!({"local":&local})));
            names.push(local);
        }
        let body = self.eval(&f.body, &scope, b)?;
        Ok((json!({"op":"lambda","parameters":names}), body))
    }
    fn query(&mut self, receiver: V, call: &J, e: &Env, b: usize) -> R<V> {
        let name = s(call, "value");
        let args = arr(call, "args");
        if args.iter().any(|a| a.get("name").is_some()) {
            return Err(crate::error("Sequence methods take positional arguments"));
        }
        let mut input = list(receiver)?;
        let arity = match name {
            "where" | "filter" | "select" | "map" | "selectMany" | "flatMap" | "orderBy"
            | "orderByDescending" | "thenBy" | "thenByDescending" | "takeWhile" | "skipWhile"
            | "groupBy" | "distinctBy" | "take" | "skip" | "chunk" | "window" | "concat"
            | "append" | "prepend" | "contains" | "elementAt" | "at" | "except" | "intersect"
            | "union" | "zip" => (1, 1),
            "aggregate" | "reduce" | "scan" => (2, 2),
            "join" | "groupJoin" => (4, 4),
            "count" | "any" | "all" | "first" | "last" | "single" | "sum" | "min" | "max"
            | "average" | "firstOrDefault" | "lastOrDefault" | "defaultIfEmpty" => (0, 1),
            "reverse" | "distinct" | "toArray" | "enumerate" | "length" | "flatten" => (0, 0),
            _ => return Err(crate::error(format!("Unknown sequence method {name}"))),
        };
        if args.len() < arity.0 || args.len() > arity.1 || (name == "all" && args.is_empty()) {
            return Err(crate::error(format!("Invalid argument count for {name}")));
        }
        if name == "toArray" {
            return Ok(V::Json(json!({"sequence":input})));
        }
        if ["select", "map", "selectMany", "flatMap", "where", "filter"].contains(&name) {
            let (mut f, body) = self.callback(&args[0]["value"], e, b, 1)?;
            if is_geometry(&body) {
                if ["where", "filter"].contains(&name) {
                    return Err(crate::error("Predicate must be numeric"));
                }
                let id = self.geometry(body)?;
                if arr(&f, "parameters").len() == 2 {
                    self.serial += 1;
                    let pair = format!("q{}", self.serial);
                    let names = arr(&f, "parameters").to_vec();
                    for node in &mut self.nodes {
                        replace_query_local(
                            node,
                            names[0].as_str().unwrap(),
                            &json!({"op":"at","input":{"local":&pair},"index":1}),
                        );
                        replace_query_local(
                            node,
                            names[1].as_str().unwrap(),
                            &json!({"op":"at","input":{"local":&pair},"index":0}),
                        );
                    }
                    input = json!({"op":"enumerate","input":input});
                    f["parameters"] = json!([pair]);
                }
                let result = self.add(
                    json!({"op":"collect","values":input,"binding":&f["parameters"][0],"input":id}),
                )?;
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
                return Ok(result);
            }
            f["body"] = if ["selectMany", "flatMap"].contains(&name) {
                list(body)?
            } else {
                expression(body)?
            };
            let op = match name {
                "where" | "filter" => "filter",
                "selectMany" | "flatMap" => "flatmap",
                _ => "map",
            };
            return Ok(V::Json(
                json!({"sequence":{"op":op,"input":input,"function":f}}),
            ));
        }
        if ["aggregate", "reduce", "scan"].contains(&name) {
            let initial = expression(self.eval(&args[0]["value"], e, b)?)?;
            let (mut f, body) = self.callback(&args[1]["value"], e, b, 2)?;
            f["body"] = expression(body)?;
            return Ok(V::Json(if name == "scan" {
                json!({"sequence":{"op":"query","method":"scan","input":input,"function":f,"argument":initial}})
            } else {
                json!({"op":"reduce","input":input,"function":f,"initial":initial})
            }));
        }
        if ["join", "groupJoin"].contains(&name) {
            let other = list(self.eval(&args[0]["value"], e, b)?)?;
            let mut functions = Vec::new();
            for (i, arg) in args[1..].iter().enumerate() {
                let (mut f, body) =
                    self.callback(&arg["value"], e, b, if i == 2 { 2 } else { 1 })?;
                f["body"] = expression(body)?;
                functions.push(f);
            }
            return Ok(V::Json(
                json!({"sequence":{"op":"query","method":name,"input":input,"argument":other,"functions":functions}}),
            ));
        }
        if name == "zip" {
            let other = list(self.eval(&args[0]["value"], e, b)?)?;
            return Ok(V::Json(
                json!({"sequence":{"op":"zip","inputs":[input,other]}}),
            ));
        }
        if ["at", "elementAt"].contains(&name) {
            let index = scalar(self.eval(&args[0]["value"], e, b)?)?;
            return Ok(V::Json(json!({"op":"at","input":input,"index":index})));
        }
        if name == "enumerate" {
            return Ok(V::Json(
                json!({"sequence":{"op":"enumerate","input":input}}),
            ));
        }
        let mut node = json!({"op":"query","method":name,"input":input});
        if ["orderBy", "orderByDescending", "thenBy", "thenByDescending"].contains(&name) {
            let (mut f, body) = self.callback(&args[0]["value"], e, b, 1)?;
            f["body"] = expression(body)?;
            let key = json!({"function":f,"descending":name.ends_with("Descending")});
            if name.starts_with("then") {
                input = node["input"].clone();
                if s(&input, "op") != "query" || s(&input, "method") != "orderBy" {
                    return Err(crate::error("thenBy must immediately follow orderBy or thenBy"));
                }
                let mut keys = arr(&input, "keys").to_vec();
                keys.push(key);
                input["keys"] = json!(keys);
                node = input;
            } else {
                node["method"] = json!("orderBy");
                node["keys"] = json!([key]);
            }
        } else if !args.is_empty() {
            let callback = [
                "takeWhile",
                "skipWhile",
                "groupBy",
                "distinctBy",
                "all",
                "count",
                "any",
                "first",
                "last",
                "single",
                "sum",
                "min",
                "max",
                "average",
            ]
            .contains(&name);
            if callback {
                let (mut f, body) = self.callback(&args[0]["value"], e, b, 1)?;
                f["body"] = expression(body)?;
                node["function"] = f;
            } else {
                node["argument"] = expression(self.eval(&args[0]["value"], e, b)?)?;
            }
        }
        let seq = [
            "orderBy",
            "orderByDescending",
            "thenBy",
            "thenByDescending",
            "takeWhile",
            "skipWhile",
            "groupBy",
            "distinctBy",
            "take",
            "skip",
            "chunk",
            "window",
            "concat",
            "append",
            "prepend",
            "except",
            "intersect",
            "union",
            "reverse",
            "distinct",
            "flatten",
            "defaultIfEmpty",
        ]
        .contains(&name);
        Ok(V::Json(if seq { json!({"sequence":node}) } else { node }))
    }
}

fn replace_query_local(node: &mut J, name: &str, replacement: &J) {
    if node["local"].as_str() == Some(name) {
        *node = replacement.clone();
        return;
    }
    match node {
        J::Array(xs) => {
            for x in xs {
                replace_query_local(x, name, replacement);
            }
        }
        J::Object(xs) => {
            for x in xs.values_mut() {
                replace_query_local(x, name, replacement);
            }
        }
        _ => {}
    }
}
