use super::*;
impl<'a> Emitter<'a> {
    pub(super) fn emit_cases_2(&mut self, cx: NativeEmitCtx<'a, '_>) -> Result<()> {
        let NativeEmitCtx { item, key, op, nodes, scope, current, depth, expected, parent, allow_assembly, source_start } = cx;
        let fail = |code: &str, message: &str| Error::new(code, current, message);
        match op {
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
                    return Err(fail(
                        "invalid_dimension",
                        "Cone needs nonnegative radii, at least one positive radius, and positive height.",
                    ));
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
                let size = self.geometry_vector(
                    &item["size"],
                    scope,
                    &current,
                    "size",
                    LENGTH,
                    if op == "box" { 3 } else { 2 },
                )?;
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
                let vector = self.geometry_vector(
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
                    3,
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
