use crate::parser::{number, Statement};
use crate::value::json;
use indexmap::IndexMap as Map;
use serde_json::Value as J;
use std::{collections::BTreeSet as Set, rc::Rc};
type R<T> = Result<T, String>;
type Env = Map<String, V>;
type Types = Map<String, J>;
#[derive(Clone)]
enum V {
    Json(J),
    Array(Vec<V>),
    Record(Map<String, V>, Option<J>, bool),
    Function(Rc<Function>),
    Lambda(Rc<Lambda>),
}
struct Function {
    ast: J,
    env: Env,
    types: Types,
}
struct Lambda {
    name: String,
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
        return Err("Type substitution depth exceeds 16".into());
    }
    if let Some(x) = bs.get(s(t, "name")) {
        return Ok(x.clone());
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
        _ => Err("Expected scalar".into()),
    }
}
fn expression(v: V) -> R<J> {
    match v {
        V::Array(a) => {
            Ok(json!({"op":"list","items":a.into_iter().map(expression).collect::<R<Vec<_>>>()?}))
        }
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        v => scalar(v),
    }
}
fn list(v: V) -> R<J> {
    match &v {
        V::Array(_) => expression(v),
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        _ => Err("Expected a sequence".into()),
    }
}
fn raw(v: V) -> R<J> {
    match v {
        V::Json(j) => Ok(j),
        V::Array(a) => a.into_iter().map(raw).collect::<R<Vec<_>>>().map(J::Array),
        _ => Err("Expected numeric geometry argument".into()),
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
    active: Types,
    serial: usize,
    expansions: usize,
}
impl Compiler {
    fn add(&mut self, mut node: J) -> R<V> {
        if self.nodes.len() >= 128 {
            return Err("At most 128 geometry nodes".into());
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
            _ => Err("Expected geometry".into()),
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
            return Err("Function expansion exceeds 64".into());
        }
        let b = b + 1;
        let name = s(a, "value");
        match s(a, "kind") {
            "string" => return Ok(V::Json(json!({"text":name}))),
            "function" => {
                let generic = self
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
                for f in arr(a, "inputs").iter().chain(arr(a, "outputs")) {
                    self.validate(&f["type"], &generic, &Set::new())?
                }
                return Ok(V::Function(Rc::new(Function {
                    ast: a.clone(),
                    env: e.clone(),
                    types: self.active.clone(),
                })));
            }
            "record" => {
                let mut r = Map::new();
                for arg in arr(a, "args") {
                    r.insert(s(arg, "name").into(), self.eval(&arg["value"], e, b)?);
                }
                let v = V::Record(r, None, false);
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
                    V::Record(mut r, _, _) => r
                        .shift_remove(name)
                        .ok_or_else(|| format!("Unknown record field {name}")),
                    _ => Err(format!("Unknown record field {name}")),
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
            "name" => {
                return e
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Unknown name {name}"))
            }
            "array" => {
                return arr(a, "items")
                    .iter()
                    .map(|a| self.eval(a, e, b))
                    .collect::<R<Vec<_>>>()
                    .map(V::Array)
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
            "conditional" => {
                let condition = scalar(self.eval(&a["left"], e, b)?)?;
                let yes = self.eval(&a["items"][0], e, b)?;
                let no = self.eval(&a["items"][1], e, b)?;
                if is_geometry(&yes) || is_geometry(&no) {
                    if !is_geometry(&yes) || !is_geometry(&no) {
                        return Err(
                            "Conditional branches must both be geometry or both be values".into(),
                        );
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
                if let (V::Array(y), V::Array(n)) = (&yes, &no) {
                    if y.len() == n.len() {
                        return y.iter().cloned().zip(n.iter().cloned()).map(|(y,n)|Ok(V::Json(json!({"op":"if","condition":&condition,"then":expression(y)?,"else":expression(n)?})))).collect::<R<Vec<_>>>().map(V::Array);
                    }
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
                    name: name.into(),
                    body: a["left"].clone(),
                    env: e.clone(),
                })))
            }
            "neg" => {
                return Ok(V::Json(
                    json!({"op":"negate","value":scalar(self.eval(&a["left"],e,b)?)?}),
                ))
            }
            "binary" => {
                let mut left = scalar(self.eval(&a["left"], e, b)?)?;
                let mut right = scalar(self.eval(&a["right"], e, b)?)?;
                if name == ">" || name == ">=" {
                    std::mem::swap(&mut left, &mut right)
                }
                let op = match name {
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
            let v = self.eval(&a["left"], e, b)?;
            (&a["right"], Some(self.geometry(v)?))
        } else {
            (a, None)
        };
        if s(call, "kind") != "call" {
            return Err("Expected a call".into());
        }
        let name = s(call, "value");
        let args = arr(call, "args");
        if ["zip", "enumerate", "length", "at"].contains(&name) {
            if input.is_some() || args.iter().any(|a| a.get("name").is_some()) {
                return Err("Sequence functions require positional arguments".into());
            }
            let mut values = args
                .iter()
                .map(|a| self.eval(&a["value"], e, b))
                .collect::<R<Vec<_>>>()?;
            if name == "zip" {
                if !(2..=8).contains(&values.len()) {
                    return Err("zip expects 2..8 sequences".into());
                }
                return Ok(V::Json(
                    json!({"sequence":{"op":"zip","inputs":values.into_iter().map(list).collect::<R<Vec<_>>>()?}}),
                ));
            }
            if values.len() != if name == "at" { 2 } else { 1 } {
                return Err("Invalid sequence function arity".into());
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
                return Err("repeat(count, i => geometry) expected".into());
            }
            let count = scalar(self.eval(&args[0]["value"], e, b)?)?;
            self.mark(&count);
            let f = self.eval(&args[1]["value"], e, b)?;
            if let V::Lambda(f) = f {
                self.serial += 1;
                let index = format!("i{}", self.serial);
                let mut scope = f.env.clone();
                scope.insert(f.name.clone(), V::Json(json!({"local":&index})));
                let v = self.eval(&f.body, &scope, b)?;
                let id = self.geometry(v)?;
                return self.add(json!({"op":"map","count":count,"index":index,"input":id}));
            }
            return Err("repeat requires a lambda".into());
        }
        if let Some(f) = e.get(name) {
            match f {
                V::Lambda(f) => {
                    if input.is_some() || args.len() != 1 || args[0].get("name").is_some() {
                        return Err("Unary function expects one positional argument".into());
                    }
                    let mut scope = f.env.clone();
                    scope.insert(f.name.clone(), self.eval(&args[0]["value"], e, b)?);
                    return self.eval(&f.body, &scope, b);
                }
                V::Function(f) => {
                    if input.is_some() {
                        return Err(
                            "Block functions are called directly, not through geometry pipelines"
                                .into(),
                        );
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
            return Err("Function expansion exceeds 64".into());
        }
        let clauses = arr(a, "items");
        let clause = &clauses[offset];
        if s(clause, "kind") != "for" {
            return Err("Generator must start with for".into());
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
        let mut condition: Option<J> = None;
        while next < clauses.len() && s(&clauses[next], "kind") != "for" {
            let c = &clauses[next];
            next += 1;
            if s(c, "kind") == "let" {
                let v = self.eval(&c["left"], &scope, b)?;
                scope.insert(s(&c["items"][0], "value").into(), v);
            } else {
                let v = scalar(self.eval(&c["left"], &scope, b)?)?;
                condition = Some(if let Some(old) = condition {
                    json!({"op":"and","args":[old,v]})
                } else {
                    v
                })
            }
        }
        if let Some(c) = condition {
            input = json!({"op":"filter","input":input,"function":{"op":"lambda","parameters":[&binding],"body":c}})
        }
        let child = if next < clauses.len() {
            self.comprehension(a, next, &scope, b + 1)?
        } else {
            self.eval(&a["left"], &scope, b)?
        };
        if let V::Json(j) = &child {
            if let Some(id) = j.get("geometry") {
                let result =
                    self.add(json!({"op":"collect","values":input,"binding":binding,"input":id}))?;
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
                return Ok(result);
            }
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
        if [
            "surface_sweep",
            "surface_loft",
            "sdf_union",
            "sdf_intersection",
            "sdf_difference",
            "sdf_smooth_union",
            "mesh_union",
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
                return Err(if name.starts_with("sdf_") {
                    "SDF operands are positional; smooth radius is named"
                } else {
                    "Boolean arguments are positional"
                }
                .into());
            }
            let mut inputs = Vec::new();
            if let Some(i) = input {
                inputs.push(i)
            }
            for (_, v) in args {
                inputs.push(self.geometry(v)?)
            }
            if name.starts_with("sdf_") && (inputs.len() != 2 || smooth && radius.is_none()) {
                return Err(
                    "SDF operation requires two fields; smooth union also requires radius".into(),
                );
            }
            if name.starts_with("mesh_") {
                if inputs.len() != 2 {
                    return Err("Mesh Boolean operation requires exactly two meshes".into());
                }
                return self.add(json!({"op":"mesh_boolean","inputs":inputs,"operation":match name{"mesh_union"=>"union","mesh_intersection"=>"intersection",_=>"difference"}}));
            }
            if inputs.is_empty() && ["union", "intersection", "subtract", "hull"].contains(&name) {
                return Err("Boolean operation needs geometry".into());
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
        if ["translate", "rotate", "scale"].contains(&name)
            && args
                .iter()
                .any(|(n, _)| matches!(n.as_deref(), Some("x" | "y" | "z")))
        {
            let mut names = Set::new();
            if args.iter().any(|(n, _)| {
                !matches!(n.as_deref(), Some("x" | "y" | "z")) || !names.insert(n.clone())
            }) {
                return Err("Use unique x/y/z arguments or one vector".into());
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
                return Err(
                    "offset(delta: distance) requires one argument and piped geometry".into(),
                );
            }
            return self.add(json!({"op":"offset","input":input,"distance":raw(args.remove(0).1)?,"mode":"delta"}));
        }
        let sig = signature(name).ok_or_else(|| format!("Unknown operation {name}"))?;
        let mut node = json!({"op":name});
        for (i, (n, v)) in args.into_iter().enumerate() {
            let key = n
                .as_deref()
                .or_else(|| sig.get(i).copied())
                .ok_or("Unknown or duplicate argument")?;
            if !sig.contains(&key) || node.get(key).is_some() {
                return Err("Unknown or duplicate argument".into());
            }
            node[key] = raw(v)?
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
            return Err(format!("{name} requires piped geometry"));
        }
        if !modifier && input.is_some() {
            return Err(format!("{name} does not accept piped geometry"));
        }
        if let Some(i) = input {
            if self.collections.contains(&i) {
                return Err("Transform each generated part inside the generator, or explicitly union the collection first".into());
            }
            node["input"] = json!(i)
        }
        self.add(node)
    }
    fn validate(&self, t: &J, g: &Set<String>, visiting: &Set<String>) -> R<()> {
        let name = s(t, "name");
        let args = arr(t, "args");
        if g.contains(name) {
            if !args.is_empty() {
                return Err("Generic parameter cannot have type arguments".into());
            }
            return Ok(());
        }
        if ["int", "f32", "f64", "str", "length", "angle", "Geometry"].contains(&name) {
            if !args.is_empty() {
                return Err(format!("Type {name} takes no arguments"));
            }
            return Ok(());
        }
        if name == "Vec" {
            if args.len() != 1 {
                return Err("Vec requires one element type".into());
            }
            return self.validate(&args[0], g, visiting);
        }
        let def = self
            .structs
            .get(name)
            .ok_or_else(|| format!("Unknown type {name}"))?;
        if visiting.contains(name) {
            return Err("Recursive value structures are not supported".into());
        }
        if args.len() != arr(def, "generics").len() {
            return Err(format!("Wrong type argument count for {name}"));
        }
        for a in args {
            self.validate(a, g, visiting)?
        }
        Ok(())
    }
    fn infer_type(&self, v: &V) -> R<J> {
        match v {
            V::Record(_, t, _) => {
                return t
                    .clone()
                    .ok_or("Cannot infer a generic type from an anonymous record".into())
            }
            V::Array(a) => {
                let first = a.first().ok_or(
                    "Cannot infer element type of an empty vector; specify generic arguments",
                )?;
                let element = self.infer_type(first)?;
                for v in a {
                    if typename(&self.infer_type(v)?) != typename(&element) {
                        return Err("Generic vector requires one element type".into());
                    }
                }
                return Ok(ty("Vec", vec![element]));
            }
            V::Json(j) => {
                let name = if j.get("text").is_some() {
                    "str"
                } else if j.get("geometry").is_some() {
                    "Geometry"
                } else if s(j, "op") == "checked" {
                    return self.infer_type(&V::Json(j["value"].clone()));
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
                        .ok_or("Unknown parameter")?;
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
            return Err("Record type nesting exceeds 16".into());
        }
        let t = substitute(t, bs, 0)?;
        let name = s(&t, "name");
        if name == "str" {
            if !matches!(&v,V::Json(j) if j.get("text").is_some()) {
                return Err(format!("{label}: expected str"));
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
                    return Err(format!("{label}: expected int"));
                }
                if name == "length" || name == "angle" {
                    return Err(format!("{label}: expected {name} units"));
                }
            }
            if s(&v, "op") == "quantity" && name != "length" && name != "angle" {
                return Err(format!("{label}: expected dimensionless {name}"));
            }
            return Ok(V::Json(json!({"op":"typed","type":name,"value":v})));
        }
        if name == "Vec" {
            if let V::Array(a) = v {
                if arr(&t, "args").len() == 1 {
                    return a
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| {
                            self.check_type(
                                v,
                                &t["args"][0],
                                bs,
                                &format!("{label}[{i}]"),
                                level + 1,
                            )
                        })
                        .collect::<R<Vec<_>>>()
                        .map(V::Array);
                }
            }
            return Err(format!("{label}: expected {} literal vector", typename(&t)));
        }
        let def = self
            .structs
            .get(name)
            .cloned()
            .ok_or_else(|| format!("{label}: unknown or incomplete type {}", typename(&t)))?;
        if arr(&t, "args").len() != arr(&def, "generics").len() {
            return Err(format!(
                "{label}: unknown or incomplete type {}",
                typename(&t)
            ));
        }
        let V::Record(mut r, old, _) = v else {
            return Err(format!("{label}: expected {} record", typename(&t)));
        };
        if let Some(old) = old {
            if typename(&old) != typename(&t) {
                return Err(format!(
                    "{label}: expected {}, got {}",
                    typename(&t),
                    typename(&old)
                ));
            }
        }
        if r.len() != arr(&def, "fields").len()
            || arr(&def, "fields")
                .iter()
                .any(|f| !r.contains_key(s(f, "name")))
        {
            return Err(format!("{label}: fields must exactly match {name}"));
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
        Ok(V::Record(out, Some(t), false))
    }
    fn invoke(&mut self, f: &Function, call: &J, caller: &Env, b: usize) -> R<V> {
        if b > 64 {
            return Err("Function expansion exceeds 64".into());
        }
        self.expansions += 1;
        if self.expansions > 256 {
            return Err("Block function call expansion exceeds 256".into());
        }
        let def = &f.ast;
        let mut bs = f.types.clone();
        let generic: Set<String> = arr(def, "generics")
            .iter()
            .map(|g| g.as_str().unwrap().into())
            .collect();
        let allowed = bs.keys().cloned().chain(generic.iter().cloned()).collect();
        for field in arr(def, "inputs").iter().chain(arr(def, "outputs")) {
            self.validate(&field["type"], &allowed, &Set::new())?
        }
        let explicit = call.get("types").is_some();
        if explicit {
            if arr(call, "types").len() != arr(def, "generics").len() {
                return Err("Wrong generic argument count".into());
            }
            for (t, g) in arr(call, "types").iter().zip(arr(def, "generics")) {
                let resolved = substitute(t, &self.active, 0)?;
                self.validate(&resolved, &Set::new(), &Set::new())?;
                bs.insert(g.as_str().unwrap().into(), resolved);
            }
        }
        let args = arr(call, "args");
        let inputs = arr(def, "inputs");
        if args.len() != inputs.len() {
            return Err("Function argument count mismatch".into());
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
            return Err("Named arguments must exactly match parameters".into());
        }
        let mut values = Vec::new();
        for (i, field) in inputs.iter().enumerate() {
            let arg = if named {
                args.iter()
                    .find(|a| s(a, "name") == s(field, "name"))
                    .unwrap()
            } else {
                &args[i]
            };
            values.push(self.eval(&arg["value"], caller, b + 1)?)
        }
        if !generic.is_empty() {
            for (field, value) in inputs.iter().zip(&values) {
                infer(
                    &field["type"],
                    &self.infer_type(value)?,
                    &generic,
                    &mut bs,
                    explicit,
                )?
            }
        }
        for g in &generic {
            if !bs.contains_key(g) {
                return Err(format!("Cannot infer {g}; provide explicit type arguments"));
            }
        }
        let mut scope = f.env.clone();
        for (field, value) in inputs.iter().zip(values) {
            let name = s(field, "name");
            scope.insert(
                name.into(),
                self.check_type(value, &field["type"], &bs, &format!("argument {name}"), 0)?,
            );
        }
        let previous = std::mem::replace(&mut self.active, bs.clone());
        let result = (|| {
            let mut locals: Set<String> = inputs.iter().map(|f| s(f, "name").into()).collect();
            for st in arr(def, "items") {
                if s(st, "kind") == "statement" {
                    self.eval(&st["left"], &scope, b + 1)?;
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
                        return Err(format!("Duplicate local {name}"));
                    }
                    scope.shift_remove(name);
                }
                let v = self.eval(&st["right"], &scope, b + 1)?;
                bind(pattern, v, &mut scope)?;
            }
            let result = self.eval(&def["left"], &scope, b + 1)?;
            let output = if flag(def, "singleResult") {
                self.check_type(result, &def["outputs"][0]["type"], &bs, "result", 0)?
            } else {
                let V::Record(mut r, _, _) = result else {
                    return Err("ret must return named fields".into());
                };
                if r.len() != arr(def, "outputs").len()
                    || arr(def, "outputs")
                        .iter()
                        .any(|f| !r.contains_key(s(f, "name")))
                {
                    return Err("ret fields must exactly match named results".into());
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
                V::Record(out, None, false)
            };
            let mut checks = Vec::new();
            for field in inputs {
                gather(scope.get(s(field, "name")).unwrap(), &mut checks)
            }
            gather(&output, &mut checks);
            if checks.len() > 256 {
                return Err("Function type checks exceed 256".into());
            }
            if flag(def, "voidResult") {
                if !checks.is_empty() {
                    self.constraints.push(json!({"id":format!("v{}",self.constraints.len()+1),"left":{"op":"checked","checks":checks,"value":1},"relation":"eq","right":1,"message":"Function argument type check"}))
                }
                return Ok(V::Record(Map::new(), None, true));
            }
            if checks.is_empty() {
                Ok(output)
            } else {
                self.protect(output, &checks)
            }
        })();
        self.active = previous;
        result
    }
    fn protect(&mut self, v: V, checks: &[J]) -> R<V> {
        match v {
            V::Array(a) => a
                .into_iter()
                .map(|v| self.protect(v, checks))
                .collect::<R<Vec<_>>>()
                .map(V::Array),
            V::Record(r, t, b) => {
                let mut out = Map::new();
                for (k, v) in r {
                    out.insert(k, self.protect(v, checks)?);
                }
                Ok(V::Record(out, t, b))
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
                    Ok(V::Json(j))
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
        if chain.is_empty() {
            return Err("Check requires .check(...) methods".into());
        }
        let mut message = format!("{kind} check failed");
        let tail = chain.last().unwrap();
        if s(tail, "value") == "message" {
            let args = arr(tail, "args");
            if args.len() != 1
                || args[0].get("name").is_some()
                || s(&args[0]["value"], "kind") != "string"
            {
                return Err("message requires one string".into());
            }
            message = s(&args[0]["value"], "value").into();
            chain.pop();
        }
        if chain.is_empty() {
            return Err("At least one check is required".into());
        }
        let measurement = s(subject, "kind") == "member"
            && s(&subject["left"], "kind") == "call"
            && s(&subject["left"], "value") == "measure";
        let mut target = None;
        let mut left = J::Null;
        if measurement {
            let args = arr(&subject["left"], "args");
            if kind != "assert" || args.len() != 1 || args[0].get("name").is_some() {
                return Err("assert measure(root).height/width/depth expected".into());
            }
            let v = self.eval(&args[0]["value"], e, 0)?;
            target = Some(self.geometry(v)?)
        } else {
            let v = self.eval(subject, e, 0)?;
            if matches!(&v,V::Json(j) if j.get("geometry").is_some()) {
                if kind != "assert" {
                    return Err("Use assert for geometry".into());
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
                        return Err(
                            "Measurement requires approximately(value, tolerance: value)".into(),
                        );
                    }
                } else if !["hasBodies", "isWatertight", "hasNoDegenerateTriangles"].contains(&rule)
                    || named
                    || args.len() != if rule == "hasBodies" { 1 } else { 0 }
                {
                    return Err("Unknown geometry check or invalid arguments".into());
                }
                let mut check = json!({"id":format!("g{}",self.checks.len()+1),"target":target,"check":if measurement{s(subject,"value")}else{rule},"message":&message});
                if let Some(v) = values.first() {
                    check["expected"] = v.clone()
                }
                if let Some(v) = values.get(1) {
                    check["tolerance"] = v.clone()
                }
                self.checks.push(check)
            } else if rule == "between" {
                if values.len() != 2 || named {
                    return Err("between(min,max) expected".into());
                }
                for (v, relation) in values.into_iter().zip(["ge", "le"]) {
                    self.constraints.push(json!({"id":format!("v{}",self.constraints.len()+1),"left":&left,"relation":relation,"right":v,"message":&message}))
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
                    return Err("Unknown scalar check or invalid arguments".into());
                }
                let mut check = json!({"id":format!("v{}",self.constraints.len()+1),"left":&left,"relation":relation,"right":&values[0],"message":&message});
                if let Some(v) = values.get(1) {
                    check["tolerance"] = v.clone()
                }
                self.constraints.push(check)
            }
        }
        Ok(())
    }
}
fn gather(v: &V, out: &mut Vec<J>) {
    match v {
        V::Array(a) => a.iter().for_each(|v| gather(v, out)),
        V::Record(r, _, _) => r.values().for_each(|v| gather(v, out)),
        V::Json(j) if ["typed", "checked"].contains(&s(j, "op")) => out.push(j.clone()),
        _ => {}
    }
}
fn infer(expected: &J, actual: &J, g: &Set<String>, bs: &mut Types, explicit: bool) -> R<()> {
    let name = s(expected, "name");
    if g.contains(name) {
        if let Some(old) = bs.get(name) {
            if !explicit && typename(old) != typename(actual) {
                return Err(format!(
                    "Conflicting inference for {name}: {} and {}",
                    typename(old),
                    typename(actual)
                ));
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
fn bind(p: &J, v: V, e: &mut Env) -> R<()> {
    if matches!(&v, V::Record(_, _, true)) {
        return Err("Function has no return value".into());
    }
    if s(p, "kind") == "name" {
        let name = s(p, "value");
        if e.contains_key(name) {
            return Err(format!("Duplicate name {name}"));
        }
        e.insert(name.into(), v);
        return Ok(());
    }
    let V::Record(r, _, _) = v else {
        return Err("Destructuring requires a record".into());
    };
    for item in arr(p, "items") {
        let name = s(item, "value");
        if !r.contains_key(name) {
            return Err(format!("Unknown result field {name}"));
        }
        if e.contains_key(name) {
            return Err(format!("Duplicate name {name}"));
        }
    }
    for item in arr(p, "items") {
        let name = s(item, "value");
        e.insert(name.into(), r[name].clone());
    }
    Ok(())
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
        active: Types::new(),
        serial: 0,
        expansions: 0,
    };
    let mut e = Env::new();
    let mut root = String::new();
    let mut segments = None;
    for statement in statements {
        let a = statement.node;
        let result = (|| {
            let name = s(&a, "name");
            match s(&a, "kind") {
                "bind" => {
                    if e.contains_key(name) {
                        return Err(format!("Duplicate name {name}"));
                    }
                    let v = c.eval(&a["value"], &e, 0)?;
                    if matches!(&v, V::Record(_, _, true)) {
                        return Err("Function has no return value".into());
                    }
                    if let V::Json(j) = &v {
                        if j.get("geometry").is_some() {
                            root = s(j, "geometry").into()
                        }
                    }
                    e.insert(name.into(), v);
                }
                "struct" => {
                    if c.structs.contains_key(name)
                        || [
                            "int", "f32", "f64", "str", "length", "angle", "Geometry", "Vec",
                        ]
                        .contains(&name)
                    {
                        return Err(format!("Duplicate or reserved type {name}"));
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
                    bind(&a["pattern"], v, &mut e)?
                }
                "validate" | "assert" => c.check(s(&a, "kind"), &a["value"], &e)?,
                "segments" => {
                    if segments.is_some() {
                        return Err("Duplicate segments declaration".into());
                    }
                    segments = Some(a["value"].clone())
                }
                "param" => {
                    if e.contains_key(name) {
                        return Err(format!("Duplicate name {name}"));
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
                    c.eval(&a["value"], &e, 0)?;
                }
                _ => return Err("Unknown statement".into()),
            }
            Ok(())
        })();
        result.map_err(|m: String| format!("ModelGraph Text line {}: {m}", statement.line))?;
    }
    if root.is_empty() {
        return Err("No geometry to show".into());
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
