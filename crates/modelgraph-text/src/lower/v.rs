use super::*;

#[derive(Clone)]
pub(super) enum V {
    Json(J),
    Array(Vec<V>),
    Record(Map<String, V>, Option<J>, bool, Vec<String>),
    Function(Rc<Function>),
    Lambda(Rc<Lambda>),
    Void(Vec<J>),
}
pub(super) struct Function {
    pub(super) ast: J,
    pub(super) env: Env,
    pub(super) types: Types,
}
pub(super) struct Lambda {
    pub(super) parameters: Vec<String>,
    pub(super) body: J,
    pub(super) env: Env,
}
pub(super) fn s<'a>(v: &'a J, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
pub(super) fn arr<'a>(v: &'a J, k: &str) -> &'a [J] {
    v[k].as_array().map_or(&[], Vec::as_slice)
}
pub(super) fn flag(v: &J, k: &str) -> bool {
    v[k].as_bool().unwrap_or(false)
}
pub(super) fn ty(name: &str, args: Vec<J>) -> J {
    json!({"name":name,"args":args})
}
pub(super) fn typename(t: &J) -> String {
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
pub(super) fn substitute(t: &J, bs: &Types, depth: usize) -> R<J> {
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
pub(super) fn is_geometry(v: &V) -> bool {
    match v {
        V::Array(a) => !a.is_empty() && a.iter().all(is_geometry),
        V::Json(j) => j.get("geometry").is_some(),
        _ => false,
    }
}
pub(super) fn scalar(v: V) -> R<J> {
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
pub(super) fn expression(v: V) -> R<J> {
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
pub(super) fn list(v: V) -> R<J> {
    match &v {
        V::Array(_) => expression(v),
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        V::Json(j) if j.get("geometry").is_none() && j.get("text").is_none() => Ok(j.clone()),
        _ => Err(crate::error("Expected a sequence")),
    }
}
pub(super) fn raw(v: V) -> R<J> {
    match v {
        V::Json(j) if j.get("sequence").is_some() => Ok(j["sequence"].clone()),
        V::Json(j) => Ok(j),
        V::Array(a) => a.into_iter().map(raw).collect::<R<Vec<_>>>().map(J::Array),
        _ => Err(crate::error("Expected numeric geometry argument")),
    }
}
pub(super) struct Compiler {
    pub(super) nodes: Vec<J>,
    pub(super) parameters: Vec<J>,
    pub(super) controls: Vec<J>,
    pub(super) constraints: Vec<J>,
    pub(super) checks: Vec<J>,
    pub(super) collections: Set<String>,
    pub(super) structs: Map<String, J>,
    pub(super) traits: Map<String, J>,
    pub(super) trait_envs: Map<String, Env>,
    pub(super) implementations: Map<String, (J, Env)>,

    pub(super) active: Types,
    pub(super) serial: usize,
    pub(super) stable_ids: bool,
    pub(super) owner: String,
    pub(super) owner_serial: usize,
    pub(super) show_serial: usize,
    pub(super) author_ids: Set<String>,
    pub(super) check_serial: usize,
    pub(super) expansions: usize,
}
