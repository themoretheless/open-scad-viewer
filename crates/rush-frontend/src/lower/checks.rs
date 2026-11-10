use super::*;

impl Compiler {
    pub(super) fn protect(&mut self, v: V, checks: &[J]) -> R<V> {
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
    pub(super) fn memo_value(&mut self, value: V) -> R<V> {
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
    pub(super) fn geometry_has_checks(&self, id: &str, visited: &mut Set<String>) -> bool {
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
    pub(super) fn gather_effects(&self, value: &V, checks: &mut Vec<J>) {
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
    pub(super) fn needs_assertion(&self, value: &V) -> bool {
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
    pub(super) fn statement_checks(&mut self, value: &V) {
        let mut checks = Vec::new();
        self.gather_effects(value, &mut checks);
        if !checks.is_empty() {
            let id = self.check_id("v");
            self.constraints.push(json!({"id":id,"left":{"op":"checked","checks":checks,"value":1},"relation":"eq","right":1,"message":"Statement checks"}));
        }
    }
    pub(super) fn check(&mut self, kind: &str, a: &J, e: &Env) -> R<()> {
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
    pub(super) fn local_check(&mut self, kind: &str, value: &J, scope: &Env) -> R<J> {
        let constraints = self.constraints.len();
        let geometry = self.checks.len();
        self.check(kind, value, scope)?;
        let check = json!({"op":"assert_value","constraints":self.constraints.split_off(constraints),"geometry_assertions":self.checks.split_off(geometry),"value":1});
        Ok(json!({"op":"memo","id":self.check_id("m"),"value":check}))
    }
    pub(super) fn check_id(&mut self, prefix: &str) -> String {
        self.check_serial += 1;
        format!("{prefix}{}", self.check_serial)
    }
}
