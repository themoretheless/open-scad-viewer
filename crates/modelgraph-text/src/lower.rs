use crate::ordered_map::OrderedMap as Map;
use crate::parser::{Statement, number};
use crate::value::json;
use std::{collections::BTreeSet as Set, rc::Rc};
use value_codec::Value as J;
type R<T> = crate::Result<T>;
type Env = Map<String, V>;
type Types = Map<String, J>;
#[derive(Clone)]
enum V {
    Json(J),
    Array(Vec<V>),
    Record(Map<String, V>, Option<J>, bool, Vec<String>),
    Function(Rc<Function>),
    Lambda(Rc<Lambda>),
    Void(Vec<J>),
}
struct Function {
    ast: J,
    env: Env,
    types: Types,
}
struct Lambda {
    parameters: Vec<String>,
    body: J,
    env: Env,
}
fn s<'a>(v: &'a J, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn arr<'a>(v: &'a J, k: &str) -> &'a [J] {
    v[k].as_array().map_or(&[], Vec::as_slice)
}
fn flag(v: &J, k: &str) -> bool {
    v[k].as_bool().unwrap_or(false)
}
fn ty(name: &str, args: Vec<J>) -> J {
    json!({"name":name,"args":args})
}
fn typename(t: &J) -> String {
    let args = arr(t, "args");
    if s(t, "name") == "$record" {
        return format!(
            "{{{}}}",
            t["fields"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| format!("{k}:{}", typename(v)))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    format!(
        "{}{}",
        s(t, "name"),
        if args.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                args.iter().map(typename).collect::<Vec<_>>().join(",")
            )
        }
    )
}
fn substitute(t: &J, bs: &Types, depth: usize) -> R<J> {
    if depth > 16 {
        return Err(crate::error("Type substitution depth exceeds 16"));
    }
    if let Some(x) = bs.get(s(t, "name")) {
        return Ok(x.clone());
    }
    if s(t, "name") == "$record" {
        return Ok(t.clone());
    }
    Ok(ty(
        s(t, "name"),
        arr(t, "args")
            .iter()
            .map(|a| substitute(a, bs, depth + 1))
            .collect::<R<_>>()?,
    ))
}
fn is_geometry(v: &V) -> bool {
    match v {
        V::Array(a) => !a.is_empty() && a.iter().all(is_geometry),
        V::Json(j) => j.get("geometry").is_some(),
        _ => false,
    }
}
fn scalar(v: V) -> R<J> {
    match v {
        V::Json(j)
            if !["geometry", "sequence", "text"]
                .iter()
                .any(|k| j.get(k).is_some()) =>
        {
            Ok(j)
        }
        _ => Err(crate::error("Expected scalar")),
    }
}
fn expression(v: V) -> R<J> {
    match v {
        V::Array(a) => {
            Ok(json!({"op":"list","items":a.into_iter().map(expression).collect::<R<Vec<_>>>()?}))
        }
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        V::Json(j) if j.get("text").is_some() => Ok(json!({"op":"text","value":&j["text"]})),
        V::Record(fields, _, _, _) => {
            let mut result = value_codec::Map::new();
            for (name, value) in fields {
                result.insert(name, expression(value)?);
            }
            Ok(json!({"op":"record","fields":J::Object(result)}))
        }
        v => scalar(v),
    }
}
fn list(v: V) -> R<J> {
    match &v {
        V::Array(_) => expression(v),
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        V::Json(j) if j.get("geometry").is_none() && j.get("text").is_none() => Ok(j.clone()),
        _ => Err(crate::error("Expected a sequence")),
    }
}
fn raw(v: V) -> R<J> {
    match v {
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        V::Json(j) => Ok(j),
        V::Array(a) => a.into_iter().map(raw).collect::<R<Vec<_>>>().map(J::Array),
        _ => Err(crate::error("Expected numeric geometry argument")),
    }
}
struct Compiler {
    nodes: Vec<J>,
    parameters: Vec<J>,
    controls: Vec<J>,
    constraints: Vec<J>,
    checks: Vec<J>,
    collections: Set<String>,
    structs: Map<String, J>,
    traits: Map<String, J>,
    trait_envs: Map<String, Env>,
    implementations: Map<String, (J, Env)>,

