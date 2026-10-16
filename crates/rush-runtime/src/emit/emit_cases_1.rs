use super::*;
impl<'a> Emitter<'a> {
    pub(super) fn emit_cases_1(&mut self, cx: NativeEmitCtx<'a, '_>) -> Result<()> {
        let NativeEmitCtx { item, key, op, nodes, scope, current, depth, expected, parent, allow_assembly, source_start } = cx;
        let fail = |code: &str, message: &str| Error::new(code, current, message);
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
                    .push(include_str!("../planetary_spinner.scad").into());
                self.mechanical_reports.push(json!({"node_id":key,"kind":"planetary_spinner","sun_teeth":32,"planet_teeth":4,"ring_teeth":40,"planet_count":18,"gap_mm":gap,"profile":"fixed sampled prototype","printability":"unknown"}));
            }
            "gear" | "thread" | "planetary_gears" => {
                if op == "planetary_gears" && depth != 1 {
                    return Err(fail(
                        "mechanism_scope",
                        "Planetary gears must be the document root; export or edit individual parts separately.",
                    ));
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
                    } else if ["pressure_angle", "carrier_angle", "helix_angle"]
                        .contains(&k.as_str())
                    {
                        ANGLE
                    } else {
                        SCALAR
                    };
                    options[k] = json!(self.value(v, scope, &current, k, dim)?);
                }
                let generated = match op {
                    "gear" => mechanical_core::gear(&options, &current)?,
                    "thread" => mechanical_core::thread(&options, &current)?,
                    _ => mechanical_core::planetary(&options, &current)?,
                };
                self.mechanical_characters += generated.source.chars().count();
                if self.mechanical_characters > 220000 {
                    return Err(fail(
                        "mechanical_limit",
                        "Generated mechanical source exceeds 220000 characters; reduce resolution or instances.",
                    ));
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
                    return Err(fail(
                        "assembly_scope",
                        "Assembly must be the root or a direct assembly component; boolean/transform wrappers are not allowed.",
                    ));
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
                        .is_some_and(|n| str_at(n, "op") == "assembly");
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
            "assert" => {
                self.eval.scoped_checks(item, scope, &current)?;
                self.emit(
                    str_at(item, "input"),
                    nodes,
                    scope,
                    &current,
                    depth + 1,
                    expected,
                    parent,
                    allow_assembly,
                )?;
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
            "match" => {
                let (arm, bound) = self.eval.select_match_arm(
                    &item["value"],
                    array(&item["arms"]),
                    scope,
                    &current,
                    0,
                )?;
                self.emit(
                    str_at(&item["arms"][arm], "input"),
                    nodes,
                    &bound,
                    &format!("{current}/arms/{arm}"),
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
            _ => unreachable!("selected native emitter family"),
        }
        if self.assertion_targets.contains(key) {
            let snapshot_key = (item as *const Value as usize, Rc::as_ptr(scope) as usize);
            if !self.assertion_snapshots.contains_key(&snapshot_key) {
                let source = format!(
                    "$fn = {};\n{}",
                    self.segments,
                    self.lines[source_start..].join("\n")
                );
                let source = if self.assertion_snapshot_bytes + source.len() <= 1_000_000 {
                    self.assertion_snapshot_bytes += source.len();
                    Some(source)
                } else {
                    None
                };
                self.assertion_snapshots.insert(
                    snapshot_key,
                    AssertionSnapshot {
                        geometry_type: expected,
                        _scope: scope.clone(),
                        source,
                    },
                );
            }
        }
        Ok(())
    }
}
