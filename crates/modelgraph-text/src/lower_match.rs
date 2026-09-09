// Compile each arm in an independent scope. Pattern bindings are renamed to
// fresh runtime locals, so generated loop locals and outer names cannot collide.
fn empty_value_sequence(value: &J) -> bool {
    (s(value, "op") == "list" && arr(value, "items").is_empty())
        || (["checked", "memo"].contains(&s(value, "op")) && empty_value_sequence(&value["value"]))
}
fn empty_match_sequence(value: &J) -> bool {
    s(value, "op") == "match" && !arr(value, "arms").is_empty()
        && arr(value, "arms").iter().all(|arm| {
            empty_value_sequence(&arm["body"])
        })
}
fn pattern_bindings(p: &J) -> R<Set<String>> {
    let mut names = Set::new();
    let mut add = |children: Set<String>| -> R<()> {
        for name in children {
            if !names.insert(name.clone()) {
                return Err(format!("Duplicate pattern binding {name}"));
            }
        }
        Ok(())
    };
    match s(p, "kind") {
        "bind" => {
            add(Set::from([s(p, "name").into()]))?;
        }
        "as" => {
            add(Set::from([s(p, "name").into()]))?;
            add(pattern_bindings(&p["pattern"])?)?;
        }
        "or" => {
            let alternatives = arr(p, "patterns");
            let expected =
                pattern_bindings(alternatives.first().ok_or("Empty pattern alternatives")?)?;
            for alternative in &alternatives[1..] {
                if pattern_bindings(alternative)? != expected {
                    return Err("Pattern alternatives must bind the same names".into());
                }
            }
            add(expected)?;
        }
        "list" => {
            for item in arr(p, "prefix").iter().chain(arr(p, "suffix")) {
                add(pattern_bindings(item)?)?;
            }
            if let Some(rest) = p["rest"].as_str().filter(|r| *r != "_") {
                add(Set::from([rest.into()]))?;
            }
        }
        "record" => {
            if let Some(fields) = p["fields"].as_object() {
                for item in fields.values() {
                    add(pattern_bindings(item)?)?;
                }
            }
        }
        "type" => add(pattern_bindings(&p["pattern"])?)?,
        _ => {}
    }
    Ok(names)
}
impl Compiler {
    fn runtime_type_descriptor(&self,t:&J,depth:usize)->R<J> {
        if depth>16 {return Err("Runtime type nesting exceeds 16".into());}
        let name=s(t,"name");
        let mut descriptor=t.clone();
        if name=="Vec" {
            descriptor["args"]=json!([self.runtime_type_descriptor(&t["args"][0],depth+1)?]);
        } else if let Some(def)=self.structs.get(name) {
            let mut bindings=Types::new();
            for (generic,value) in arr(def,"generics").iter().zip(arr(t,"args")) {bindings.insert(generic.as_str().ok_or("Invalid generic name")?.into(),value.clone());}
            let mut fields=value_codec::Map::new();
            for field in arr(def,"fields") {
                let ty=substitute(&field["type"],&bindings,0)?;
                fields.insert(s(field,"name").into(),self.runtime_type_descriptor(&ty,depth+1)?);
            }
            descriptor["fields"]=J::Object(fields);
        }
        Ok(descriptor)
    }
    fn portable_pattern(&mut self, p: &J, e: &Env, b: usize, names: &Map<String, String>) -> R<J> {
        let mut result = p.clone();
        match s(p, "kind") {
            "bind" => {
                result["name"] = json!(names.get(s(p, "name")).ok_or("Missing pattern binding")?)
            }
            "as" => {
                result["name"] = json!(names.get(s(p, "name")).ok_or("Missing alias binding")?);
                result["pattern"] = self.portable_pattern(&p["pattern"], e, b, names)?;
            }
            "type" => result["pattern"] = self.portable_pattern(&p["pattern"], e, b, names)?,
            "literal" => result["value"] = expression(self.eval(&p["value"], e, b)?)?,
            "range" => {
                result["start"] = expression(self.eval(&p["start"], e, b)?)?;
                result["end"] = expression(self.eval(&p["end"], e, b)?)?;
            }
            "or" => {
                result["patterns"] = json!(arr(p, "patterns")
                    .iter()
                    .map(|p| self.portable_pattern(p, e, b, names))
                    .collect::<R<Vec<_>>>()?)
            }
            "list" => {
                for field in ["prefix", "suffix"] {
                    result[field] = json!(arr(p, field)
                        .iter()
                        .map(|p| self.portable_pattern(p, e, b, names))
                        .collect::<R<Vec<_>>>()?);
                }
                if let Some(rest) = p["rest"].as_str().filter(|r| *r != "_") {
                    result["rest"] = json!(names.get(rest).ok_or("Missing list rest binding")?);
                }
            }
            "record" => {
                let mut fields = value_codec::Map::new();
                for (name, p) in p["fields"].as_object().ok_or("Expected pattern fields")? {
                    fields.insert(name.clone(), self.portable_pattern(p, e, b, names)?);
                }
                result["fields"] = J::Object(fields);
            }
            _ => {}
        }
        Ok(result)
    }
    fn match_value(&mut self, a: &J, e: &Env, b: usize) -> R<V> {
        let value = expression(self.eval(&a["left"], e, b)?)?;
        let mut arms = Vec::new();
        let mut results = Vec::new();
        for arm in arr(a, "items") {
            let bindings = pattern_bindings(&arm["pattern"])?;
            let mut names = Map::new();
            let mut scope = e.clone();
            for name in bindings {
                self.serial += 1;
                let local = format!("p{}", self.serial);
                scope.insert(name.clone(), V::Json(json!({"local":&local})));
                names.insert(name, local);
            }
            let pattern = self.portable_pattern(&arm["pattern"], e, b, &names)?;
            let mut compiled = json!({"pattern":pattern});
            if let Some(guard) = arm.get("guard") {
                compiled["guard"] = scalar(self.eval(guard, &scope, b)?)?;
            }
            let result = self.eval(&arm["result"], &scope, b)?;
            arms.push(compiled);
            results.push(result);
        }
        if results.iter().any(is_geometry) {
            if !results
                .iter()
                .all(|r| is_geometry(r) || matches!(r,V::Array(a) if a.is_empty()) || matches!(r,V::Json(j) if empty_value_sequence(&j["sequence"])))
            {
                return Err("Match arms must all return geometry or all return values".into());
            }
            let mut collection = false;
            for (arm, result) in arms.iter_mut().zip(results) {
                let id = self.geometry(result)?;
                collection |= self.collections.contains(&id);
                arm["input"] = json!(id);
            }
            let result = self.add(json!({"op":"match","value":value,"arms":arms}))?;
            if collection {
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
            }
            return Ok(result);
        }
        let sequence = results.iter().all(|r| {
            matches!(r, V::Array(_)) || matches!(r,V::Json(j) if j.get("sequence").is_some())
        });
        for (arm, result) in arms.iter_mut().zip(results) {
            arm["body"] = expression(result)?;
        }
        let result = json!({"op":"match","input":value,"arms":arms});
        Ok(V::Json(if sequence {
            json!({"sequence":result})
        } else {
            result
        }))
    }
}
