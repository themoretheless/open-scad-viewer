// Structural patterns have their own grammar: identifiers bind rather than
// reading variables, and | is an alternative rather than boolean disjunction.
impl Parser<'_> {
    fn match_pattern(&mut self, depth: usize) -> R<J> {
        if depth > 16 {
            return self.err("Pattern nesting exceeds 16");
        }
        let first = self.match_atom(depth + 1)?;
        if self.peek() != "|" {
            return Ok(first);
        }
        let mut patterns = vec![first];
        while self.peek() == "|" {
            self.pop()?;
            patterns.push(self.match_atom(depth + 1)?);
            if patterns.len() > 32 {
                return self.err("At most 32 pattern alternatives");
            }
        }
        Ok(json!({"kind":"or","patterns":patterns}))
    }
    fn match_literal(&mut self) -> R<J> {
        let negative = self.peek() == "-";
        if negative {
            self.pop()?;
        }
        let token = self.pop()?;
        if token.starts_with('"') && !negative {
            return Ok(
                json!({"kind":"string","value":value_codec::from_str::<J>(&token).map_err(|e| crate::error(e.to_string()))?}),
            );
        }
        let token = if negative { format!("-{token}") } else { token };
        number(&token).map_err(|_| crate::error(format!("Expected a pattern literal, got {token}")))?;
        Ok(json!({"kind":"number","value":token}))
    }
    fn match_atom(&mut self, depth: usize) -> R<J> {
        if depth > 16 {
            return self.err("Pattern nesting exceeds 16");
        }
        match self.peek() {
            "_" => {
                self.pop()?;
                Ok(json!({"kind":"wildcard"}))
            }
            "(" => {
                self.pop()?;
                self.inner();
                let p = self.match_pattern(depth + 1)?;
                self.inner();
                self.take(")")?;
                Ok(p)
            }
            "[" => {
                self.pop()?;
                self.inner();
                let mut prefix = Vec::new();
                let mut suffix = Vec::new();
                let mut rest = None;
                while self.peek() != "]" {
                    if self.peek() == ".." {
                        if rest.is_some() {
                            return self.err("Only one list rest pattern is allowed");
                        }
                        self.pop()?;
                        rest = Some(if ident(self.peek()) && self.peek() != "EOF" {
                            self.id()?
                        } else {
                            "_".into()
                        });
                    } else {
                        let p = self.match_pattern(depth + 1)?;
                        if rest.is_some() {
                            suffix.push(p);
                        } else {
                            prefix.push(p);
                        }
                    }
                    if prefix.len() + suffix.len() > 256 {
                        return self.err("List pattern exceeds 256 elements");
                    }
                    self.inner();
                    if self.peek() != "," {
                        break;
                    }
                    self.pop()?;
                    self.inner();
                }
                self.take("]")?;
                let mut result = json!({"kind":"list","prefix":prefix,"suffix":suffix});
                if let Some(rest) = rest {
                    result["rest"] = json!(rest);
                }
                Ok(result)
            }
            "{" => {
                self.pop()?;
                self.inner();
                let mut fields = value_codec::Map::new();
                let mut exact = false;
                while self.peek() != "}" {
                    if ["!", ".."].contains(&self.peek()) {
                        exact = self.pop()? == "!";
                        self.inner();
                        if self.peek() == "," {
                            self.pop()?;
                            self.inner();
                        }
                        break;
                    }
                    let name = self.id()?;
                    let p = if self.peek() == ":" {
                        self.pop()?;
                        self.inner();
                        self.match_pattern(depth + 1)?
                    } else {
                        json!({"kind":"bind","name":&name})
                    };
                    if fields.insert(name.clone(), p).is_some() {
                        return self.err(format!("Duplicate pattern field {name}"));
                    }
                    if fields.len() > 32 {
                        return self.err("At most 32 record pattern fields");
                    }
                    self.inner();
                    if self.peek() != "," {
                        break;
                    }
                    self.pop()?;
                    self.inner();
                }
                self.take("}")?;
                Ok(json!({"kind":"record","fields":J::Object(fields),"exact":exact}))
            }
            "true" | "false" => {
                let value = if self.pop()? == "true" { "1" } else { "0" };
                Ok(json!({"kind":"literal","value":{"kind":"number","value":value}}))
            }
            token if ident(token) && token != "EOF" => {
                let name = self.id()?;
                if self.peek() == "@" {
                    self.pop()?;
                    Ok(json!({"kind":"as","name":name,"pattern":self.match_atom(depth+1)?}))
                } else if self.peek() == "(" {
                    if ![
                        "int", "f32", "f64", "str", "length", "angle", "list", "record",
                    ]
                    .contains(&name.as_str())
                    {
                        return self.err(format!("Unknown pattern type {name}"));
                    }
                    self.pop()?;
                    self.inner();
                    let p = self.match_pattern(depth + 1)?;
                    self.inner();
                    self.take(")")?;
                    Ok(json!({"kind":"type","name":name,"pattern":p}))
                } else {
                    Ok(json!({"kind":"bind","name":name}))
                }
            }
            _ => {
                let start = self.match_literal()?;
                if ["..", "..<"].contains(&self.peek()) {
                    let inclusive = self.pop()? == "..";
                    let end = self.match_literal()?;
                    if start["kind"] != "number" || end["kind"] != "number" {
                        return self.err("Range patterns require numeric endpoints");
                    }
                    Ok(json!({"kind":"range","start":start,"end":end,"inclusive":inclusive}))
                } else {
                    Ok(json!({"kind":"literal","value":start}))
                }
            }
        }
    }
    fn match_arm_block(&mut self, arm_indent: &str) -> R<J> {
        let indent = self.indent();
        let mut items = Vec::new();
        loop {
            if self.peek() == "EOF" || self.indent() != indent {
                return self.err("Expected ret at match body indentation");
            }
            if self.peek() == "ret" {
                self.pop()?;
                self.continuation(&indent, "Expected indented return expression")?;
                let result = self.expr(0)?;
                return Ok(json!({"kind":"match_block","items":items,"left":result}));
            }
            if indent.len() <= arm_indent.len() {
                return self.err("Expected indented match body");
            }
            if ["assert", "validate"].contains(&self.peek()) {
                items.push(self.check_statement()?);
            } else {
                let pattern = self.binding()?;
                self.take("=")?;
                if pattern["kind"] == "name" {
                    items.push(json!({"name":&pattern["value"],"value":self.expr(0)?}));
                } else {
                    items.push(json!({"pattern":pattern,"value":self.expr(0)?}));
                }
            }
            if items.len() > 64 {
                return self.err("At most 64 match body statements");
            }
            self.take("\n")?;
            self.inner();
        }
    }
    fn match_expression(&mut self) -> R<J> {
        let base = self.indent();
        let subject = self.expr(0)?;
        self.take("\n")?;
        self.inner();
        let indent = self.indent();
        if indent.len() <= base.len() || !indent.starts_with(&base) {
            return self.err("Expected indented match arms");
        }
        let mut arms = Vec::new();
        let mut patterns = Vec::new();
        let mut exhaustive = false;
        loop {
            if self.peek() == "EOF" || self.indent() != indent {
                return self.err("Match arms must have matching indentation");
            }
            if exhaustive {
                return self.err("Unreachable match arm after an unconditional catch-all");
            }
            let pattern = self.match_pattern(0)?;
            let guard = if self.peek() == "where" {
                self.pop()?;
                Some(self.expr(2)?)
            } else {
                None
            };
            // Identical guarded patterns can intentionally fall through.
            if patterns.contains(&pattern) {
                return self.err("Unreachable duplicate match pattern");
            }
            if guard.is_none() {
                patterns.push(pattern.clone());
                exhaustive = match_catch_all(&pattern);
            }
            self.take("=>")?;
            let multiline = self
                .continuation(&indent, "Expected indented match result")?
                .is_some();
            let previous_indent = self.match_indent.replace(indent.len());
            let result = if multiline
                && (["ret", "assert", "validate"].contains(&self.peek()) || self.next() == "=" || self.destructuring_ahead())
            {
                self.match_arm_block(&indent)
            } else {
                self.expr(0)
            };
            self.match_indent = previous_indent;
            let result = result?;
            let mut arm = json!({"pattern":pattern,"result":result});
            if let Some(guard) = guard {
                arm["guard"] = guard;
            }
            arms.push(arm);
            if arms.len() > 32 {
                return self.err("At most 32 match arms");
            }
            let end = self.p;
            if [")", "]", "}", ",", ";", ":"].contains(&self.peek()) {
                break;
            }
            if !["\n", "EOF"].contains(&self.peek()) {
                return self.err("Expected newline after match arm");
            }
            self.inner();
            if self.peek() == "EOF" || self.indent().len() < indent.len() {
                self.p = end;
                break;
            }
        }
        Ok(json!({"kind":"match","left":subject,"items":arms}))
    }
}
fn match_catch_all(p: &J) -> bool {
    match p["kind"].as_str().unwrap_or("") {
        "wildcard" | "bind" => true,
        "as" => match_catch_all(&p["pattern"]),
        "or" => p["patterns"]
            .as_array()
            .is_some_and(|ps| ps.iter().any(match_catch_all)),
        _ => false,
    }
}
