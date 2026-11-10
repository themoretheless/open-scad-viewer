use super::*;
use value_codec::Map;
/// Literal typed scalar equality, including the sign bit of floating zero.
pub(super) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (
            Value::Number(value_codec::Number::Float(a)),
            Value::Number(value_codec::Number::Float(b)),
        ) => a.to_bits() == b.to_bits(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ka, a), (kb, b))| ka == kb && same(a, b))
        }
        _ => a == b,
    }
}
pub(super) fn object<'a>(v: &'a Value, keys: &[&str]) -> Result<&'a Map<String, Value>> {
    let o = v
        .as_object()
        .ok_or_else(|| error("History field must be an object"))?;
    if o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)) {
        return Err(error(
            "History object fields do not match the versioned schema",
        ));
    }
    Ok(o)
}
pub(super) fn usize_value(v: &Value) -> Result<usize> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| error("History ordinal must be a bounded unsigned integer"))
}
pub(super) fn within(caps: Limits, left: Limits) -> bool {
    caps.max_cells <= left.max_cells
        && caps.max_pairs <= left.max_pairs
        && caps.max_domain_cells <= left.max_domain_cells
        && caps.mass_evaluations <= left.mass_evaluations
}
pub(super) fn limits(v: &Value) -> Result<Limits> {
    let o = object(v, &["cells", "pairs", "domain", "mass"])?;
    Ok(Limits {
        max_cells: usize_value(&o["cells"])?,
        max_pairs: usize_value(&o["pairs"])?,
        max_domain_cells: usize_value(&o["domain"])?,
        mass_evaluations: usize_value(&o["mass"])?,
    })
}
pub(super) fn trim(t: brep_topology::CoedgeTrim) -> Value {
    value_codec::json!({"curve":t.curve_parameter,"pcurve":t.pcurve_parameter,"lifts":t.periodic_lift})
}
pub(super) fn authored(records: &[AuthoredRelation]) -> Value {
    let relations=records.iter().map(|r|value_codec::json!({
        "face":r.face,"wire":r.wire,"coedge":r.coedge,"original":trim(r.original),
        "fragments":r.fragments.iter().map(|f|value_codec::json!({"range":f.relation_range.iter().map(ToString::to_string).collect::<Vec<_>>(),"edge":f.edge,"trim":trim(f.transferred)})).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    Value::Array(relations)
}
pub(super) fn maps(s: &Split) -> Value {
    value_codec::json!({"edges":s.edges,"vertices":s.vertices,"relations":authored(&s.relations)})
}
fn step(s: &Split) -> Value {
    value_codec::json!({
        "edge":s.edge,"parent":s.source.1.edges[s.edge].to_string(),"cuts":s.cuts,"key":s.key,"uvTolerance":s.uv_tolerance,
        "limits":{"cells":s.limits.max_cells,"pairs":s.limits.max_pairs,"domain":s.limits.max_domain_cells,"mass":s.limits.mass_evaluations},
        "model":s.model,"maps":maps(s)
    })
}
pub(super) fn document(h: &History, extra: Option<&Split>) -> Value {
    let steps = h.steps.iter().chain(extra).map(step).collect::<Vec<_>>();
    value_codec::json!({"schema":"brep-edge-history/1","root":h.root,"steps":steps})
}
/// Measure the exact canonical JSON footprint without allocating its encoded
/// text first; finite values, bounded nodes and bounded recursion are mandatory.
pub(super) fn bounded(v: &Value, max: usize) -> Result<usize> {
    fn string(s: &str) -> usize {
        2 + s
            .bytes()
            .map(|b| match b {
                b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 8 | 12 => 2,
                0..=31 => 6,
                _ => 1,
            })
            .sum::<usize>()
    }
    fn walk(v: &Value, left: &mut usize, nodes: &mut usize, depth: usize) -> Result<()> {
        if depth > 128 || *nodes == 2000000 {
            return Err(error(
                "Edge-history structure exceeds bounded parser limits",
            ));
        }
        *nodes += 1;
        let mut spend = |n: usize| -> Result<()> {
            if n > *left {
                return Err(error("Edge-history document exceeds byte allowance"));
            }
            *left -= n;
            Ok(())
        };
        match v {
            Value::Null => spend(4)?,
            Value::Bool(b) => spend(if *b { 4 } else { 5 })?,
            Value::Number(n) => {
                if n.as_f64().is_none_or(|v| !v.is_finite()) {
                    return Err(error("History numeric scalars must be finite"));
                }
                spend(n.to_string().len())?;
            }
            Value::String(s) => spend(string(s))?,
            Value::Array(a) => {
                spend(2 + a.len().saturating_sub(1))?;
                for v in a {
                    walk(v, left, nodes, depth + 1)?;
                }
            }
            Value::Object(o) => {
                spend(2 + o.len().saturating_sub(1))?;
                for (k, v) in o {
                    let n = string(k) + 1;
                    if n > *left {
                        return Err(error("Edge-history document exceeds byte allowance"));
                    }
                    *left -= n;
                    walk(v, left, nodes, depth + 1)?;
                }
            }
        }
        Ok(())
    }
    let mut left = max;
    walk(v, &mut left, &mut 0, 0)?;
    Ok(max - left)
}
