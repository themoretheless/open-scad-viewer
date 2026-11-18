use super::*;

impl Compiler {
    pub(super) fn add(&mut self, mut node: J) -> R<V> {
        if self.nodes.len() >= 128 {
            return Err(crate::error("At most 128 geometry nodes"));
        }
        self.serial += 1;
        let id = if self.stable_ids && !self.owner.is_empty() {
            self.owner_serial += 1;
            let hash = self.owner.bytes().fold(0xcbf29ce484222325u64, |h,b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3));
            format!("r{hash:016x}_{}", self.owner_serial)
        } else { format!("n{}", self.serial) };
        if self.nodes.iter().any(|n| s(n,"id") == id) { return Err(crate::error("Node identity collision")); }
        node["id"] = json!(&id);
        self.nodes.push(node);
        Ok(V::Json(json!({"geometry":id})))
    }
    pub(super) fn loft_references(&mut self, value: V, depth: usize) -> R<J> {
        if depth==0 { return Ok(json!(self.geometry(value)?)); }
        let V::Array(items)=value else { return Err(crate::error("Loft references require nested arrays")); };
        Ok(J::Array(items.into_iter().map(|v|self.loft_references(v,depth-1)).collect::<R<Vec<_>>>()?))
    }
    pub(super) fn geometry(&mut self, v: V) -> R<String> {
        match v {
            V::Array(a) => {
                let inputs = a
                    .into_iter()
                    .map(|v| self.geometry(v))
                    .collect::<R<Vec<_>>>()?;
                let g = self.add(json!({"op":"group","inputs":inputs}))?;
                let id = self.geometry(g)?;
                self.collections.insert(id.clone());
                Ok(id)
            }
            V::Json(j) if j.get("geometry").is_some() => Ok(s(&j, "geometry").into()),
            V::Json(j) if empty_value_sequence(&j["sequence"]) => {
                let value = self.geometry(V::Array(Vec::new()))?;
                let protected =
                    self.protect(V::Json(json!({"geometry":value})), &[j["sequence"].clone()])?;
                self.geometry(protected)
            }
            V::Json(j) if empty_match_sequence(&j["sequence"]) => {
                let sequence = &j["sequence"];
                let empty = self.geometry(V::Array(Vec::new()))?;
                let mut arms = Vec::new();
                for original in arr(sequence, "arms") {
                    let mut arm = original.clone();
                    let input = if s(&arm["body"], "op") != "list" {
                        let protected = self
                            .protect(V::Json(json!({"geometry":&empty})), &[arm["body"].clone()])?;
                        self.geometry(protected)?
                    } else {
                        empty.clone()
                    };
                    arm.as_object_mut().unwrap().remove("body");
                    arm["input"] = json!(input);
                    arms.push(arm);
                }
                let value =
                    self.add(json!({"op":"match","value":&sequence["input"],"arms":arms}))?;
                let id = self.geometry(value)?;
                self.collections.insert(id.clone());
                Ok(id)
            }
            _ => Err(crate::error("Expected geometry")),
        }
    }
    pub(super) fn mark(&mut self, v: &J) {
        if let Some(id) = v["param"].as_str() {
            if let Some(p) = self.parameters.iter_mut().find(|p| s(p, "id") == id) {
                p["integer"] = json!(true)
            }
            if let Some(c) = self.controls.iter_mut().find(|p| s(p, "name") == id) {
                c["step"] = json!(1)
            }
        }
    }
    pub(super) fn eval(&mut self, a: &J, e: &Env, b: usize) -> R<V> {
        if b > 64 {
            return Err(crate::error("Function expansion exceeds 64"));
        }
        let b = b + 1;
        let name = s(a, "value");
        match s(a, "kind") {
            "string" => return Ok(V::Json(json!({"text":name}))),
            "function" => {
                for generic in arr(a, "generics") {
                    for bound in arr(&a["bounds"], generic.as_str().unwrap()) {
                        let name = bound
                            .as_str()
                            .ok_or_else(|| crate::error("Invalid trait constraint"))?;
                        if !self.traits.contains_key(name) {
                            return Err(crate::error(format!("Unknown trait {name}")));
                        }
                    }
                }
                let mut generic: Set<String> = self
                    .active
                    .keys()
                    .cloned()
                    .chain(
                        arr(a, "generics")
                            .iter()
                            .filter_map(J::as_str)
                            .map(str::to_owned),
                    )
                    .collect();
                self.associated_names(a, &mut generic)?;
                for f in arr(a, "inputs").iter().chain(arr(a, "outputs")) {
                    self.validate(&f["type"], &generic, &Set::new())?
                }
                let ast = a.clone();
                return Ok(V::Function(Rc::new(Function {
                    ast,
                    env: e.clone(),
                    types: self.active.clone(),
                })));
            }
            "with" => {
                let base = self.eval(&a["left"], e, b)?;
                let V::Record(mut fields, nominal, marker, bounds) = base else {
                    return Err(crate::error(
                        "with requires a record or structure with known fields",
                    ));
                };
                let original = V::Record(fields.clone(), nominal.clone(), marker, bounds.clone());
                let declared_fields = nominal.as_ref().map(|t| self.type_fields(t)).transpose()?;
                let mut update_checks = Vec::new();
                for update in arr(&a["right"], "args") {
                    let key = s(update, "name");
                    let old = fields
                        .get(key)
                        .ok_or_else(|| crate::error(format!("with: unknown field {key}")))?;
                    let expected = if let Some(declared) = &declared_fields {
                        declared
                            .get(key)
                            .cloned()
                            .ok_or_else(|| crate::error("Missing declared field"))?
                    } else {
                        self.infer_type(old)?
                    };
                    let value = self.eval(&update["value"], e, b)?;
                    let value = self.check_type(
                        value,
                        &expected,
                        &Types::new(),
                        &format!("with.{key}"),
                        0,
                    )?;
                    self.gather_effects(&value, &mut update_checks);
                    fields.insert(key.into(), value);
                }
                let mut checks = Vec::new();
                self.gather_effects(&original, &mut checks);
                if !update_checks.is_empty() {
                    checks.push(json!({"op":"assert_value","value":{"op":"checked","checks":update_checks,"value":1}}));
                }
                return self.protect(V::Record(fields, nominal, marker, bounds), &checks);
            }
            "record" => {
                let mut r = Map::new();
                for arg in arr(a, "args") {
                    r.insert(s(arg, "name").into(), self.eval(&arg["value"], e, b)?);
                }
                let v = V::Record(r, None, false, Vec::new());
                return if name.is_empty() {
                    Ok(v)
                } else {
                    let args = arr(a, "types")
                        .iter()
                        .map(|t| substitute(t, &self.active, 0))
                        .collect::<R<Vec<_>>>()?;
                    self.check_type(v, &ty(name, args), &Types::new(), name, 0)
                };
            }
            "member" => {
                let v = self.eval(&a["left"], e, b)?;
                return match v {
                    V::Record(mut r, _, _, _) => r
                        .shift_remove(name)
                        .ok_or_else(|| crate::error(format!("Unknown record field {name}"))),
                    V::Json(j) if j.get("geometry").is_none() => {
                        Ok(V::Json(json!({"op":"field","input":j,"name":name})))
                    }
                    _ => Err(crate::error(format!("Unknown record field {name}"))),
                };
            }
            "number" => {
                let (n, u) = number(name)?;
                return Ok(V::Json(if u.is_empty() {
                    json!(n)
                } else {
                    json!({"op":"quantity","value":n,"unit":u})
                }));
            }
            "name" if name == "true" || name == "false" => {
                return Ok(V::Json(json!(if name == "true" { 1 } else { 0 })));
            }
            "name" => {
                return e
                    .get(name)
                    .cloned()
                    .ok_or_else(|| crate::error(format!("Unknown name {name}")));
            }
            "array" => {
                return arr(a, "items")
                    .iter()
                    .map(|a| self.eval(a, e, b))
                    .collect::<R<Vec<_>>>()
                    .map(V::Array);
            }
            "index" => {
                let value = self.eval(&a["left"], e, b)?;
                let index = scalar(self.eval(&a["right"], e, b)?)?;
                if is_geometry(&value) {
                    let V::Array(items) = value else {
                        return Err(crate::error("Indexing requires a list"));
                    };
                    let mut arms = Vec::new();
                    for (i, item) in items.into_iter().enumerate() {
                        arms.push(json!({"pattern":{"kind":"literal","value":i},"input":self.geometry(item)?}));
                    }
                    return self.add(json!({"op":"match","value":index,"arms":arms}));
                }
                return Ok(V::Json(
                    json!({"op":"at","input":list(value)?,"index":index}),
                ));
            }
            "interval" => {
                let mode = arr(a, "args").first();
                let amount = if let Some(m) = mode {
                    let v = scalar(self.eval(&m["value"], e, b)?)?;
                    if s(m, "name") == "count" {
                        self.mark(&v)
                    }
                    Some(v)
                } else {
                    None
                };
                let start = scalar(self.eval(&a["left"], e, b)?)?;
                let end = scalar(self.eval(&a["right"], e, b)?)?;
                let mut node =
                    json!({"op":"interval","start":start,"end":end,"inclusive":name==".."});
                if let Some(v) = amount {
                    node[if s(mode.unwrap(), "name") == "by" {
                        "step"
                    } else {
                        "count"
                    }] = v
                }
                return Ok(V::Json(json!({"sequence":node})));
            }
            "comprehension" => return self.comprehension(a, 0, e, b),
            "match" => return self.match_value(a, e, b),
            "match_block" => {
                let mut scope = e.clone();
                let mut names = Set::new();
                let mut checks = Vec::new();
                for item in arr(a, "items") {
                    if ["assert", "validate"].contains(&s(item, "kind")) {
                        checks.push(self.local_check(s(item, "kind"), &item["left"], &scope)?);
                        continue;
                    }
                    let pattern = item
                        .get("pattern")
                        .cloned()
                        .unwrap_or_else(|| json!({"kind":"name","value":s(item,"name")}));
                    let bindings = binding_names(&pattern);
                    for name in &bindings {
                        if !names.insert(name.to_string()) {
                            return Err(crate::error(format!(
                                "Duplicate match body binding {name}"
                            )));
                        }
                    }
                    let value = self.eval(&item["value"], &scope, b)?;
                    self.gather_effects(&value, &mut checks);
                    for name in bindings {
                        scope.shift_remove(name);
                    }
                    checks.extend(self.bind(&pattern, value, &mut scope)?);
                }
                let value = self.eval(&a["left"], &scope, b)?;
                return self.protect(value, &checks);
            }
            "conditional" => {
                let condition = scalar(self.eval(&a["left"], e, b)?)?;
                let yes = self.eval(&a["items"][0], e, b)?;
                let no = self.eval(&a["items"][1], e, b)?;
                if is_geometry(&yes) || is_geometry(&no) {
                    if !is_geometry(&yes) || !is_geometry(&no) {
                        return Err(crate::error(
                            "Conditional branches must both be geometry or both be values",
                        ));
                    }
                    let y = self.geometry(yes)?;
                    let n = self.geometry(no)?;
                    let result =
                        self.add(json!({"op":"if","condition":&condition,"then":&y,"else":&n}))?;
                    if self.collections.contains(&y) || self.collections.contains(&n) {
                        let id = self.geometry(result.clone())?;
                        self.collections.insert(id);
                    }
                    return Ok(result);
                }
                if let (V::Array(y), V::Array(n)) = (&yes, &no)
                    && y.len() == n.len()
                {
                    return y.iter().cloned().zip(n.iter().cloned()).map(|(y,n)|Ok(V::Json(json!({"op":"if","condition":&condition,"then":expression(y)?,"else":expression(n)?})))).collect::<R<Vec<_>>>().map(V::Array);
                }
                let seq = matches!(&yes, V::Array(_))
                    || matches!(&no, V::Array(_))
                    || matches!(&yes,V::Json(j) if j.get("sequence").is_some())
                    || matches!(&no,V::Json(j) if j.get("sequence").is_some());
                let node = json!({"op":"if","condition":&condition,"then":expression(yes)?,"else":expression(no)?});
                return Ok(V::Json(if seq { json!({"sequence":node}) } else { node }));
            }
            "lambda" => {
                return Ok(V::Lambda(Rc::new(Lambda {
                    parameters: if arr(a, "parameters").is_empty() {
                        vec![name.into()]
                    } else {
                        arr(a, "parameters")
                            .iter()
                            .map(|p| p.as_str().unwrap().into())
                            .collect()
                    },
                    body: a["left"].clone(),
                    env: e.clone(),
                })));
            }
            "not" => {
                return Ok(V::Json(
                    json!({"op":"not","value":scalar(self.eval(&a["left"],e,b)?)?}),
                ));
            }
            "neg" => {
                return Ok(V::Json(
                    json!({"op":"negate","value":scalar(self.eval(&a["left"],e,b)?)?}),
                ));
            }
            "binary" if name == "==" || name == "!=" => {
                let left = expression(self.eval(&a["left"], e, b)?)?;
                let right = expression(self.eval(&a["right"], e, b)?)?;
                let eq = json!({"op":"eq","args":[left,right]});
                return Ok(V::Json(if name == "!=" {
                    json!({"op":"not","value":eq})
                } else {
                    eq
                }));
            }
            "binary" => {
                let mut left = scalar(self.eval(&a["left"], e, b)?)?;
                let mut right = scalar(self.eval(&a["right"], e, b)?)?;
                if name == ">" || name == ">=" {
                    std::mem::swap(&mut left, &mut right)
                }
                let op = match name {
                    "&&" => "and",
                    "||" => "or",
                    "+" => "add",
                    "-" => "subtract",
                    "*" => "multiply",
                    "/" => "divide",
                    "%" => "mod",
                    "**" => "pow",
                    "<" | ">" => "lt",
                    "<=" | ">=" => "le",
                    _ => "eq",
                };
                let result = json!({"op":op,"args":[left,right]});
                return Ok(V::Json(if name == "!=" {
                    json!({"op":"not","value":result})
                } else {
                    result
                }));
            }
            _ => {}
        }
        let (call, input) = if s(a, "kind") == "pipe" {
            if s(&a["left"], "kind") == "name"
                && !e.contains_key(s(&a["left"], "value"))
                && self.traits.contains_key(s(&a["left"], "value"))
            {
                let mut call = a["right"].clone();
                let args = arr(&call, "args");
                let first = args
                    .first()
                    .ok_or_else(|| crate::error("Qualified trait call requires a receiver"))?;
                if first.get("name").is_some() {
                    return Err(crate::error("Qualified trait receiver must be positional"));
                }
                let receiver = self.eval(&first["value"], e, b)?;
                call["args"] = json!(args[1..].to_vec());
                return self.invoke_method(receiver, &call, e, b, Some(s(&a["left"], "value")));
            }
            let v = self.eval(&a["left"], e, b)?;
            if matches!(&v, V::Record(..)) {
                return self.invoke_method(v, &a["right"], e, b, None);
            }
            let empty_geometry = matches!(&v,V::Array(items) if items.is_empty())
                && ["move", "translate", "rotate", "scale", "mirror"]
                    .contains(&s(&a["right"], "value"));
            if !is_geometry(&v)
                && !empty_geometry
                && (matches!(&v, V::Array(_))
                    || matches!(&v, V::Json(j) if j.get("sequence").is_some() || (j.get("geometry").is_none() && j.get("text").is_none())))
            {
                return self.query(v, &a["right"], e, b);
            }
            (&a["right"], Some(self.geometry(v)?))
        } else {
            (a, None)
        };
        if s(call, "kind") != "call" {
            return Err(crate::error("Expected a call"));
        }
        let name = s(call, "value");
        let args = arr(call, "args");
        if ["zip", "enumerate", "length", "at"].contains(&name) {
            if input.is_some() || args.iter().any(|a| a.get("name").is_some()) {
                return Err(crate::error(
                    "Sequence functions require positional arguments",
                ));
            }
            let mut values = args
                .iter()
                .map(|a| self.eval(&a["value"], e, b))
                .collect::<R<Vec<_>>>()?;
            if name == "zip" {
                if !(2..=8).contains(&values.len()) {
                    return Err(crate::error("zip expects 2..8 sequences"));
                }
                return Ok(V::Json(
                    json!({"sequence":{"op":"zip","inputs":values.into_iter().map(list).collect::<R<Vec<_>>>()?}}),
                ));
            }
            if values.len() != if name == "at" { 2 } else { 1 } {
                return Err(crate::error("Invalid sequence function arity"));
            }
            let seq = list(values.remove(0))?;
            return Ok(V::Json(match name {
                "length" => json!({"op":"length","input":seq}),
                "at" => json!({"op":"at","input":seq,"index":scalar(values.remove(0))?}),
                _ => json!({"sequence":{"op":"enumerate","input":seq}}),
            }));
        }
        if name == "repeat" {
            if input.is_some() || args.len() != 2 || args.iter().any(|a| a.get("name").is_some()) {
                return Err(crate::error("repeat(count, i => geometry) expected"));
            }
            let count = scalar(self.eval(&args[0]["value"], e, b)?)?;
            self.mark(&count);
            let (f, value) = self.callback(&args[1]["value"], e, b, 1)?;
            if arr(&f, "parameters").len() != 1 {
                return Err(crate::error("repeat callback requires one index parameter"));
            }
            let id = self.geometry(value)?;
            return self
                .add(json!({"op":"map","count":count,"index":&f["parameters"][0],"input":id}));
        }
        if let Some(f) = e.get(name) {
            match f {
                V::Lambda(f) => {
                    if input.is_some()
                        || args.len() != f.parameters.len()
                        || args.iter().any(|a| a.get("name").is_some())
                    {
                        return Err(crate::error("Lambda argument count mismatch"));
                    }
                    let mut scope = f.env.clone();
                    for (name, arg) in f.parameters.iter().zip(args) {
                        scope.insert(name.clone(), self.eval(&arg["value"], e, b)?);
                    }
                    return self.eval(&f.body, &scope, b);
                }
                V::Function(f) => {
                    if input.is_some() {
                        return Err(crate::error(
                            "Block functions are called directly, not through geometry pipelines",
                        ));
                    }
                    return self.invoke(f, call, e, b);
                }
                _ => {}
            }
        }
        let values = args
            .iter()
            .map(|a| {
                Ok((
                    a["name"].as_str().map(str::to_owned),
                    self.eval(&a["value"], e, b)?,
                ))
            })
            .collect::<R<Vec<_>>>()?;
        self.operation(name, input, values)
    }
}
