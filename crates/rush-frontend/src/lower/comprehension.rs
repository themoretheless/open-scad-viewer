use super::*;

impl Compiler {
    pub(super) fn comprehension(&mut self, a: &J, offset: usize, outer: &Env, b: usize) -> R<V> {
        if b > 64 {
            return Err(crate::error("Function expansion exceeds 64"));
        }
        let clauses = arr(a, "items");
        let clause = &clauses[offset];
        if s(clause, "kind") != "for" {
            return Err(crate::error("Generator must start with for"));
        }
        let mut input = list(self.eval(&clause["left"], outer, b)?)?;
        self.serial += 1;
        let binding = format!("v{}", self.serial);
        let mut scope = outer.clone();
        for (i, n) in arr(clause, "items").iter().enumerate() {
            scope.insert(
                s(n, "value").into(),
                V::Json(if arr(clause, "items").len() == 1 {
                    json!({"local":&binding})
                } else {
                    json!({"op":"at","input":{"local":&binding},"index":i})
                }),
            );
        }
        let mut next = offset + 1;
        let mut steps = Vec::new();
        let mut guarded = false;

        while next < clauses.len() && s(&clauses[next], "kind") != "for" {
            let c = &clauses[next];
            next += 1;
            if ["assert", "validate"].contains(&s(c, "kind")) {
                let check = self.local_check(s(c, "kind"), &c["left"], &scope)?;
                guarded = true;
                steps.push(json!({"kind":"assert","value":check}));
            } else if s(c, "kind") == "let" {
                let v = self.eval(&c["left"], &scope, b)?;
                let mut checks = Vec::new();
                self.gather_effects(&v, &mut checks);
                let pattern = c
                    .get("pattern")
                    .cloned()
                    .unwrap_or_else(|| json!({"kind":"name","value":s(&c["items"][0],"value")}));
                for name in binding_names(&pattern) {
                    scope.shift_remove(name);
                }
                checks.extend(self.bind(&pattern, v, &mut scope)?);
                if !checks.is_empty() {
                    guarded = true;
                    steps.push(
                        json!({"kind":"assert","value":{"op":"checked","checks":checks,"value":1}}),
                    );
                }
            } else {
                let v = scalar(self.eval(&c["left"], &scope, b)?)?;
                guarded |= s(c, "kind") == "while";
                steps.push(json!({"kind":s(c,"kind"),"value":v}));
            }
        }
        if guarded {
            // One ordered pass keeps assertions before/after continue and break
            // in their lexical positions, including on non-yielding iterations.
            input = json!({"op":"guarded","input":input,"binding":&binding,"steps":steps});
        } else {
            for step in steps {
                input = json!({"op":"filter","input":input,"function":{"op":"lambda","parameters":[&binding],"body":&step["value"]}});
            }
        }
        let child = if next < clauses.len() {
            self.comprehension(a, next, &scope, b + 1)?
        } else {
            self.eval(&a["left"], &scope, b)?
        };
        if is_geometry(&child) {
            let id = self.geometry(child)?;
            let result =
                self.add(json!({"op":"collect","values":input,"binding":binding,"input":id}))?;
            let id = self.geometry(result.clone())?;
            self.collections.insert(id);
            return Ok(result);
        }
        Ok(V::Json(
            json!({"sequence":{"op":if next<clauses.len(){"flatmap"}else{"map"},"input":input,"function":{"op":"lambda","parameters":[&binding],"body":expression(child)?}}}),
        ))
    }
}