    active: Types,
    serial: usize,
    check_serial: usize,
    expansions: usize,
}
impl Compiler {
    fn add(&mut self, mut node: J) -> R<V> {
        if self.nodes.len() >= 128 {
            return Err(crate::error("At most 128 geometry nodes"));
        }
        self.serial += 1;
        let id = format!("n{}", self.serial);
        node["id"] = json!(&id);
        self.nodes.push(node);
        Ok(V::Json(json!({"geometry":id})))
    }
    fn geometry(&mut self, v: V) -> R<String> {
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
    fn mark(&mut self, v: &J) {
        if let Some(id) = v["param"].as_str() {
            if let Some(p) = self.parameters.iter_mut().find(|p| s(p, "id") == id) {
                p["integer"] = json!(true)
            }
            if let Some(c) = self.controls.iter_mut().find(|p| s(p, "name") == id) {
                c["step"] = json!(1)
            }
        }
    }
    fn eval(&mut self, a: &J, e: &Env, b: usize) -> R<V> {
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
    fn comprehension(&mut self, a: &J, offset: usize, outer: &Env, b: usize) -> R<V> {
        if b > 64 {
            return Err(crate::error("Function expansion exceeds 64"));
        }
        let clauses = arr(a, "items");
        let clause = &clauses[offset];
        if s(clause, "kind") != "for" {
            return Err(crate::error("Generator must start with for"));
        }
        let mut input = list(self.eval(&clause["left"], outer, b)?)?;
        self.serial += 1;
        let binding = format!("v{}", self.serial);
        let mut scope = outer.clone();
        for (i, n) in arr(clause, "items").iter().enumerate() {
            scope.insert(
                s(n, "value").into(),
                V::Json(if arr(clause, "items").len() == 1 {
                    json!({"local":&binding})
                } else {
                    json!({"op":"at","input":{"local":&binding},"index":i})
                }),
            );
        }
        let mut next = offset + 1;
        let mut steps = Vec::new();
        let mut guarded = false;

        while next < clauses.len() && s(&clauses[next], "kind") != "for" {
            let c = &clauses[next];
            next += 1;
            if ["assert", "validate"].contains(&s(c, "kind")) {
                let check = self.local_check(s(c, "kind"), &c["left"], &scope)?;
                guarded = true;
                steps.push(json!({"kind":"assert","value":check}));
            } else if s(c, "kind") == "let" {
                let v = self.eval(&c["left"], &scope, b)?;
                let mut checks = Vec::new();
                self.gather_effects(&v, &mut checks);
                let pattern = c
                    .get("pattern")
                    .cloned()
                    .unwrap_or_else(|| json!({"kind":"name","value":s(&c["items"][0],"value")}));
                for name in binding_names(&pattern) {
                    scope.shift_remove(name);
                }
                checks.extend(self.bind(&pattern, v, &mut scope)?);
                if !checks.is_empty() {
                    guarded = true;
                    steps.push(
                        json!({"kind":"assert","value":{"op":"checked","checks":checks,"value":1}}),
                    );
                }
            } else {
                let v = scalar(self.eval(&c["left"], &scope, b)?)?;
                guarded |= s(c, "kind") == "while";
                steps.push(json!({"kind":s(c,"kind"),"value":v}));
            }
        }
        if guarded {
            // One ordered pass keeps assertions before/after continue and break
            // in their lexical positions, including on non-yielding iterations.
            input = json!({"op":"guarded","input":input,"binding":&binding,"steps":steps});
        } else {
            for step in steps {
                input = json!({"op":"filter","input":input,"function":{"op":"lambda","parameters":[&binding],"body":&step["value"]}});
            }
        }
        let child = if next < clauses.len() {
            self.comprehension(a, next, &scope, b + 1)?
        } else {
            self.eval(&a["left"], &scope, b)?
        };
        if is_geometry(&child) {
            let id = self.geometry(child)?;
            let result =
                self.add(json!({"op":"collect","values":input,"binding":binding,"input":id}))?;
            let id = self.geometry(result.clone())?;
            self.collections.insert(id);
            return Ok(result);
        }
        Ok(V::Json(
            json!({"sequence":{"op":if next<clauses.len(){"flatmap"}else{"map"},"input":input,"function":{"op":"lambda","parameters":[&binding],"body":expression(child)?}}}),
        ))
    }
    fn operation(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let name = match name {
            "move" => "translate",
            "rect" => "rectangle",
            _ => name,
        };
        if [
            "surface_sweep",
            "surface_loft",
            "sdf_union",
            "sdf_intersection",
            "sdf_difference",
            "sdf_smooth_union",
            "mesh_union",
            "brep_union",
            "brep_subtract",
            "brep_intersection",
            "brep_xor",
            "mesh_intersection",
            "mesh_subtract",
            "union",
            "intersection",
            "subtract",
            "hull",
        ]
        .contains(&name)
        {
            let smooth = name == "sdf_smooth_union";
            let radius = if smooth {
                args.iter()
                    .position(|(n, _)| n.as_deref() == Some("radius"))
                    .map(|i| args.remove(i).1)
            } else {
                None
            };
            if args.iter().any(|(n, _)| n.is_some()) {
                return Err(crate::error(if name.starts_with("sdf_") {
                    "SDF operands are positional; smooth radius is named"
                } else {
                    "Boolean arguments are positional"
                }));
            }
            let mut inputs = Vec::new();
            if let Some(i) = input {
                inputs.push(i)
            }
            for (_, v) in args {
                inputs.push(self.geometry(v)?)
            }
            if name.starts_with("sdf_") && (inputs.len() != 2 || smooth && radius.is_none()) {
                return Err(crate::error(
                    "SDF operation requires two fields; smooth union also requires radius",
                ));
            }
            if name.starts_with("brep_") {
                if inputs.len() != 2 {
                    return Err(crate::error("B-rep Boolean requires exactly two bodies"));
                }
                return self.add(json!({"op":"brep_boolean","inputs":inputs,"operation":match name {"brep_union"=>"union","brep_intersection"=>"intersection","brep_xor"=>"xor",_=>"difference"}}));
            }
            if name.starts_with("mesh_") {
                if inputs.len() != 2 {
                    return Err(crate::error(
                        "Mesh Boolean operation requires exactly two meshes",
                    ));
                }
                return self.add(json!({"op":"mesh_boolean","inputs":inputs,"operation":match name{"mesh_union"=>"union","mesh_intersection"=>"intersection",_=>"difference"}}));
            }
            if inputs.is_empty() && ["union", "intersection", "subtract", "hull"].contains(&name) {
                return Err(crate::error("Boolean operation needs geometry"));
            }
            let mut node = if name == "subtract" {
                json!({"op":"difference","base":&inputs[0],"subtract":&inputs[1..]})
            } else {
                json!({"op":name,"inputs":inputs})
            };
            if let Some(v) = radius {
                node["radius"] = raw(v)?
            }
            return self.add(node);
        }
        if ["box", "translate"].contains(&name)
            && args.len() == 3
            && args.iter().all(|(name, _)| name.is_none())
        {
            let values = args
                .into_iter()
                .map(|(_, value)| scalar(value).map(V::Json))
                .collect::<R<Vec<_>>>()?;
            args = vec![(
                Some(if name == "box" { "size" } else { "vector" }.into()),
                V::Array(values),
            )];
        }
        if ["translate", "rotate", "scale"].contains(&name)
            && args
                .iter()
                .any(|(n, _)| matches!(n.as_deref(), Some("x" | "y" | "z")))
        {
            let mut names = Set::new();
            if args.iter().any(|(n, _)| {
                !matches!(n.as_deref(), Some("x" | "y" | "z")) || !names.insert(n.clone())
            }) {
                return Err(crate::error("Use unique x/y/z arguments or one vector"));
            }
            let vec = ["x", "y", "z"]
                .iter()
                .map(|axis| {
                    args.iter()
                        .find(|(n, _)| n.as_deref() == Some(axis))
                        .map_or(
                            V::Json(json!(if name == "scale" { 1 } else { 0 })),
                            |(_, v)| v.clone(),
                        )
                })
                .collect();
            args = vec![(Some("vector".into()), V::Array(vec))]
        }
        if name == "offset" && args.iter().any(|(n, _)| n.as_deref() == Some("delta")) {
            if input.is_none() || args.len() != 1 || args[0].0.as_deref() != Some("delta") {
                return Err(crate::error(
                    "offset(delta: distance) requires one argument and piped geometry",
                ));
            }
            return self.add(json!({"op":"offset","input":input,"distance":raw(args.remove(0).1)?,"mode":"delta"}));
        }
        let sig =
            signature(name).ok_or_else(|| crate::error(format!("Unknown operation {name}")))?;
        let mut node = json!({"op":name});
        for (i, (n, v)) in args.into_iter().enumerate() {
            let key = n
                .as_deref()
                .or_else(|| sig.get(i).copied())
                .ok_or_else(|| crate::error("Unknown or duplicate argument"))?;
            if !sig.contains(&key) || node.get(key).is_some() {
                return Err(crate::error("Unknown or duplicate argument"));
            }
            node[key] = if name == "brep_extrude_curves" && key == "loops" {
                let V::Array(loops) = v else {
                    return Err(crate::error("Curve extrusion requires nested curve lists"));
                };
                if loops.len() > 64 {
                    return Err(crate::error("Curve extrusion accepts at most 64 loops"));
                }
                let mut count = 0;
                J::Array(loops.into_iter().map(|wire| {
                    let V::Array(curves) = wire else {
                        return Err(crate::error("Curve extrusion requires nested curve lists"));
                    };
                    count += curves.len();
                    if curves.is_empty() || count > 254 {
                        return Err(crate::error("Curve extrusion requires nonempty loops and at most 254 curve references"));
                    }
                    curves.into_iter().map(|curve| {
                        if matches!(curve, V::Array(_)) {
                            return Err(crate::error("Each loop item must be one curve"));
                        }
                        self.geometry(curve).map(J::String)
                    }).collect::<R<Vec<_>>>().map(J::Array)
                }).collect::<R<_>>()?)
            } else if ["herringbone", "internal", "left_handed"].contains(&key) {
                // Text has no boolean literal: true/false lower to 1/0, and the
                // canonical schema wants a JSON boolean for these flags.
                match raw(v)? {
                    J::Bool(b) => J::Bool(b),
                    J::Number(n) => J::Bool(n.as_f64().is_some_and(|x| x != 0.)),
                    _ => return Err(crate::error(format!("{key} expects true or false"))),
                }
            } else {
                raw(v)?
            }
        }
        let modifier = [
            "polygon_extrude",
            "polygon_sweep",
            "mesh_to_nurbs_brep",
            "mesh_to_sdf",
            "mesh_to_subdivision",
            "subdivision_tessellate",
            "mesh_to_nurbs",
            "mesh_fit_nurbs",
            "nurbs_patches_tessellate",
            "sdf_tessellate",
            "sdf_offset",
            "sdf_translate",
            "brep_tessellate",
            "brep_extrude",
            "brep_revolve",
            "brep_chamfer",
            "brep_fillet",
            "surface_extrude",
            "surface_revolve",
            "tessellate",
            "thicken",
            "transform",
            "extrude",
            "revolve",
            "translate",
            "rotate",
            "scale",
            "offset",
            "mirror",
        ]
        .contains(&name);
        if modifier && input.is_none() {
            return Err(crate::error(format!("{name} requires piped geometry")));
        }
        if !modifier && input.is_some() {
            return Err(crate::error(format!(
                "{name} does not accept piped geometry"
            )));
        }
        if let Some(i) = input {
            let collection = self.collections.contains(&i);
            if collection && !["translate", "rotate", "scale", "mirror"].contains(&name) {
                return Err(crate::error(
                    "Transform each generated part inside the generator, or explicitly union the collection first",
                ));
            }
            node["input"] = json!(i);
            let result = self.add(node)?;
            if collection {
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
            }
            return Ok(result);
        }
        self.add(node)
    }
    fn validate(&self, t: &J, g: &Set<String>, visiting: &Set<String>) -> R<()> {
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
    fn infer_type(&self, v: &V) -> R<J> {
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
    fn check_type(&mut self, v: V, t: &J, bs: &Types, label: &str, level: usize) -> R<V> {
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
    fn type_fields(&self, t: &J) -> R<Types> {
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
    fn invoke(&mut self, f: &Function, call: &J, caller: &Env, b: usize) -> R<V> {
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
    fn protect(&mut self, v: V, checks: &[J]) -> R<V> {
        if checks.is_empty() {
            return Ok(v);
        }
        match v {
            V::Void(mut own) => {
                let mut all = checks.to_vec();
                all.append(&mut own);
                Ok(V::Void(all))
            }
            V::Array(a) if a.is_empty() => Ok(V::Json(
                json!({"sequence":{"op":"checked","checks":checks,"value":{"op":"list","items":[]}}}),
            )),
            V::Array(a) => a
                .into_iter()
                .map(|v| self.protect(v, checks))
                .collect::<R<Vec<_>>>()
                .map(V::Array),
            V::Record(r, t, b, bounds) => {
                let mut out = Map::new();
                for (k, v) in r {
                    out.insert(k, self.protect(v, checks)?);
                }
                Ok(V::Record(out, t, b, bounds))
            }
            V::Function(_) | V::Lambda(_) => Ok(v),
            V::Json(j) => {
                if j.get("geometry").is_some() {
                    let id = s(&j, "geometry");
                    let result=self.add(json!({"op":"if","condition":{"op":"checked","checks":checks,"value":1},"then":id,"else":id}))?;
                    if self.collections.contains(id) {
                        let g = self.geometry(result.clone())?;
                        self.collections.insert(g);
                    }
                    Ok(result)
                } else if j.get("text").is_some() {
                    Ok(V::Json(
                        json!({"op":"checked","checks":checks,"value":{"op":"text","value":&j["text"]}}),
                    ))
                } else if let Some(seq) = j.get("sequence") {
                    Ok(V::Json(
                        json!({"sequence":{"op":"checked","checks":checks,"value":seq}}),
                    ))
                } else {
                    Ok(V::Json(json!({"op":"checked","checks":checks,"value":j})))
                }
            }
        }
    }
    fn memo_value(&mut self, value: V) -> R<V> {
        match value {
            V::Array(items) => items
                .into_iter()
                .map(|v| self.memo_value(v))
                .collect::<R<Vec<_>>>()
                .map(V::Array),
            V::Record(fields, t, b, bounds) => {
                let mut out = Map::new();
                for (name, value) in fields {
                    out.insert(name, self.memo_value(value)?);
                }
                Ok(V::Record(out, t, b, bounds))
            }
            V::Json(j) if j.get("geometry").is_none() && has_assertion(&j) => {
                let id = self.check_id("m");
                if let Some(sequence) = j.get("sequence") {
                    Ok(V::Json(
                        json!({"sequence":{"op":"memo","id":id,"value":sequence}}),
                    ))
                } else {
                    Ok(V::Json(
                        json!({"op":"memo","id":id,"value":expression(V::Json(j))?}),
                    ))
                }
            }
            value => Ok(value),
        }
    }
    fn geometry_has_checks(&self, id: &str, visited: &mut Set<String>) -> bool {
        if !visited.insert(id.into()) {
            return false;
        }
        let Some(node) = self.nodes.iter().find(|node| s(node, "id") == id) else {
            return false;
        };
        if has_assertion(node) {
            return true;
        }
        let mut children = Vec::new();
        for key in ["input", "base", "then", "else"] {
            if let Some(id) = node[key].as_str() {
                children.push(id);
            }
        }
        for key in ["inputs", "subtract"] {
            children.extend(arr(node, key).iter().filter_map(J::as_str));
        }
        children.extend(
            arr(node, "arms")
                .iter()
                .filter_map(|arm| arm["input"].as_str()),
        );
        children
            .into_iter()
            .any(|id| self.geometry_has_checks(id, visited))
    }
    fn gather_effects(&self, value: &V, checks: &mut Vec<J>) {
        match value {
            V::Array(items) => items.iter().for_each(|v| self.gather_effects(v, checks)),
            V::Record(fields, _, _, _) => {
                fields.values().for_each(|v| self.gather_effects(v, checks))
            }
            V::Json(j) if j.get("geometry").is_some() => {
                let id = s(j, "geometry");
                if self.geometry_has_checks(id, &mut Set::new()) {
                    checks.push(json!({"op":"geometry_effects","input":id,"value":1}));
                }
            }
            V::Json(j) if has_assertion(j) => {
                checks.push(expression(value.clone()).expect("Scalar or sequence assertion value"));
            }
            value => gather(value, checks),
        }
    }
    fn needs_assertion(&self, value: &V) -> bool {
        match value {
            V::Array(items) => items.iter().any(|v| self.needs_assertion(v)),
            V::Record(fields, _, _, _) => fields.values().any(|v| self.needs_assertion(v)),
            V::Json(j) if j.get("geometry").is_some() => {
                self.geometry_has_checks(s(j, "geometry"), &mut Set::new())
            }
            V::Json(j) => has_assertion(j),
            V::Void(checks) => checks.iter().any(has_assertion),
            _ => false,
        }
    }
    fn statement_checks(&mut self, value: &V) {
        let mut checks = Vec::new();
        self.gather_effects(value, &mut checks);
        if !checks.is_empty() {
            let id = self.check_id("v");
            self.constraints.push(json!({"id":id,"left":{"op":"checked","checks":checks,"value":1},"relation":"eq","right":1,"message":"Statement checks"}));
        }
    }
    fn check(&mut self, kind: &str, a: &J, e: &Env) -> R<()> {
        let mut chain = Vec::new();
        let mut subject = a;
        let check_names = [
            "message",
            "between",
            "greaterThan",
            "atLeast",
            "lessThan",
            "atMost",
            "equalTo",
            "approximately",
            "hasBodies",
            "isWatertight",
            "hasNoDegenerateTriangles",
        ];
        while s(subject, "kind") == "pipe" && check_names.contains(&s(&subject["right"], "value")) {
            chain.push(&subject["right"]);
            subject = &subject["left"]
        }
        chain.reverse();
        let mut message = format!("{kind} check failed");
        if let Some(tail) = chain.last().filter(|tail| s(tail, "value") == "message") {
            let args = arr(tail, "args");
            if args.len() != 1
                || args[0].get("name").is_some()
                || s(&args[0]["value"], "kind") != "string"
            {
                return Err(crate::error("message requires one string"));
            }
            message = s(&args[0]["value"], "value").into();
            chain.pop();
        }
        if chain.is_empty() {
            let value = scalar(self.eval(subject, e, 0)?)?;
            // The same dimensionless truthiness rules as if/where, not a numeric == 1.
            let id = self.check_id("v");
            self.constraints.push(json!({"id":id,"left":{"op":"not","value":{"op":"not","value":value}},"relation":"eq","right":1,"message":message}));
            return Ok(());
        }
        let measurement = s(subject, "kind") == "member"
            && s(&subject["left"], "kind") == "call"
            && s(&subject["left"], "value") == "measure";
        let mut target = None;
        let mut left = J::Null;
        if measurement {
            let args = arr(&subject["left"], "args");
            if kind != "assert" || args.len() != 1 || args[0].get("name").is_some() {
                return Err(crate::error(
                    "assert measure(root).height/width/depth expected",
                ));
            }
            let v = self.eval(&args[0]["value"], e, 0)?;
            target = Some(self.geometry(v)?)
        } else {
            let v = self.eval(subject, e, 0)?;
            if matches!(&v,V::Json(j) if j.get("geometry").is_some()) {
                if kind != "assert" {
                    return Err(crate::error("Use assert for geometry"));
                }
                target = Some(self.geometry(v)?)
            } else {
                left = scalar(v)?
            }
        }
        for c in chain {
            let args = arr(c, "args");
            let values = args
                .iter()
                .map(|a| self.eval(&a["value"], e, 0).and_then(scalar))
                .collect::<R<Vec<_>>>()?;
            let rule = s(c, "value");
            let named = args.iter().any(|a| a.get("name").is_some());
            if let Some(target) = &target {
                if measurement {
                    if !["height", "width", "depth"].contains(&s(subject, "value"))
                        || rule != "approximately"
                        || args.is_empty()
                        || args.len() > 2
                        || args[0].get("name").is_some()
                        || (args.len() == 2 && s(&args[1], "name") != "tolerance")
                    {
                        return Err(crate::error(
                            "Measurement requires approximately(value, tolerance: value)",
                        ));
                    }
                } else if !["hasBodies", "isWatertight", "hasNoDegenerateTriangles"].contains(&rule)
                    || named
                    || args.len() != if rule == "hasBodies" { 1 } else { 0 }
                {
                    return Err(crate::error("Unknown geometry check or invalid arguments"));
                }
                let id = self.check_id("g");
                let mut check = json!({"id":id,"target":target,"check":if measurement{s(subject,"value")}else{rule},"message":&message});
                if let Some(v) = values.first() {
                    check["expected"] = v.clone()
                }
                if let Some(v) = values.get(1) {
                    check["tolerance"] = v.clone()
                }
                self.checks.push(check)
            } else if rule == "between" {
                if values.len() != 2 || named {
                    return Err(crate::error("between(min,max) expected"));
                }
                for (v, relation) in values.into_iter().zip(["ge", "le"]) {
                    let id = self.check_id("v");
                    self.constraints.push(json!({"id":id,"left":&left,"relation":relation,"right":v,"message":&message}))
                }
            } else {
                let relation = match rule {
                    "greaterThan" => "gt",
                    "atLeast" => "ge",
                    "lessThan" => "lt",
                    "atMost" => "le",
                    "equalTo" | "approximately" => "eq",
                    _ => "",
                };
                if relation.is_empty()
                    || values.is_empty()
                    || values.len() > if rule == "approximately" { 2 } else { 1 }
                    || args[0].get("name").is_some()
                    || (args.len() == 2 && s(&args[1], "name") != "tolerance")
                {
                    return Err(crate::error("Unknown scalar check or invalid arguments"));
                }
                let id = self.check_id("v");
                let mut check = json!({"id":id,"left":&left,"relation":relation,"right":&values[0],"message":&message});
                if let Some(v) = values.get(1) {
                    check["tolerance"] = v.clone()
                }
                self.constraints.push(check)
            }
        }
        Ok(())
    }
    fn local_check(&mut self, kind: &str, value: &J, scope: &Env) -> R<J> {
        let constraints = self.constraints.len();
        let geometry = self.checks.len();
        self.check(kind, value, scope)?;
        let check = json!({"op":"assert_value","constraints":self.constraints.split_off(constraints),"geometry_assertions":self.checks.split_off(geometry),"value":1});
        Ok(json!({"op":"memo","id":self.check_id("m"),"value":check}))
    }
    fn check_id(&mut self, prefix: &str) -> String {
        self.check_serial += 1;
        format!("{prefix}{}", self.check_serial)
    }
}
fn has_assertion(value: &J) -> bool {
    match value {
        J::Object(fields) => {
            ["assert_value", "geometry_effects"].contains(&s(value, "op"))
                || fields.values().any(has_assertion)
        }
        J::Array(items) => items.iter().any(has_assertion),
        _ => false,
    }
}
fn gather(v: &V, out: &mut Vec<J>) {
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
fn infer(expected: &J, actual: &J, g: &Set<String>, bs: &mut Types, explicit: bool) -> R<()> {
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
fn binding_names(pattern: &J) -> Vec<&str> {
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
    fn bind(&mut self, p: &J, v: V, e: &mut Env) -> R<Vec<J>> {
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
pub fn compile(statements: Vec<Statement>) -> R<J> {
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
        check_serial: 0,
        expansions: 0,
    };
    let mut e = Env::new();
    let mut root = String::new();
    let mut segments = None;
    for statement in statements {
        let a = statement.node;
        let result: R<()> = try {
            let name = s(&a, "name");
            match s(&a, "kind") {
                "bind" => {
                    if e.contains_key(name) {
                        do yeet crate::error(format!("Duplicate name {name}"));
                    }
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
            .map_err(|e| crate::error(format!("ModelGraph Text line {}: {}", statement.line, e)))?;
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
fn signature(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "polygon" => &["points"],
        "polygon_profile" => &["outer", "holes"],
        "polygon_extrude" => &["vector"],
        "polygon_sweep" => &["path", "up"],
        "polygon_loft" => &["sections"],
        "triangle_mesh" => &["vertices", "triangles"],
        "mesh_to_nurbs_brep" => &[],
        "mesh_to_sdf" => &[],
        "mesh_to_subdivision" => &["iterations"],
        "subdivision_tessellate" => &["levels"],
        "mesh_to_nurbs" => &[],
        "mesh_fit_nurbs" => &["max_deviation"],
        "nurbs_patches_tessellate" => &["segments"],
        "subdivision" => &["vertices", "faces", "levels"],
        "sdf_sphere" => &["center", "radius"],
        "sdf_box" => &["center", "half_size"],
        "sdf_torus" => &["center", "major_radius", "minor_radius"],
        "sdf_tessellate" => &["min", "max", "cells"],
        "sdf_offset" => &["distance"],
        "sdf_translate" => &["vector"],
        "brep_box" => &["min", "max"],
        "brep_sphere" => &["radius"],
        "brep_torus" => &["major_radius", "minor_radius"],
        "brep_cylinder" => &["radius", "height"],
        "brep_frustum" => &["bottom_radius", "top_radius", "height"],
        "brep_tube" => &["outer_radius", "inner_radius", "height"],
        "brep_gear" => &[
            "module",
            "teeth",
            "height",
            "pressure_angle",
            "helix_angle",
            "herringbone",
            "bore",
            "internal",
            "rim_width",
            "clearance",
            "backlash",
        ],
        "brep_revolve" => &["angle"],
        "brep_extrude" => &["height"],
        "brep_extrude_curves" => &["loops", "z_min", "z_max"],
        "brep_chamfer" => &["edges", "size"],
        "brep_fillet" => &["edges", "radius", "segments"],
        "brep_tessellate" => &["segments"],
        "nurbs_surface" => &[
            "degree_u",
            "degree_v",
            "knots_u",
            "knots_v",
            "control_points",
            "weights",
        ],
        "nurbs_curve" => &["degree", "knots", "control_points", "weights"],
        "surface_extrude" => &["vector"],
        "surface_revolve" => &["origin", "axis", "angle"],
        "tessellate" => &["segments_u", "segments_v"],
        "thicken" => &["vector"],
        "transform" => &["matrix"],
        "gear" => &[
            "teeth",
            "module",
            "thickness",
            "pressure_angle",
            "bore",
            "backlash",
            "clearance",
            "internal",
            "rim_width",
            "flank_segments",
            "helix_angle",
            "herringbone",
        ],
        "planetary_gears" => &[
            "sun_teeth",
            "planet_teeth",
            "planet_count",
            "module",
            "thickness",
            "pressure_angle",
            "bore",
            "backlash",
            "clearance",
            "rim_width",
            "flank_segments",
            "carrier_angle",
            "helix_angle",
            "herringbone",
        ],
        "planetary_spinner" => &[
            "inner_radius",
            "outer_radius",
            "bore",
            "gap",
            "height",
            "helix_angle",
        ],
        "circle" => &["radius"],
        "rectangle" => &["size"],
        "box" => &["size"],
        "sphere" => &["radius"],
        "cylinder" => &["radius", "height"],
        "extrude" => &["height"],
        "revolve" => &["angle"],
        "translate" => &["vector"],
        "rotate" => &["vector"],
        "scale" => &["vector"],
        "offset" => &["distance"],
        "mirror" => &["normal"],
        _ => return None,
    })
}

include!("lower_query.rs");

include!("lower_match.rs");

include!("lower_traits.rs");
