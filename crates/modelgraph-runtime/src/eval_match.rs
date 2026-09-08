// Structural matches are predicates: a different value kind or dimension is a
// mismatch, never an arithmetic type error. Only selected guards/bodies execute.
impl<'a> Evaluator<'a> {
    pub fn select_match_arm(
        &mut self,
        input: &'a Json,
        arms: &'a [Json],
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<(usize, Scope<'a>)> {
        let value = self.resolve(input, scope, &format!("{path}/input"), depth + 1)?;
        for (index, arm) in arms.iter().enumerate() {
            let arm_path = format!("{path}/arms/{index}");
            let mut bindings = HashMap::new();
            if !self.match_pattern(
                &arm["pattern"], &value, scope, &mut bindings,
                &format!("{arm_path}/pattern"), depth + 1,
            )? {
                continue;
            }
            let mut bound = (**scope).clone();
            bound.extend(bindings);
            let bound = Rc::new(bound);
            if let Some(guard) = arm.get("guard") {
                if self.evaluate(guard, &bound, &format!("{arm_path}/guard"), depth + 1)? == 0.0 {
                    continue;
                }
            }
            return Ok((index, bound));
        }
        Err(Error::new(
            "non_exhaustive_match", path,
            "No match arm accepted this value. Add a fallback '_' arm or handle the missing case.",
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn match_pattern(
        &mut self,
        pattern: &'a Json,
        value: &Value<'a>,
        scope: &Scope<'a>,
        bindings: &mut HashMap<String, Value<'a>>,
        path: &str,
        depth: usize,
    ) -> Result<bool> {
        self.tick(depth, path)?;
        let insert = |bindings: &mut HashMap<String, Value<'a>>, name: &str, value: Value<'a>| {
            if bindings.insert(name.to_owned(), value).is_some() {
                Err(Error::new("invalid_pattern", path, format!("Duplicate pattern binding {name}.")))
            } else {
                Ok(())
            }
        };
        Ok(match string(&pattern["kind"]) {
            "wildcard" => true,
            "bind" => {
                insert(bindings, string(&pattern["name"]), value.clone())?;
                true
            }
            "literal" => {
                let literal = self.resolve(&pattern["value"], scope, path, depth + 1)?;
                match (value, literal) {
                    (Value::Numeric(a), Value::Numeric(b)) => *a == b,
                    (Value::Text(a), Value::Text(b)) => *a == b,
                    _ => false,
                }
            }
            "as" => {
                if !self.match_pattern(&pattern["pattern"], value, scope, bindings, path, depth + 1)? {
                    return Ok(false);
                }
                insert(bindings, string(&pattern["name"]), value.clone())?;
                true
            }
            "or" => {
                for (index, alternative) in array(&pattern["patterns"]).iter().enumerate() {
                    // Failed alternatives cannot leak bindings into later ones.
                    let mut trial = bindings.clone();
                    if self.match_pattern(alternative, value, scope, &mut trial,
                        &format!("{path}/patterns/{index}"), depth + 1)? {
                        *bindings = trial;
                        return Ok(true);
                    }
                }
                false
            }
            "range" => {
                let Value::Numeric(value) = value else { return Ok(false) };
                let start = self.resolve(&pattern["start"], scope, &format!("{path}/start"), depth + 1)?;
                let end = self.resolve(&pattern["end"], scope, &format!("{path}/end"), depth + 1)?;
                let (Value::Numeric(start), Value::Numeric(end)) = (start, end) else {
                    return Err(Error::new("invalid_pattern", path, "Range pattern endpoints must be numbers or quantities."));
                };
                if start.dimension != end.dimension {
                    return Err(Error::new("invalid_pattern", path, "Range pattern endpoints must have the same dimension."));
                }
                if start.value > end.value {
                    return Err(Error::new("invalid_pattern", path, "Range pattern start must not exceed its end."));
                }
                value.dimension == start.dimension && value.value >= start.value
                    && if pattern["inclusive"] == true { value.value <= end.value } else { value.value < end.value }
            }
            "list" => {
                let Value::List(values) = value else { return Ok(false) };
                let prefix = array(&pattern["prefix"]);
                let suffix = array(&pattern["suffix"]);
                let rest = pattern.get("rest").and_then(Json::as_str);
                let fixed = prefix.len() + suffix.len();
                if values.len() < fixed || (rest.is_none() && values.len() != fixed) {
                    return Ok(false);
                }
                for (index, child) in prefix.iter().enumerate() {
                    if !self.match_pattern(child, &values[index], scope, bindings,
                        &format!("{path}/prefix/{index}"), depth + 1)? {
                        return Ok(false);
                    }
                }
                let end = values.len() - suffix.len();
                for (index, child) in suffix.iter().enumerate() {
                    if !self.match_pattern(child, &values[end + index], scope, bindings,
                        &format!("{path}/suffix/{index}"), depth + 1)? {
                        return Ok(false);
                    }
                }
                if let Some(name) = rest.filter(|name| *name != "_") {
                    self.allocate(end - prefix.len(), path)?;
                    insert(bindings, name, Value::List(values[prefix.len()..end].to_vec().into()))?;
                }
                true
            }
            "record" => {
                let Value::Record(values) = value else { return Ok(false) };
                let fields = pattern["fields"].as_object().ok_or_else(|| {
                    Error::new("invalid_pattern", path, "Record pattern fields must be an object.")
                })?;
                if pattern["exact"] == true && values.len() != fields.len() {
                    return Ok(false);
                }
                for (key, child) in fields {
                    let Some(field) = values.get(key) else { return Ok(false) };
                    if !self.match_pattern(child, field, scope, bindings,
                        &format!("{path}/fields/{key}"), depth + 1)? {
                        return Ok(false);
                    }
                }
                true
            }
            "type" => {
                let accepts = match (string(&pattern["name"]), value) {
                    ("int", Value::Numeric(n)) => n.dimension == units::SCALAR && n.value.fract() == 0.0,
                    ("f32" | "f64", Value::Numeric(n)) => n.dimension == units::SCALAR,
                    ("length", Value::Numeric(n)) => n.dimension == units::LENGTH,
                    ("angle", Value::Numeric(n)) => n.dimension == units::ANGLE,
                    ("str", Value::Text(_)) | ("list", Value::List(_)) | ("record", Value::Record(_)) => true,
                    _ => false,
                };
                accepts && self.match_pattern(&pattern["pattern"], value, scope, bindings,
                    &format!("{path}/pattern"), depth + 1)?
            }
            kind => return Err(Error::new("invalid_pattern", path, format!("Unknown pattern kind {kind}."))),
        })
    }
}

#[cfg(test)]
mod match_tests {
    use super::*;

    fn run(expr: &Json) -> Result<Value<'_>> {
        static DOCUMENT: std::sync::OnceLock<Json> = std::sync::OnceLock::new();
        let document = DOCUMENT.get_or_init(|| json!({"parameters": [], "functions": []}));
        Evaluator::new(document)?.resolve(expr, &Scope::default(), "/match", 0)
    }

    #[test]
    fn matches_literals_without_cross_kind_or_unit_errors() {
        let mut expr = json!({"op":"match","input":{"op":"quantity","value":2,"unit":"cm"},"arms":[
            {"pattern":{"kind":"literal","value":{"op":"text","value":"20"}},"body":{"local":"bad"}},
            {"pattern":{"kind":"literal","value":20},"body":{"local":"bad"}},
            {"pattern":{"kind":"literal","value":{"op":"quantity","value":20,"unit":"mm"}},"body":7},
            {"pattern":{"kind":"wildcard"},"body":{"local":"bad"}}
        ]});
        assert_eq!(numeric(&run(&expr).unwrap(), "/").unwrap(), 7.0);
        expr["input"] = json!({"op":"text","value":"20"});
        expr["arms"][0]["body"] = json!(9);
        assert_eq!(numeric(&run(&expr).unwrap(), "/").unwrap(), 9.0);
    }

    #[test]
    fn guards_and_alternatives_isolate_partial_bindings() {
        let expr = json!({"op":"match","input":{"op":"list","items":[4,2]},"arms":[
            {"pattern":{"kind":"literal","value":1},"guard":{"local":"never"},"body":{"local":"never"}},
            {"pattern":{"kind":"bind","name":"discarded"},"guard":0,"body":{"local":"never"}},
            {"pattern":{"kind":"or","patterns":[
                {"kind":"list","prefix":[{"kind":"bind","name":"x"},{"kind":"literal","value":1}],"suffix":[]},
                {"kind":"list","prefix":[{"kind":"literal","value":4},{"kind":"bind","name":"x"}],"suffix":[]}
            ]},"guard":{"op":"eq","args":[{"local":"x"},2]},"body":{"local":"x"}}
        ]});
        assert_eq!(numeric(&run(&expr).unwrap(), "/").unwrap(), 2.0);
    }

    #[test]
    fn destructures_nested_tagged_records_and_list_rest() {
        let expr = json!({"op":"match","input":{"op":"record","fields":{
            "kind":{"op":"text","value":"points"},"items":{"op":"list","items":[1,3,5,7]},"extra":10
        }},"arms":[
            {"pattern":{"kind":"record","exact":true,"fields":{"kind":{"kind":"wildcard"}}},"body":{"local":"bad"}},
            {"pattern":{"kind":"as","name":"all","pattern":{"kind":"record","exact":false,"fields":{
                "kind":{"kind":"literal","value":{"op":"text","value":"points"}},
                "items":{"kind":"list","prefix":[{"kind":"literal","value":1}],"suffix":[{"kind":"bind","name":"last"}],"rest":"middle"}
            }}},"body":{"op":"list","items":[{"op":"length","input":{"local":"middle"}},{"local":"last"},{"op":"field","input":{"local":"all"},"name":"extra"}]}}
        ]});
        let value = run(&expr).unwrap();
        let items = sequence(&value, "/").unwrap();
        assert_eq!(items.iter().map(|n| numeric(n,"/").unwrap()).collect::<Vec<_>>(), vec![2.0,7.0,10.0]);
    }

    #[test]
    fn list_shapes_and_missing_record_fields_are_plain_mismatches() {
        for subject in [json!({"op":"list","items":[1]}),json!({"op":"record","fields":{}}),json!(5)] {
            let expr = json!({"op":"match","input":subject,"arms":[
                {"pattern":{"kind":"list","prefix":[{"kind":"wildcard"}],"suffix":[{"kind":"wildcard"}],"rest":"_"},"body":{"local":"bad"}},
                {"pattern":{"kind":"record","exact":false,"fields":{"missing":{"kind":"wildcard"}}},"body":{"local":"bad"}},
                {"pattern":{"kind":"wildcard"},"body":8}
            ]});
            assert_eq!(numeric(&run(&expr).unwrap(), "/").unwrap(),8.0);
        }
    }

    #[test]
    fn ranges_check_boundaries_and_units_without_coercion() {
        let mut expr = json!({"op":"match","input":{"op":"quantity","value":20,"unit":"mm"},"arms":[
            {"pattern":{"kind":"range","start":0,"end":100,"inclusive":true},"body":{"local":"bad"}},
            {"pattern":{"kind":"range","start":{"op":"quantity","value":1,"unit":"cm"},"end":{"op":"quantity","value":2,"unit":"cm"},"inclusive":false},"body":1},
            {"pattern":{"kind":"wildcard"},"body":2}
        ]});
        assert_eq!(numeric(&run(&expr).unwrap(),"/").unwrap(),2.0);
        expr["arms"][1]["pattern"]["inclusive"] = json!(true);
        assert_eq!(numeric(&run(&expr).unwrap(),"/").unwrap(),1.0);
    }

    #[test]
    fn type_patterns_only_accept_their_runtime_value_categories() {
        let subjects = [json!(2),json!(2.5),json!({"op":"text","value":"x"}),json!({"op":"list","items":[]}),json!({"op":"record","fields":{}}),json!({"op":"quantity","value":2,"unit":"mm"}),json!({"op":"quantity","value":2,"unit":"deg"})];
        let kinds = ["int","f32","f64","str","list","record","length","angle"];
        for (index, subject) in subjects.iter().enumerate() {
            for kind in kinds {
                let expr = json!({"op":"match","input":subject,"arms":[
                    {"pattern":{"kind":"type","name":kind,"pattern":{"kind":"bind","name":"x"}},"body":1},
                    {"pattern":{"kind":"wildcard"},"body":0}
                ]});
                let expected = matches!((index,kind),(0,"int"|"f32"|"f64")|(1,"f32"|"f64")|(2,"str")|(3,"list")|(4,"record")|(5,"length")|(6,"angle"));
                assert_eq!(numeric(&run(&expr).unwrap(),"/").unwrap(),f64::from(expected),"{index}: {kind}");
            }
        }
    }

    #[test]
    fn evaluates_subject_once_and_reports_missing_case_with_path() {
        let document = json!({"parameters":[],"functions":[]});
        let expr = json!({"op":"match","input":{"op":"list","items":[1,2,3]},"arms":[
            {"pattern":{"kind":"literal","value":1},"body":0},
            {"pattern":{"kind":"literal","value":2},"body":0},
            {"pattern":{"kind":"bind","name":"x"},"guard":0,"body":0}
        ]});
        let mut eval = Evaluator::new(&document).unwrap();
        let error = eval.resolve(&expr,&Scope::default(),"/selected",0).unwrap_err();
        assert_eq!(eval.allocated,3);
        assert_eq!((error.code.as_str(),error.path.as_str()),("non_exhaustive_match","/selected"));
    }
}

impl<'a> Evaluator<'a> {
    fn check_value_type(
        &mut self,
        value: Value<'a>,
        descriptor: &Json,
        path: &str,
        depth: usize,
    ) -> Result<Value<'a>> {
        self.tick(depth, path)?;
        let name = string(&descriptor["name"]);
        let wrong_type = || Error::new("type_error", path, format!("Expected {name}."));
        match name {
            "int" | "f32" | "f64" | "length" | "angle" => {
                Ok(units::check_type(numeric_value(&value, path)?, name, path)?.into())
            }
            "str" => match value {
                Value::Text(_) => Ok(value),
                _ => Err(wrong_type()),
            },
            "Vec" => {
                let items = sequence(&value, path)?;
                let element = &descriptor["args"][0];
                self.allocate(items.len(), path)?;
                let mut checked = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    checked.push(self.check_value_type(item.clone(), element, &format!("{path}/{index}"), depth + 1)?);
                }
                Ok(Value::List(checked.into()))
            }
            _ => {
                let (Value::Record(values), Some(fields)) = (&value, descriptor["fields"].as_object()) else {
                    return Err(wrong_type());
                };
                if values.len() != fields.len() || fields.keys().any(|key| !values.contains_key(key)) {
                    return Err(Error::new("type_error", path, format!("Expected exactly the declared fields of {name}.")));
                }
                self.allocate(fields.len(), path)?;
                let mut checked = HashMap::with_capacity(fields.len());
                for (key, field) in fields {
                    checked.insert(key.clone(), self.check_value_type(values[key].clone(), field, &format!("{path}/{key}"), depth + 1)?);
                }
                Ok(Value::Record(Rc::new(checked)))
            }
        }
    }
}

#[cfg(test)]
mod typed_value_tests {
    use super::*;
    fn run(expr: &Json) -> Result<Value<'_>> {
        static DOCUMENT: std::sync::OnceLock<Json> = std::sync::OnceLock::new();
        let document = DOCUMENT.get_or_init(|| json!({"parameters": [], "functions": []}));
        Evaluator::new(document)?.resolve(expr, &Scope::default(), "/typed", 0)
    }

