// Trait contracts are resolved during bounded monomorphization. Method bodies
// retain their declaration environment and share the normal function evaluator.
impl Compiler {
    fn constrain_receiver(&self, value: V, declared: &J, constraints: &J, depth: usize) -> R<V> {
        if depth > 16 {
            return Err("Receiver type nesting exceeds 16".into());
        }
        let bounds = arr(constraints, s(declared, "name"))
            .iter()
            .filter_map(J::as_str)
            .map(str::to_owned)
            .collect();
        match value {
            V::Record(mut fields, nominal, marker, _) => {
                if let Some(def) = self.structs.get(s(declared, "name")) {
                    let substitutions: Types = arr(def, "generics")
                        .iter()
                        .zip(arr(declared, "args"))
                        .map(|(g, t)| (g.as_str().unwrap().into(), t.clone()))
                        .collect();
                    for field in arr(def, "fields") {
                        let key = s(field, "name");
                        if let Some(value) = fields.shift_remove(key) {
                            fields.insert(
                                key.into(),
                                self.constrain_receiver(
                                    value,
                                    &substitute(&field["type"], &substitutions, 0)?,
                                    constraints,
                                    depth + 1,
                                )?,
                            );
                        }
                    }
                }
                Ok(V::Record(fields, nominal, marker, bounds))
            }
            V::Array(values) if s(declared, "name") == "Vec" => values
                .into_iter()
                .map(|v| self.constrain_receiver(v, &declared["args"][0], constraints, depth + 1))
                .collect::<R<Vec<_>>>()
                .map(V::Array),
            value => Ok(value),
        }
    }
    fn associated_names(&self, def: &J, allowed: &mut Set<String>) -> R<()> {
        for generic in arr(def, "generics") {
            let generic = generic.as_str().ok_or("Invalid generic name")?;
            for bound in arr(&def["bounds"], generic) {
                let bound = bound.as_str().ok_or("Invalid trait name")?;
                let contract = self
                    .traits
                    .get(bound)
                    .ok_or_else(|| format!("Unknown trait {bound}"))?;
                for associated in arr(contract, "associated") {
                    allowed.insert(format!("{generic}.{}", s(associated, "name")));
                }
            }
        }
        Ok(())
    }
    fn validate_trait(&self, def: &J) -> R<()> {
        let mut allowed = Set::from(["Self".into()]);
        for associated in arr(def, "associated") {
            allowed.insert(format!("Self.{}", s(associated, "name")));
        }
        for field in arr(def, "fields") {
            self.validate(&field["type"], &allowed, &Set::new())?;
        }
        for method in arr(def, "methods") {
            self.validate_receiver(method)?;
            for field in arr(method, "inputs").iter().chain(arr(method, "outputs")) {
                self.validate(&field["type"], &allowed, &Set::new())?;
            }
        }
        Ok(())
    }
    fn validate_receiver(&self, method: &J) -> R<()> {
        let inputs = arr(method, "inputs");
        if inputs.first().is_none_or(|input| {
            s(input, "name") != "self"
                || typename(&input["type"]) != "Self"
                || input.get("default").is_some()
        }) {
            return Err(format!(
                "Method {} must start with self: Self",
                s(method, "name")
            ));
        }
        if !arr(method, "generics").is_empty() {
            return Err("Method generic parameters are not supported".into());
        }
        Ok(())
    }
    fn impl_key(bound: &str, target: &J) -> String {
        format!("{bound}|{}", typename(target))
    }
    fn trait_bindings(&self, bound: &str, target: &J) -> R<Types> {
        let contract = self
            .traits
            .get(bound)
            .ok_or_else(|| format!("Unknown trait {bound}"))?;
        let implementation = self.implementations.get(&Self::impl_key(bound, target));
        if implementation.is_none()
            && (!arr(contract, "associated").is_empty()
                || arr(contract, "methods").iter().any(|m| flag(m, "required")))
        {
            return Err(format!("{} requires impl {bound}", typename(target)));
        }
        let mut bindings = Types::from_iter([("Self".into(), target.clone())]);
        if let Some((implementation, _)) = implementation {
            for associated in arr(implementation, "associated") {
                bindings.insert(
                    format!("Self.{}", s(associated, "name")),
                    associated["type"].clone(),
                );
            }
        }
        // A record always remains its concrete type; traits only constrain it.
        let fields = self
            .type_fields(target)
            .map_err(|_| format!("trait {bound} requires a record type"))?;
        for field in arr(contract, "fields") {
            let key = s(field, "name");
            let actual = fields
                .get(key)
                .ok_or_else(|| format!("trait {bound} requires field {key}"))?;
            let expected = substitute(&field["type"], &bindings, 0)?;
            let widening =
                ["int", "f32"].contains(&s(actual, "name")) && s(&expected, "name") == "f64";
            if typename(actual) != typename(&expected) && !widening {
                return Err(format!(
                    "{key}: trait {bound} expects {}, got {}",
                    typename(&expected),
                    typename(actual)
                ));
            }
        }
        Ok(bindings)
    }
    fn resolve_function_bounds(&self, def: &J, bindings: &mut Types) -> R<()> {
        for generic in arr(def, "generics") {
            let generic = generic.as_str().ok_or("Invalid generic name")?;
            let Some(target) = bindings.get(generic).cloned() else {
                continue;
            };
            for bound in arr(&def["bounds"], generic) {
                let bound = bound.as_str().ok_or("Invalid trait name")?;
                let resolved = self.trait_bindings(bound, &target)?;
                for (key, value) in resolved {
                    if let Some(member) = key.strip_prefix("Self.") {
                        let key = format!("{generic}.{member}");
                        if bindings
                            .get(&key)
                            .is_some_and(|old| typename(old) != typename(&value))
                        {
                            return Err(format!("Ambiguous associated type {key}"));
                        }
                        bindings.insert(key, value);
                    }
                }
            }
        }
        Ok(())
    }
    fn resolve_associated(
        t: &J,
        target: &J,
        implementation: &J,
        visiting: &mut Set<String>,
        depth: usize,
    ) -> R<J> {
        if depth > 16 {
            return Err("Associated type nesting exceeds 16".into());
        }
        let name = s(t, "name");
        if (name == "Self" || name.starts_with("Self.")) && !arr(t, "args").is_empty() {
            return Err("Self and associated types take no type arguments".into());
        }
        if name == "Self" {
            return Ok(target.clone());
        }
        if let Some(member) = name.strip_prefix("Self.") {
            if !visiting.insert(member.into()) {
                return Err(format!("Cyclic associated type {member}"));
            }
            let definition = arr(implementation, "associated")
                .iter()
                .find(|a| s(a, "name") == member)
                .ok_or_else(|| format!("Unknown associated type {member}"))?;
            let value = Self::resolve_associated(
                &definition["type"],
                target,
                implementation,
                visiting,
                depth + 1,
            )?;
            visiting.remove(member);
            return Ok(value);
        }
        Ok(ty(
            name,
            arr(t, "args")
                .iter()
                .map(|t| Self::resolve_associated(t, target, implementation, visiting, depth + 1))
                .collect::<R<Vec<_>>>()?,
        ))
    }
    fn register_impl(&mut self, original: &J, env: &Env) -> R<()> {
        let bound = s(original, "name");
        let target = &original["target"];
        self.validate(target, &Set::new(), &Set::new())?;
        if !self.structs.contains_key(s(target, "name")) {
            return Err("impl requires a named structure type".into());
        }
        let contract = self
            .traits
            .get(bound)
            .cloned()
            .ok_or_else(|| format!("Unknown trait {bound}"))?;
        let key = Self::impl_key(bound, target);
        if self.implementations.contains_key(&key) {
            return Err(format!("Duplicate impl {bound} for {}", typename(target)));
        }
        let mut implementation = original.clone();
        let mut types = Types::from_iter([("Self".into(), target.clone())]);
        if arr(original, "associated").len() != arr(&contract, "associated").len() {
            return Err(format!(
                "impl {bound}: associated types must match the trait"
            ));
        }
        for associated in arr(original, "associated") {
            let name = s(associated, "name");
            if !arr(&contract, "associated")
                .iter()
                .any(|a| s(a, "name") == name)
            {
                return Err(format!("Unknown associated type {name}"));
            }
            let value = Self::resolve_associated(
                &associated["type"],
                target,
                original,
                &mut Set::from([name.into()]),
                0,
            )?;
            self.validate(&value, &Set::new(), &Set::new())?;
            types.insert(format!("Self.{name}"), value);
        }
        let mut methods = Vec::new();
        for method in arr(original, "methods") {
            let name = s(method, "name");
            let expected = arr(&contract, "methods")
                .iter()
                .find(|m| s(m, "name") == name)
                .ok_or_else(|| format!("Unknown trait method {name}"))?;
            self.validate_receiver(method)?;
            if arr(method, "inputs").len() != arr(expected, "inputs").len() {
                return Err(format!("Method {name}: input count differs from trait"));
            }
            let mut method = method.clone();
            let mut inputs = Vec::new();
            for (actual, expected) in arr(&method, "inputs").iter().zip(arr(expected, "inputs")) {
                if s(actual, "name") != s(expected, "name")
                    || typename(&substitute(&actual["type"], &types, 0)?)
                        != typename(&substitute(&expected["type"], &types, 0)?)
                {
                    return Err(format!(
                        "Method {name}: incompatible parameter {}",
                        s(expected, "name")
                    ));
                }
                let mut input = actual.clone();
                if input.get("default").is_none() {
                    if let Some(value) = expected.get("default") {
                        input["default"] = value.clone();
                    }
                }
                inputs.push(input);
            }
            if !flag(&method, "inferResult") && arr(&method, "outputs") != arr(expected, "outputs")
            {
                if arr(&method, "outputs").len() != 1
                    || typename(&substitute(&method["outputs"][0]["type"], &types, 0)?)
                        != typename(&substitute(&expected["outputs"][0]["type"], &types, 0)?)
                {
                    return Err(format!("Method {name}: incompatible result type"));
                }
            }
            method["inputs"] = json!(inputs);
            method["outputs"] = expected["outputs"].clone();
            method["inferResult"] = json!(false);
            method["singleResult"] = json!(true);
            methods.push(method);
        }
        for method in arr(&contract, "methods") {
            if flag(method, "required")
                && !methods.iter().any(|m| s(m, "name") == s(method, "name"))
            {
                return Err(format!(
                    "impl {bound}: missing method {}",
                    s(method, "name")
                ));
            }
        }
        implementation["methods"] = json!(methods);
        // Store resolved associated types, including Self in concrete generic targets.
        implementation["associated"] = json!(arr(original,"associated").iter().map(|a|json!({"name":s(a,"name"),"type":&types[format!("Self.{}",s(a,"name")).as_str()]})).collect::<Vec<_>>());
        self.implementations
            .insert(key.clone(), (implementation, env.clone()));
        if let Err(error) = self.trait_bindings(bound, target) {
            self.implementations.shift_remove(&key);
            return Err(error);
        }
        Ok(())
    }
    fn invoke_method(
        &mut self,
        receiver: V,
        call: &J,
        caller: &Env,
        depth: usize,
        qualified: Option<&str>,
    ) -> R<V> {
        let name = s(call, "value");
        let target = self.infer_type(&receiver)?;
        let mut candidates = Vec::new();
        for (bound, contract) in &self.traits {
            if arr(contract, "methods")
                .iter()
                .any(|m| s(m, "name") == name)
                && self.trait_bindings(bound, &target).is_ok()
            {
                candidates.push(bound.clone());
            }
        }
        if let Some(bound) = qualified {
            self.trait_bindings(bound, &target)?;
            candidates.retain(|candidate| candidate == bound);
        } else if let V::Record(_, _, _, bounds) = &receiver {
            let constrained: Vec<_> = candidates
                .iter()
                .filter(|candidate| bounds.contains(candidate))
                .cloned()
                .collect();
            if !constrained.is_empty() {
                candidates = constrained;
            }
        }
        if candidates.len() != 1 {
            return Err(if candidates.is_empty() {
                format!(
                    "No applicable trait method {name} for {}",
                    typename(&target)
                )
            } else {
                format!("Ambiguous trait method {name}: {}", candidates.join(", "))
            });
        }
        let bound = &candidates[0];
        let types = self.trait_bindings(bound, &target)?;
        let implementation = self.implementations.get(&Self::impl_key(bound, &target));
        let custom = implementation.and_then(|(def, env)| {
            arr(def, "methods")
                .iter()
                .find(|m| s(m, "name") == name)
                .map(|m| (m, env))
        });
        let (method, env) = custom.unwrap_or_else(|| {
            (
                arr(&self.traits[bound.as_str()], "methods")
                    .iter()
                    .find(|m| s(m, "name") == name)
                    .unwrap(),
                &self.trait_envs[bound.as_str()],
            )
        });
        let mut ast = method.clone();
        ast["traitOwner"] = json!(bound);
        let function = Function {
            ast,
            env: env.clone(),
            types,
        };
        self.serial += 1;
        let local = format!("receiver{}", self.serial);
        let mut scope = caller.clone();
        scope.insert(local.clone(), receiver);
        let mut args = arr(call, "args").to_vec();
        if args.iter().any(|a| s(a, "name") == "self") {
            return Err("Method receiver is supplied by the dot call".into());
        }
        let named = args.iter().any(|a| a.get("name").is_some());
        let mut first = json!({"value":{"kind":"name","value":local}});
        if named {
            first["name"] = json!("self");
        }
        args.insert(0, first);
        self.invoke(&function, &json!({"args":args}), &scope, depth + 1)
    }
}
