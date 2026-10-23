use super::*;

pub fn number(v: f64) -> String {
    let mut out = String::with_capacity(24);
    append_number(&mut out, v);
    out
}
pub fn json_string(v: &Value) -> String {
    fn append(v: &Value, out: &mut String) {
        match v {
            Value::Number(n) => append_number(out, n.as_f64().unwrap()),
            Value::Array(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i != 0 {
                        out.push(',');
                    }
                    append(v, out);
                }
                out.push(']');
            }
            Value::Object(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i != 0 {
                        out.push(',');
                    }
                    out.push_str(&value_codec::to_string(k).unwrap());
                    out.push(':');
                    append(v, out);
                }
                out.push('}');
            }
            _ => out.push_str(&v.to_string()),
        }
    }
    let mut out = String::new();
    append(v, &mut out);
    out
}
pub(super) fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap()
}
pub(super) fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
pub(super) fn boolean(v: &Value, k: &str) -> bool {
    v[k].as_bool().unwrap_or(false)
}
pub(super) fn children(node: &Value) -> Vec<&str> {
    if str_at(node, "op") == "assembly" {
        array(&node["components"])
            .iter()
            .map(|c| str_at(c, "input"))
            .collect()
    } else if let Some(v) = node.get("input") {
        vec![v.as_str().unwrap()]
    } else if let Some(v) = node.get("inputs") {
        array(v).iter().map(|v| v.as_str().unwrap()).collect()
    } else if let Some(v) = node.get("base") {
        std::iter::once(v.as_str().unwrap())
            .chain(array(&node["subtract"]).iter().map(|v| v.as_str().unwrap()))
            .collect()
    } else if str_at(node, "op") == "match" {
        array(&node["arms"])
            .iter()
            .map(|arm| str_at(arm, "input"))
            .collect()
    } else if str_at(node, "op") == "if" {
        vec![str_at(node, "then"), str_at(node, "else")]
    } else {
        vec![]
    }
}
pub(super) type Nodes<'a> = HashMap<&'a str, &'a Value>;
pub(super) fn index_nodes<'a>(
    items: &'a Value,
    root: &str,
    path: &str,
    assertion_targets: &HashSet<&str>,
) -> Result<Rc<Nodes<'a>>> {
    let items = array(items);
    let mut nodes = HashMap::with_capacity(items.len());
    for item in items {
        if nodes.insert(str_at(item, "id"), item).is_some() {
            return Err(Error::new("duplicate_id", path, "Node IDs must be unique."));
        }
    }
    fn walk<'a>(
        key: &'a str,
        depth: usize,
        nodes: &Nodes<'a>,
        active: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
        path: &str,
    ) -> Result<()> {
        if depth > 32 {
            return Err(Error::new("graph_limit", path, "Maximum depth 32."));
        }
        if active.contains(key) {
            return Err(Error::new("cycle", path, format!("Cycle through {key}.")));
        }
        if visited.contains(key) {
            return Ok(());
        }
        let item = nodes
            .get(key)
            .ok_or_else(|| Error::new("unknown_node", path, format!("Unknown node {key}.")))?;
        active.insert(key);
        for child in children(item) {
            walk(child, depth + 1, nodes, active, visited, path)?;
        }
        active.remove(key);
        visited.insert(key);
        Ok(())
    }
    let actual_root = nodes
        .get_key_value(root)
        .map(|(k, _)| *k)
        .ok_or_else(|| Error::new("unknown_node", path, format!("Unknown node {root}.")))?;
    let mut visited = HashSet::with_capacity(nodes.len());
    walk(
        actual_root,
        1,
        &nodes,
        &mut HashSet::new(),
        &mut visited,
        path,
    )?;
    // A checked intermediate is a graph use even when it is not displayed.
    for target in assertion_targets {
        if let Some((key, _)) = nodes.get_key_value(target) {
            walk(key, 1, &nodes, &mut HashSet::new(), &mut visited, path)?;
        }
    }
    if visited.len() != nodes.len() {
        return Err(Error::new(
            "unreachable_node",
            path,
            "Every node must be reachable from root; remove unused nodes.",
        ));
    }
    Ok(Rc::new(nodes))
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum GeometryType {
    Any,
    Profile,
    Solid,
}
impl GeometryType {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Any => "geometry",
            Self::Profile => "profile",
            Self::Solid => "solid",
        }
    }
}
pub(super) struct Parent {
    pub(super) path: String,
    pub(super) matrix: assembly::Matrix,
}
pub(super) struct AssertionSnapshot<'a> {
    pub(super) geometry_type: GeometryType,
    // Retain the scope so its pointer cannot be reused by another invocation.
    pub(super) _scope: Scope<'a>,
    pub(super) source: Option<String>,
}
pub(super) struct Emitter<'a> {
    pub(super) assertion_targets: HashSet<&'a str>,
    pub(super) assertion_snapshots: HashMap<(usize, usize), AssertionSnapshot<'a>>,
    pub(super) assertion_snapshot_bytes: usize,
    pub(super) eval: Evaluator<'a>,
    pub(super) bodies: HashMap<&'a str, Rc<Nodes<'a>>>,
    pub(super) lines: Vec<String>,
    pub(super) source_map: Vec<Value>,
    pub(super) sketch_solutions: Vec<Value>,
    pub(super) assembly_components: Vec<Value>,
    pub(super) mechanical_reports: Vec<Value>,
    pub(super) mechanical_parts: Vec<Value>,
    pub(super) expanded: usize,
    pub(super) profile_work: usize,
    pub(super) mechanical_characters: usize,
    pub(super) segments: u64,
}