    #[test]
    fn validates_dynamic_strings_vectors_and_exact_named_records() {
        let subject = json!({"op":"match","input":2,"arms":[{"pattern":{"kind":"bind","name":"r"},"body":{"op":"record","fields":{
            "label":{"op":"text","value":"point"},"values":{"op":"list","items":[{"local":"r"},3]}
        }}}]});
        let descriptor = json!({"name":"PointSet","args":[{"name":"int"}],"fields":{
            "label":{"name":"str"},"values":{"name":"Vec","args":[{"name":"int"}]}
        }});
        let mut expr = json!({"op":"typed_value","value":subject,"type":descriptor});
        let Value::Record(record) = run(&expr).unwrap() else { panic!("expected record") };
        assert!(matches!(record["label"],Value::Text("point")));
        assert_eq!(sequence(&record["values"],"/").unwrap().len(),2);
        expr["type"]["fields"]["label"] = json!({"name":"int"});
        assert_eq!(run(&expr).unwrap_err().path,"/typed/label");
        expr["type"]["fields"]["label"] = json!({"name":"str"});
        expr["type"]["fields"]["missing"] = json!({"name":"int"});
        assert_eq!(run(&expr).unwrap_err().code,"type_error");
    }

    #[test]
    fn recursively_coerces_f32_and_preserves_unit_checks() {
        let expr = json!({"op":"typed_value","type":{"name":"Vec","args":[{"name":"f32"}]},"value":{"op":"list","items":[0.1]}});
        let values = run(&expr).unwrap();
        assert_eq!(numeric(&sequence(&values,"/").unwrap()[0],"/").unwrap(),0.1f32 as f64);
        for (name, value, pass) in [
            ("length",json!({"op":"quantity","value":2,"unit":"cm"}),true),
            ("angle",json!({"op":"quantity","value":2,"unit":"cm"}),false),
            ("int",json!(2.5),false),
            ("str",json!(2),false),
            ("f64",json!({"op":"text","value":"2"}),false),
        ] {
            let expr = json!({"op":"typed_value","value":value,"type":{"name":name}});
            assert_eq!(run(&expr).is_ok(),pass,"{name}");
        }
        let expr = json!({"op":"typed_value","value":{"op":"list","items":[1,2.5]},"type":{"name":"Vec","args":[{"name":"int"}]}});
        assert_eq!(run(&expr).unwrap_err().path,"/typed/1");
    }
}
