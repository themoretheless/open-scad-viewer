use super::*;

impl<'a> Evaluator<'a> {
    pub fn validate_checks(&mut self) -> Result<Checks> {
        let (constraint_report, geometry_assertions) = self.evaluate_checks(self.document)?;
        Ok(Checks {
            constraint_report,
            geometry_assertions,
        })
    }
    pub fn evaluate_checks(&mut self, document: &'a Json) -> Result<(Json, Json)> {
        let scope = self.global_scope.clone();
        for (i, assertion) in array(&document["assertions"]).iter().enumerate() {
            if self.evaluate(
                &assertion["condition"],
                &scope,
                &format!("/assertions/{i}/condition"),
                0,
            )? == 0.0
            {
                return Err(Error::new(
                    "assertion_failed",
                    format!("/assertions/{i}"),
                    string(&assertion["message"]),
                ));
            }
        }
        let report = self.evaluate_constraints(&document["constraints"], &scope, "/constraints")?;
        let geometry = self.resolve_geometry_checks(
            &document["geometry_assertions"],
            &scope,
            "/geometry_assertions",
        )?;
        Ok((Json::Array(report), Json::Array(geometry)))
    }
    pub fn scoped_checks(&mut self, value: &'a Json, scope: &Scope<'a>, path: &str) -> Result<()> {
        if self.suppress_checks {
            return Ok(());
        }
        let report = self.evaluate_constraints(
            &value["constraints"],
            scope,
            &format!("{path}/constraints"),
        )?;
        self.constraint_report.extend(report);
        let checks = self.resolve_geometry_checks(
            &value["geometry_assertions"],
            scope,
            &format!("{path}/geometry_assertions"),
        )?;
        self.geometry_check_count += checks.len();
        if self.geometry_check_count > 256 {
            return Err(Error::new(
                "assertion_limit",
                path,
                "Maximum 256 expanded geometry checks.",
            ));
        }
        self.pending_geometry_checks
            .extend(checks.into_iter().map(|mut check| {
                check["instance_path"] = json!(path);
                PendingGeometryCheck {
                    check,
                    scope: scope.clone(),
                }
            }));
        Ok(())
    }
    pub(super) fn evaluate_constraints(
        &mut self,
        constraints: &'a Json,
        scope: &Scope<'a>,
        base: &str,
    ) -> Result<Vec<Json>> {
        self.expanded_checks += array(constraints).len();
        if self.expanded_checks > 4096 {
            return Err(Error::new(
                "assertion_limit",
                base,
                "Maximum 4096 expanded scalar checks.",
            ));
        }
        let mut ids = HashSet::new();
        let mut report = Vec::with_capacity(array(constraints).len());
        for (i, constraint) in array(constraints).iter().enumerate() {
            let path = format!("{base}/{i}");
            if !ids.insert(string(&constraint["id"])) {
                return Err(Error::new(
                    "duplicate_id",
                    &path,
                    "Constraint IDs must be unique.",
                ));
            }
            let left = numeric_value(
                &self.resolve(&constraint["left"], scope, &format!("{path}/left"), 0)?,
                &path,
            )?;
            let right = numeric_value(
                &self.resolve(&constraint["right"], scope, &format!("{path}/right"), 0)?,
                &path,
            )?;
            units::equal(left, right, &path)?;
            let relation = string(&constraint["relation"]);
            if matches!(relation, "lt" | "gt") && constraint.get("tolerance").is_some() {
                return Err(Error::new(
                    "invalid_tolerance",
                    &path,
                    "Strict comparisons do not accept tolerance.",
                ));
            }
            let mut tolerance = 0.0;
            if let Some(value) = constraint.get("tolerance") {
                let value = numeric_value(
                    &self.resolve(value, scope, &format!("{path}/tolerance"), 0)?,
                    &path,
                )?;
                units::equal(left, value, &path)?;
                tolerance = value.value;
                if tolerance < 0.0 {
                    return Err(Error::new(
                        "invalid_tolerance",
                        &path,
                        "Tolerance must be nonnegative.",
                    ));
                }
            }
            let (a, b) = (left.value, right.value);
            let passed = match relation {
                "lt" => a < b,
                "gt" => a > b,
                "eq" => (a - b).abs() <= tolerance,
                "le" => a <= b + tolerance,
                _ => a >= b - tolerance,
            };
            report.push(json!({"id":constraint["id"],"path":path,"passed":passed,"status":if passed {"passed"} else {"failed"},"actual":a,"expected":b,"relation":relation,"tolerance":tolerance,"dimension":left.dimension,"message":constraint["message"]}));
        }
        if report.iter().any(|value| value["passed"] == false) {
            let failures = report
                .iter()
                .filter(|value| value["passed"] == false)
                .map(|value| {
                    let relation = match value["relation"].as_str().unwrap_or("") {
                        "le" => "<=",
                        "ge" => ">=",
                        "eq" => "==",
                        "lt" => "<",
                        "gt" => ">",
                        _ => "?",
                    };
                    format!(
                        "{} (actual: {}; required: {} {}; tolerance: {})",
                        value["message"].as_str().unwrap_or("Constraint failed"),
                        value["actual"],
                        relation,
                        value["expected"],
                        value["tolerance"]
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Err(
                Error::new("constraint_failed", base, &failures).with_details(Json::Array(report))
            );
        }
        Ok(report)
    }
    pub(super) fn resolve_geometry_checks(
        &mut self,
        checks: &'a Json,
        scope: &Scope<'a>,
        base: &str,
    ) -> Result<Vec<Json>> {
        let mut geometry = Vec::with_capacity(array(checks).len());
        let mut ids = HashSet::new();
        for (i, check) in array(checks).iter().enumerate() {
            let path = format!("{base}/{i}");
            if !ids.insert(string(&check["id"])) {
                return Err(Error::new(
                    "duplicate_id",
                    &path,
                    "Check IDs must be unique.",
                ));
            }
            let kind = string(&check["check"]);
            let measured = matches!(kind, "height" | "width" | "depth");
            let expected = if let Some(value) = check.get("expected") {
                let value = numeric_value(&self.resolve(value, scope, &path, 0)?, &path)?;
                if measured {
                    units::equal(value, units::quantity(1.0, "mm", &path)?, &path)?;
                } else {
                    units::scalar(value, &path)?;
                }
                value.value
            } else if kind == "hasBodies" || measured {
                return Err(Error::new(
                    "missing_expected",
                    &path,
                    "Expected value required.",
                ));
            } else {
                0.0
            };
            if expected < 0.0 || (kind == "hasBodies" && expected.fract() != 0.0) {
                return Err(Error::new(
                    "invalid_expected",
                    &path,
                    "Expected value must be nonnegative; body count must be integral.",
                ));
            }
            let mut tolerance = 0.0;
            if let Some(value) = check.get("tolerance") {
                let value = numeric_value(&self.resolve(value, scope, &path, 0)?, &path)?;
                if !measured {
                    return Err(Error::new(
                        "invalid_tolerance",
                        &path,
                        "Tolerance applies only to measurements.",
                    ));
                }
                units::equal(value, units::quantity(1.0, "mm", &path)?, &path)?;
                tolerance = value.value;
                if tolerance < 0.0 {
                    return Err(Error::new(
                        "invalid_tolerance",
                        &path,
                        "Tolerance must be nonnegative.",
                    ));
                }
            }
            if !measured && kind != "hasBodies" && check.get("expected").is_some() {
                return Err(Error::new(
                    "invalid_expected",
                    &path,
                    "Topology checks take no expected argument.",
                ));
            }
            let mut result = check.clone();
            result["expected"] = json!(expected);
            result["tolerance"] = json!(tolerance);
            geometry.push(result);
        }
        Ok(geometry)
    }
}
