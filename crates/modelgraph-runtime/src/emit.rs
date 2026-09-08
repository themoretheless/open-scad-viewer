//! Canonical graph indexing, bounded evaluation, and geometry lowering.
use crate::eval::{sequence, Evaluator, Scope, Value as RuntimeValue};
use crate::units::{Dimension, ANGLE, LENGTH, SCALAR};
use crate::{assembly, mechanical, profiles, sketch};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::rc::Rc;
use value_codec::{json, Value};

/// ECMAScript-compatible spelling for finite generated coordinates. Keep the
/// decimal interval used by JSON.stringify, including a canonical positive zero.
pub fn append_number(out: &mut String, v: f64) {
    if v == 0. {
        out.push('0');
    } else if v.abs() < 1e-6 || v.abs() >= 1e21 {
        let scientific = format!("{v:e}");
        let (mantissa, exponent) = scientific.split_once('e').unwrap();
        let exponent: i32 = exponent.parse().unwrap();
        write!(
            out,
            "{mantissa}e{}{exponent}",
            if exponent >= 0 { "+" } else { "" }
        )
        .unwrap();
    } else {
        write!(out, "{v}").unwrap();
    }
}
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
fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap()
}
fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn boolean(v: &Value, k: &str) -> bool {
    v[k].as_bool().unwrap_or(false)
}
fn children(node: &Value) -> Vec<&str> {
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
    } else if str_at(node, "op") == "if" {
        vec![str_at(node, "then"), str_at(node, "else")]
    } else {
        vec![]
    }
}
type Nodes<'a> = HashMap<&'a str, &'a Value>;
fn index_nodes<'a>(items: &'a Value, root: &str, path: &str) -> Result<Rc<Nodes<'a>>> {
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
enum GeometryType {
    Profile,
    Solid,
}
impl GeometryType {
    fn name(self) -> &'static str {
        if self == Self::Profile {
            "profile"
        } else {
            "solid"
        }
    }
}
struct Parent {
    path: String,
    matrix: assembly::Matrix,
}
struct Emitter<'a> {
    eval: Evaluator<'a>,
    bodies: HashMap<&'a str, Rc<Nodes<'a>>>,
    lines: Vec<String>,
    source_map: Vec<Value>,
    sketch_solutions: Vec<Value>,
    assembly_components: Vec<Value>,
    mechanical_reports: Vec<Value>,
    mechanical_parts: Vec<Value>,
    expanded: usize,
    profile_work: usize,
    mechanical_characters: usize,
    segments: u64,
}
impl<'a> Emitter<'a> {
    fn value(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
    ) -> Result<f64> {
        self.eval
            .field(expr, scope, &format!("{current}/{field}"), dimension)
    }
    fn vector(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
        dimension: Dimension,
    ) -> Result<Vec<f64>> {
        array(expr)
            .iter()
            .enumerate()
            .map(|(i, v)| self.value(v, scope, current, &format!("{field}/{i}"), dimension))
            .collect()
    }
    fn frame(
        &mut self,
        expr: &'a Value,
        scope: &Scope<'a>,
        current: &str,
        field: &str,
    ) -> Result<Value> {
        Ok(
            json!({"origin":self.vector(&expr["origin"],scope,current,&format!("{field}/origin"),LENGTH)?,"rotation":self.vector(&expr["rotation"],scope,current,&format!("{field}/rotation"),ANGLE)?}),
        )
    }
    fn child(
        &mut self,
        item: &'a Value,
        nodes: &Rc<Nodes<'a>>,
        scope: &Scope<'a>,
        current: &str,
        depth: usize,
        expected: GeometryType,
    ) -> Result<()> {
        self.emit(
            str_at(item, "input"),
            nodes,
            scope,
            current,
            depth + 1,
            expected,
            None,
            false,
        )
    }
    // Traversal carries distinct lexical, geometric and assembly contexts.
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        key: &str,
        nodes: &Rc<Nodes<'a>>,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
        expected: GeometryType,
        parent: Option<&Parent>,
        allow_assembly: bool,
    ) -> Result<()> {
        self.expanded += 1;
        if self.expanded > 4096 || depth > 32 {
            return Err(Error::new(
                "graph_limit",
                path,
                "Maximum depth 32 and expanded node uses 4096.",
            ));
        }
        let item = *nodes.get(key).unwrap();
        let current = format!("{path}/{key}");
        let op = str_at(item, "op");
        let fail = |code: &str, message: &str| Error::new(code, &current, message);
        let produced = if [
            "sketch",
            "rectangle",
            "circle",
            "polygon",
            "offset",
            "projection",
            "section",
        ]
        .contains(&op)
        {
            GeometryType::Profile
        } else if [
            "box",
            "sphere",
            "cylinder",
            "extrude",
            "revolve",
            "loft",
            "advanced_extrude",
            "cone",
            "torus",
            "gear",
            "thread",
            "planetary_gears",
            "planetary_spinner",
        ]
        .contains(&op)
        {
            GeometryType::Solid
        } else {
            expected
        };
        if produced != expected {
            return Err(fail("geometry_type_mismatch",&format!("Expected {}, received {}. Extrude or revolve a profile before using it as a solid.",expected.name(),produced.name())));
        }
        self.source_map
            .push(json!({"node_id":key,"line":self.lines.len()+1,"instance_path":current}));
        match op {
            "planetary_spinner" => {
                if depth != 1 {
                    return Err(fail(
                        "mechanism_scope",
                        "Planetary spinner must be the root to preserve separate parts.",
                    ));
                }
                let r = self.value(
                    &item["inner_radius"],
                    scope,
                    &current,
                    "inner_radius",
                    LENGTH,
                )?;
                let outer = self.value(
                    &item["outer_radius"],
                    scope,
                    &current,
                    "outer_radius",
                    LENGTH,
                )?;
                let bore = self.value(&item["bore"], scope, &current, "bore", LENGTH)?;
                let gap = self.value(&item["gap"], scope, &current, "gap", LENGTH)?;
                let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
                let helix =
                    self.value(&item["helix_angle"], scope, &current, "helix_angle", ANGLE)?;
                let scale = r / 30.845;
                if r <= 0.
                    || outer <= r
                    || height <= 0.
                    || gap < 0.
                    || gap > 0.6 * scale
                    || !(0.0..80.0).contains(&helix)
                    || bore < 0.
                    || bore
                        >= 2. * (23.154388 * scale - gap / 2.) * (std::f64::consts::PI / 45.).cos()
                    || outer * (std::f64::consts::PI / 96.).cos() <= r + gap / 2.
                {
                    return Err(fail(
                        "invalid_spinner",
                        "Invalid spinner dimensions, gap, bore or rim thickness.",
                    ));
                }
                self.lines.push(format!("inner_radius={};outer_radius={};center_hole_diameter={};gap={};spinner_height={};helix_angle={};",number(r),number(outer),number(bore),number(gap),number(height),number(helix)));
                self.lines
                    .push(include_str!("planetary_spinner.scad").into());
                self.mechanical_reports.push(json!({"node_id":key,"kind":"planetary_spinner","sun_teeth":32,"planet_teeth":4,"ring_teeth":40,"planet_count":18,"gap_mm":gap,"profile":"fixed sampled prototype","printability":"unknown"}));
            }
            "gear" | "thread" | "planetary_gears" => {
                if op == "planetary_gears" && depth != 1 {
                    return Err(fail("mechanism_scope","Planetary gears must be the document root; export or edit individual parts separately."));
                }
                let mut options = json!({});
                for (k, v) in item.as_object().unwrap() {
                    if k == "id" || k == "op" {
                        continue;
                    }
                    if v.is_boolean() {
                        options[k] = v.clone();
                        continue;
                    }
                    let dim = if [
                        "module",
                        "thickness",
                        "bore",
                        "backlash",
                        "clearance",
                        "rim_width",
                        "diameter",
                        "pitch",
                        "length",
                        "wall",
                    ]
                    .contains(&k.as_str())
                    {
                        LENGTH
                    } else if ["pressure_angle", "carrier_angle"].contains(&k.as_str()) {
                        ANGLE
                    } else {
                        SCALAR
                    };
                    options[k] = json!(self.value(v, scope, &current, k, dim)?);
                }
                let generated = match op {
                    "gear" => mechanical::gear(&options, &current)?,
                    "thread" => mechanical::thread(&options, &current)?,
                    _ => mechanical::planetary(&options, &current)?,
                };
                self.mechanical_characters += generated.source.chars().count();
                if self.mechanical_characters > 220000 {
                    return Err(fail("mechanical_limit","Generated mechanical source exceeds 220000 characters; reduce resolution or instances."));
                }
                let mut report = generated.report;
                report["node_id"] = json!(key);
                report["instance_path"] = json!(current);
                self.mechanical_reports.push(report);
                self.mechanical_parts.extend(generated.parts);
                self.lines
                    .extend(generated.source.split('\n').map(str::to_owned));
            }
            "assembly" => {
                if !allow_assembly || expected != GeometryType::Solid {
                    return Err(fail("assembly_scope","Assembly must be the root or a direct assembly component; boolean/transform wrappers are not allowed."));
                }
                let mut components = Vec::with_capacity(array(&item["components"]).len());
                for (i, c) in array(&item["components"]).iter().enumerate() {
                    let prefix = format!("components/{i}");
                    let mut anchors = Vec::with_capacity(array(&c["anchors"]).len());
                    for (j, a) in array(&c["anchors"]).iter().enumerate() {
                        let mut v =
                            self.frame(a, scope, &current, &format!("{prefix}/anchors/{j}"))?;
                        v["id"] = a["id"].clone();
                        anchors.push(v);
                    }
                    let mut component = json!({"id":c["id"],"input":c["input"],"anchors":anchors});
                    if let Some(p) = c.get("placement") {
                        component["placement"] =
                            self.frame(p, scope, &current, &format!("{prefix}/placement"))?;
                    }
                    if let Some(m) = c.get("mate") {
                        let mut mate = json!({"component":m["component"],"anchor":m["anchor"],"own_anchor":m["own_anchor"],"gap":self.value(&m["gap"],scope,&current,&format!("{prefix}/mate/gap"),LENGTH)?,"rotation":self.vector(&m["rotation"],scope,&current,&format!("{prefix}/mate/rotation"),ANGLE)?});
                        if let Some(j) = m.get("joint") {
                            let dim = if str_at(j, "kind") == "revolute" {
                                ANGLE
                            } else {
                                LENGTH
                            };
                            let mut joint = json!({"kind":j["kind"]});
                            for k in ["position", "min", "max"] {
                                joint[k] = json!(self.value(
                                    &j[k],
                                    scope,
                                    &current,
                                    &format!("{prefix}/joint/{k}"),
                                    dim
                                )?);
                            }
                            mate["joint"] = joint;
                        }
                        component["mate"] = mate;
                    }
                    components.push(component);
                }
                let placed = assembly::place(&components, &current)?;
                for mut component in placed {
                    if self.assembly_components.len() >= 64 {
                        return Err(fail(
                            "assembly_limit",
                            "Maximum 64 expanded assembly components.",
                        ));
                    }
                    let m = assembly::read_matrix(&component["matrix"]);
                    self.lines.push(format!(
                        "multmatrix({}){{",
                        json_string(&assembly::matrix_json(m))
                    ));
                    let world = parent.map_or(m, |p| assembly::multiply(p.matrix, m));
                    if world.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
                        return Err(fail(
                            "assembly_limit",
                            "World transform exceeds numeric limits.",
                        ));
                    }
                    let component_path =
                        format!("{current}/components/{}", str_at(&component, "id"));
                    let input = str_at(&component, "input").to_owned();
                    let is_assembly = nodes
                        .get(input.as_str())
                        .map(|n| str_at(n, "op") == "assembly")
                        .unwrap_or(false);
                    component["matrix"] = json!(world);
                    for a in component["anchors"].as_array_mut().unwrap() {
                        if let Some(p) = parent {
                            a["matrix"] = json!(assembly::multiply(
                                p.matrix,
                                assembly::read_matrix(&a["matrix"])
                            ));
                        }
                    }
                    component["instance_path"] = json!(component_path);
                    component["parent_path"] = parent.map_or(Value::Null, |p| json!(p.path));
                    component["is_assembly"] = json!(is_assembly);
                    let record_index = self.assembly_components.len();
                    self.assembly_components.push(component);
                    let start = self.lines.len();
                    let child_parent = Parent {
                        path: component_path.clone(),
                        matrix: world,
                    };
                    self.emit(
                        &input,
                        nodes,
                        scope,
                        &component_path,
                        depth + 1,
                        GeometryType::Solid,
                        Some(&child_parent),
                        true,
                    )?;
                    if !is_assembly {
                        self.assembly_components[record_index]["source"] = json!(format!(
                            "$fn={};\nmultmatrix({}){{\n{}\n}}",
                            self.segments,
                            json_string(&assembly::matrix_json(world)),
                            self.lines[start..].join("\n")
                        ));
                    }
                    self.lines.push("}".into());
                }
            }
            "evaluate" => {
                let shape =
                    self.eval
                        .resolve(&item["value"], scope, &format!("{current}/value"), 0)?;
                if let RuntimeValue::Geometry {
                    function,
                    scope: bound_scope,
                } = shape
                {
                    let fn_value = *self.eval.functions.get(function).unwrap();
                    if str_at(fn_value, "kind") != "geometry" {
                        return Err(fail("type_error", "Expected a geometry function."));
                    }
                    let body = self.bodies.get(function).unwrap().clone();
                    self.emit(
                        str_at(fn_value, "root"),
                        &body,
                        &bound_scope,
                        &format!("{current}/value:{function}"),
                        depth + 1,
                        expected,
                        None,
                        false,
                    )?;
                } else {
                    return Err(fail("type_error", "Expected a geometry value."));
                }
            }
            "call" => {
                let bound =
                    self.eval
                        .bind(str_at(item, "function"), &item["args"], scope, &current, 0)?;
                if str_at(bound.function, "kind") != "geometry" {
                    return Err(fail("type_error", "Expected a geometry function."));
                }
                let id = str_at(bound.function, "id");
                let body = self.bodies.get(id).unwrap().clone();
                self.emit(
                    str_at(bound.function, "root"),
                    &body,
                    &bound.scope,
                    &format!("{current}/call:{id}"),
                    depth + 1,
                    expected,
                    None,
                    false,
                )?;
            }
            "if" => {
                let condition =
                    self.value(&item["condition"], scope, &current, "condition", SCALAR)?;
                self.emit(
                    str_at(item, if condition != 0. { "then" } else { "else" }),
                    nodes,
                    scope,
                    &current,
                    depth + 1,
                    expected,
                    None,
                    false,
                )?;
            }
            "group" => {
                for child in children(item) {
                    self.emit(
                        child,
                        nodes,
                        scope,
                        &current,
                        depth + 1,
                        expected,
                        None,
                        false,
                    )?;
                }
            }
            "collect" => {
                let list =
                    self.eval
                        .resolve(&item["values"], scope, &format!("{current}/values"), 0)?;
                let items = sequence(&list, &current)?;
                if items.len() > 256 {
                    return Err(fail("invalid_count", "Collection exceeds 256 elements."));
                }
                for (i, v) in items.iter().enumerate() {
                    let mut local = (**scope).clone();
                    local.insert(str_at(item, "binding").into(), v.clone());
                    self.emit(
                        str_at(item, "input"),
                        nodes,
                        &Rc::new(local),
                        &format!("{current}[{i}]"),
                        depth + 1,
                        expected,
                        None,
                        false,
                    )?;
                }
            }
            "map" => {
                let count = self.value(&item["count"], scope, &current, "count", SCALAR)?;
                if count.fract() != 0. || !(1.0..=256.0).contains(&count) {
                    return Err(fail(
                        "invalid_count",
                        "Map count must be an integer from 1 to 256.",
                    ));
                }
                self.lines.push("union(){".into());
                for i in 0..count as usize {
                    let mut local = (**scope).clone();
                    local.insert(str_at(item, "index").into(), RuntimeValue::from(i as f64));
                    self.emit(
                        str_at(item, "input"),
                        nodes,
                        &Rc::new(local),
                        &format!("{current}[{i}]"),
                        depth + 1,
                        expected,
                        None,
                        false,
                    )?;
                }
                self.lines.push("}".into());
            }
            "affine" => {
                let mut m = vec![];
                for (i, row) in array(&item["rows"]).iter().enumerate() {
                    let mut values = vec![];
                    for (j, v) in array(row).iter().enumerate() {
                        values.push(self.value(
                            v,
                            scope,
                            &current,
                            &format!("rows/{i}/{j}"),
                            if j == 3 { LENGTH } else { SCALAR },
                        )?);
                    }
                    m.push(values);
                }
                let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                    - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                    + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
                if !det.is_finite() || det == 0. {
                    return Err(fail(
                        "invalid_transform",
                        "Affine matrix must be invertible.",
                    ));
                }
                if expected == GeometryType::Profile
                    && (m[2][0] != 0. || m[2][1] != 0. || m[2][3] != 0.)
                {
                    return Err(fail(
                        "nonplanar_profile",
                        "Affine transform must preserve XY.",
                    ));
                }
                m.push(vec![0., 0., 0., 1.]);
                self.lines
                    .push(format!("multmatrix({}){{", json_string(&json!(m))));
                self.child(item, nodes, scope, &current, depth, expected)?;
                self.lines.push("}".into());
            }
            "mirror" => {
                let normal = self.vector(&item["normal"], scope, &current, "normal", SCALAR)?;
                if normal.iter().all(|v| *v == 0.) {
                    return Err(fail("invalid_normal", "Mirror normal must be nonzero."));
                }
                if expected == GeometryType::Profile && normal[2] != 0. {
                    return Err(fail(
                        "nonplanar_profile",
                        "Profile mirror normal must be in XY.",
                    ));
                }
                self.lines
                    .push(format!("mirror({}){{", json_string(&json!(normal))));
                self.child(item, nodes, scope, &current, depth, expected)?;
                self.lines.push("}".into());
            }
            "projection" | "section" => {
                self.lines
                    .push(format!("projection(cut={}){{", op == "section"));
                if op == "section" {
                    let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
                    self.lines
                        .push(format!("translate([0,0,{}]){{", number(-height)));
                }
                self.child(item, nodes, scope, &current, depth, GeometryType::Solid)?;
                if op == "section" {
                    self.lines.push("}".into());
                }
                self.lines.push("}".into());
            }
            "offset" => {
                let distance =
                    self.value(&item["distance"], scope, &current, "distance", LENGTH)?;
                self.lines.push(format!(
                    "offset({}={}){{",
                    if item["mode"].as_str() == Some("delta") {
                        "delta"
                    } else {
                        "r"
                    },
                    number(distance)
                ));
                self.child(item, nodes, scope, &current, depth, GeometryType::Profile)?;
                self.lines.push("}".into());
            }
            "advanced_extrude" => {
                let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
                let twist = self.value(&item["twist"], scope, &current, "twist", ANGLE)?;
                let scale =
                    self.vector(&item["top_scale"], scope, &current, "top_scale", SCALAR)?;
                if height <= 0. || scale.iter().any(|v| *v <= 0.) {
                    return Err(fail(
                        "invalid_dimension",
                        "Height and top scales must be positive.",
                    ));
                }
                if twist.abs() > 3600. {
                    return Err(fail(
                        "invalid_angle",
                        "Twist must be within +/-3600 degrees.",
                    ));
                }
                self.lines.push(format!(
                    "linear_extrude(height={},twist={},scale={},slices={},center={}){{",
                    number(height),
                    number(twist),
                    json_string(&json!(scale)),
                    number(item["slices"].as_f64().unwrap()),
                    boolean(item, "center")
                ));
                self.child(item, nodes, scope, &current, depth, GeometryType::Profile)?;
                self.lines.push("}".into());
            }
            "cone" => {
                let bottom = self.value(
                    &item["radius_bottom"],
                    scope,
                    &current,
                    "radius_bottom",
                    LENGTH,
                )?;
                let top = self.value(&item["radius_top"], scope, &current, "radius_top", LENGTH)?;
                let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
                if bottom < 0. || top < 0. || bottom + top <= 0. || height <= 0. {
                    return Err(fail("invalid_dimension","Cone needs nonnegative radii, at least one positive radius, and positive height."));
                }
                self.lines.push(format!(
                    "cylinder(r1={},r2={},h={},center={});",
                    number(bottom),
                    number(top),
                    number(height),
                    boolean(item, "center")
                ));
            }
            "torus" => {
                let major = self.value(
                    &item["major_radius"],
                    scope,
                    &current,
                    "major_radius",
                    LENGTH,
                )?;
                let minor = self.value(
                    &item["minor_radius"],
                    scope,
                    &current,
                    "minor_radius",
                    LENGTH,
                )?;
                if minor <= 0. || major <= minor {
                    return Err(fail(
                        "invalid_dimension",
                        "Torus requires major_radius > minor_radius > 0.",
                    ));
                }
                self.lines.push(format!(
                    "rotate_extrude(angle=360){{translate([{},0,0]){{circle(r={});}}}}",
                    number(major),
                    number(minor)
                ));
            }
            "linear_pattern" | "circular_pattern" => {
                let count = self.value(&item["count"], scope, &current, "count", SCALAR)?;
                if count.fract() != 0. || !(1.0..=256.0).contains(&count) {
                    return Err(fail(
                        "invalid_count",
                        "Pattern count must be an integer from 1 to 256.",
                    ));
                }
                let step = if op == "linear_pattern" {
                    self.vector(&item["step"], scope, &current, "step", LENGTH)?
                } else {
                    vec![
                        0.,
                        0.,
                        self.value(&item["angle_step"], scope, &current, "angle_step", ANGLE)?,
                    ]
                };
                if expected == GeometryType::Profile && op == "linear_pattern" && step[2] != 0. {
                    return Err(fail(
                        "nonplanar_profile",
                        "Profile pattern must remain in XY.",
                    ));
                }
                self.lines.push("union(){".into());
                for i in 0..count as usize {
                    let vector: Vec<f64> = step.iter().map(|v| v * i as f64).collect();
                    if vector.iter().any(|v| v.abs() > 1e6) {
                        return Err(fail(
                            "graph_limit",
                            "Pattern transform exceeds numeric limits.",
                        ));
                    }
                    self.lines.push(format!(
                        "{}({}){{",
                        if op == "linear_pattern" {
                            "translate"
                        } else {
                            "rotate"
                        },
                        json_string(&json!(vector))
                    ));
                    self.emit(
                        str_at(item, "input"),
                        nodes,
                        scope,
                        &format!("{current}[{i}]"),
                        depth + 1,
                        expected,
                        None,
                        false,
                    )?;
                    self.lines.push("}".into());
                }
                self.lines.push("}".into());
            }
            "loft" => {
                let count = array(&item["profile"]).len();
                self.profile_work += count * count + count * array(&item["sections"]).len();
                if self.profile_work > 262144 {
                    return Err(fail("profile_limit", "Loft work budget exceeded."));
                }
                let mut profile = vec![];
                for (i, p) in array(&item["profile"]).iter().enumerate() {
                    let v = self.vector(p, scope, &current, &format!("profile/{i}"), LENGTH)?;
                    profile.push([v[0], v[1]]);
                }
                let mut sections = vec![];
                for (i, s) in array(&item["sections"]).iter().enumerate() {
                    let scale = self.vector(
                        &s["scale"],
                        scope,
                        &current,
                        &format!("sections/{i}/scale"),
                        SCALAR,
                    )?;
                    let offset = self.vector(
                        &s["offset"],
                        scope,
                        &current,
                        &format!("sections/{i}/offset"),
                        LENGTH,
                    )?;
                    sections.push(profiles::Section {
                        z: self.value(
                            &s["z"],
                            scope,
                            &current,
                            &format!("sections/{i}/z"),
                            LENGTH,
                        )?,
                        scale: [scale[0], scale[1]],
                        offset: [offset[0], offset[1]],
                    });
                }
                let mesh = profiles::loft(profile, &sections, &current)?;
                self.lines.push(format!(
                    "polyhedron(points={},faces={});",
                    json_string(&mesh["points"]),
                    json_string(&mesh["faces"])
                ));
            }
            "sketch" => {
                if self.sketch_solutions.len() >= 16 {
                    return Err(fail(
                        "sketch_limit",
                        "Maximum 16 sketch solves per compilation.",
                    ));
                }
                let mut points = vec![];
                for (i, p) in array(&item["points"]).iter().enumerate() {
                    points.push(json!({"id":p["id"],"position":self.vector(&p["position"],scope,&current,&format!("points/{i}/position"),LENGTH)?}));
                }
                let mut constraints = vec![];
                for (i, c) in array(&item["constraints"]).iter().enumerate() {
                    let mut constraint = c.clone();
                    match str_at(c, "kind") {
                        "fix" => {
                            constraint["at"] = json!(self.vector(
                                &c["at"],
                                scope,
                                &current,
                                &format!("constraints/{i}/at"),
                                LENGTH
                            )?)
                        }
                        "distance" => {
                            constraint["value"] = json!(self.value(
                                &c["value"],
                                scope,
                                &current,
                                &format!("constraints/{i}/value"),
                                LENGTH
                            )?)
                        }
                        _ => {}
                    }
                    constraints.push(constraint);
                }
                let boundary = array(&item["boundary"]);
                let mut seen = HashSet::new();
                for id in boundary {
                    if !seen.insert(id.as_str().unwrap()) || !points.iter().any(|p| p["id"] == *id)
                    {
                        return Err(fail(
                            "invalid_boundary",
                            "Boundary IDs must exist and be unique; closing is implicit.",
                        ));
                    }
                }
                let mut report = sketch::solve(&points, &constraints, &current)?;
                report["instance_path"] = json!(current);
                self.sketch_solutions.push(report.clone());
                let status = str_at(&report, "status");
                if status != "solved"
                    && !(status == "underconstrained" && boolean(item, "allow_underconstrained"))
                {
                    return Err(fail(&format!("sketch_{status}"),"Sketch could not produce a fully constrained profile. Inspect the solver report.").with_details(report));
                }
                let polygon: Vec<[f64; 2]> = boundary
                    .iter()
                    .map(|id| {
                        let p = array(&report["points"])
                            .iter()
                            .find(|p| p["id"] == *id)
                            .unwrap();
                        [
                            p["position"][0].as_f64().unwrap(),
                            p["position"][1].as_f64().unwrap(),
                        ]
                    })
                    .collect();
                profiles::validate(&polygon, &current)?;
                self.lines
                    .push(format!("polygon(points={});", json_string(&json!(polygon))));
            }
            "rectangle" | "box" => {
                let size = self.vector(&item["size"], scope, &current, "size", LENGTH)?;
                if size.iter().any(|v| *v <= 0.) {
                    return Err(Error::new(
                        "invalid_dimension",
                        if op == "box" {
                            format!("{current}/size")
                        } else {
                            current.clone()
                        },
                        if op == "box" {
                            "Box dimensions must be positive."
                        } else {
                            "Rectangle dimensions must be positive."
                        },
                    ));
                }
                self.lines.push(format!(
                    "{}({},center={});",
                    if op == "box" { "cube" } else { "square" },
                    json_string(&json!(size)),
                    boolean(item, "center")
                ));
            }
            "circle" | "sphere" | "cylinder" => {
                let radius = self.value(&item["radius"], scope, &current, "radius", LENGTH)?;
                if radius <= 0. {
                    return Err(Error::new(
                        "invalid_dimension",
                        if op == "circle" {
                            current.clone()
                        } else {
                            format!("{current}/radius")
                        },
                        if op == "circle" {
                            "Circle radius must be positive."
                        } else {
                            "Radius must be positive."
                        },
                    ));
                }
                if op == "cylinder" {
                    let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
                    if height <= 0. {
                        return Err(Error::new(
                            "invalid_dimension",
                            format!("{current}/height"),
                            "Height must be positive.",
                        ));
                    }
                    self.lines.push(format!(
                        "cylinder(r={},h={},center={});",
                        number(radius),
                        number(height),
                        boolean(item, "center")
                    ));
                } else {
                    self.lines.push(format!("{op}(r={});", number(radius)));
                }
            }
            "polygon" => {
                let mut points = vec![];
                for (i, p) in array(&item["points"]).iter().enumerate() {
                    let v = self.vector(p, scope, &current, &format!("points/{i}"), LENGTH)?;
                    points.push([v[0], v[1]]);
                }
                self.profile_work += points.len() * points.len();
                if self.profile_work > 262144 {
                    return Err(fail(
                        "profile_limit",
                        "Polygon validation work budget exceeded.",
                    ));
                }
                profiles::validate(&points, &current)?;
                self.lines
                    .push(format!("polygon(points={});", json_string(&json!(points))));
            }
            "extrude" | "revolve" => {
                if op == "extrude" {
                    let height = self.value(&item["height"], scope, &current, "height", LENGTH)?;
                    if height <= 0. {
                        return Err(fail(
                            "invalid_dimension",
                            "Extrusion height must be positive.",
                        ));
                    }
                    self.lines.push(format!(
                        "linear_extrude(height={},center={}){{",
                        number(height),
                        boolean(item, "center")
                    ));
                } else {
                    let angle = self.value(&item["angle"], scope, &current, "angle", ANGLE)?;
                    if angle <= 0. || angle > 360. {
                        return Err(fail(
                            "invalid_angle",
                            "Revolution angle must be greater than zero and at most 360 degrees.",
                        ));
                    }
                    self.lines
                        .push(format!("rotate_extrude(angle={}){{", number(angle)));
                }
                self.child(item, nodes, scope, &current, depth, GeometryType::Profile)?;
                self.lines.push("}".into());
            }
            "translate" | "rotate" | "scale" => {
                let vector = self.vector(
                    &item["vector"],
                    scope,
                    &current,
                    "vector",
                    if op == "rotate" {
                        ANGLE
                    } else if op == "translate" {
                        LENGTH
                    } else {
                        SCALAR
                    },
                )?;
                if expected == GeometryType::Profile
                    && ((op == "translate" && vector[2] != 0.)
                        || (op == "rotate" && (vector[0] != 0. || vector[1] != 0.))
                        || (op == "scale" && vector[2] != 1.))
                {
                    return Err(fail(
                        "nonplanar_profile",
                        "Profiles must remain in XY: translate Z=0, rotate X=Y=0, scale Z=1.",
                    ));
                }
                if op == "scale" && vector.contains(&0.) {
                    return Err(Error::new(
                        "invalid_scale",
                        format!("{current}/vector"),
                        "Scale factors must be nonzero.",
                    ));
                }
                self.lines
                    .push(format!("{op}({}){{", json_string(&json!(vector))));
                self.child(item, nodes, scope, &current, depth, expected)?;
                self.lines.push("}".into());
            }
            "union" | "intersection" | "difference" | "hull" => {
                self.lines.push(format!("{op}(){{"));
                for child in children(item) {
                    self.emit(
                        child,
                        nodes,
                        scope,
                        &current,
                        depth + 1,
                        expected,
                        None,
                        false,
                    )?;
                }
                self.lines.push("}".into());
            }
            _ => return Err(fail("invalid_document", "Unsupported geometry operation.")),
        }
        Ok(())
    }
}
pub fn compile(document: &Value) -> Result<Value> {
    let eval = Evaluator::new(document)?;
    let main = index_nodes(&document["nodes"], str_at(document, "root"), "/nodes")?;
    let mut bodies = HashMap::new();
    for fn_value in document["functions"].as_array().into_iter().flatten() {
        if str_at(fn_value, "kind") == "geometry" {
            let id = str_at(fn_value, "id");
            bodies.insert(
                id,
                index_nodes(
                    &fn_value["nodes"],
                    str_at(fn_value, "root"),
                    &format!("/functions/{id}/nodes"),
                )?,
            );
        }
    }
    let segments = document["segments"].as_f64().unwrap() as u64;
    let mut emitter = Emitter {
        eval,
        bodies,
        lines: vec![
            "// Generated from modelgraph/1; execution target: legacy/current + Manifold".into(),
            format!("$fn = {segments};"),
        ],
        source_map: vec![],
        sketch_solutions: vec![],
        assembly_components: vec![],
        mechanical_reports: vec![],
        mechanical_parts: vec![],
        expanded: 0,
        profile_work: 0,
        mechanical_characters: 0,
        segments,
    };
    let checks = emitter.eval.validate_checks()?;
    emitter.emit(
        str_at(document, "root"),
        &main,
        &Rc::new(HashMap::new()),
        "",
        1,
        GeometryType::Solid,
        None,
        true,
    )?;
    Ok(
        json!({"geometry_assertions":checks.geometry_assertions,"constraint_report":checks.constraint_report,"sketch_solutions":emitter.sketch_solutions,"assembly_components":emitter.assembly_components,"mechanical_reports":emitter.mechanical_reports,"mechanical_parts":emitter.mechanical_parts,"source":emitter.lines.join("\n"),"source_map":emitter.source_map,"execution_target":"legacy/current+own-rust-cad"}),
    )
}
