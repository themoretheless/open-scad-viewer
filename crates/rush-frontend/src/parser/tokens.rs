use super::*;

impl Parser<'_> {
    pub(super) fn peek(&self) -> &str {
        &self.tokens[self.p].text
    }
    pub(super) fn next(&self) -> &str {
        self.tokens
            .get(self.p + 1)
            .map_or("EOF", |t| t.text.as_str())
    }
    pub(super) fn err<T>(&self, m: impl AsRef<str>) -> R<T> {
        Err(crate::error(format!(
            "Rush line {}: {}",
            self.line(),
            m.as_ref()
        )))
    }
    pub(super) fn line(&self) -> usize {
        self.line_starts
            .partition_point(|start| *start <= self.tokens[self.p].start)
    }
    pub(super) fn pop(&mut self) -> R<String> {
        if self.peek() == "EOF" {
            return self.err("Unexpected end of source");
        }
        let s = self.peek().to_owned();
        self.p += 1;
        Ok(s)
    }
    pub(super) fn take(&mut self, s: &str) -> R<()> {
        if self.peek() != s {
            return self.err(format!("Expected {s}, got {}", self.peek()));
        }
        self.pop()?;
        Ok(())
    }
    pub(super) fn inner(&mut self) {
        while self.peek() == "\n" {
            self.p += 1
        }
    }
    pub(super) fn skip(&mut self) {
        while ["\n", ";"].contains(&self.peek()) {
            self.p += 1
        }
    }
    pub(super) fn commas(&mut self) {
        while ["\n", ",", ";"].contains(&self.peek()) {
            self.p += 1
        }
    }
    pub(super) fn id(&mut self) -> R<String> {
        let s = self.pop()?;
        if !ident(&s) {
            return self.err("Expected an identifier");
        }
        Ok(s)
    }
    pub(super) fn ty(&mut self, depth: usize) -> R<J> {
        if depth > 16 {
            return self.err("Type nesting exceeds 16");
        }
        let mut name = self.id()?;
        if self.peek() == "." {
            self.pop()?;
            name.push('.');
            name.push_str(&self.id()?);
        }
        let mut args = Vec::new();
        if self.peek() == "<" {
            self.pop()?;
            self.inner();
            loop {
                args.push(self.ty(depth + 1)?);
                self.inner();
                if self.peek() != "," {
                    break;
                }
                self.pop()?;
                self.inner();
                if self.peek() == ">" {
                    break;
                }
            }
            self.take(">")?
        }
        Ok(json!({"name":&name,"args":args}))
    }
    pub(super) fn generics(&mut self, open: &str, close: &str) -> R<Vec<String>> {
        let mut xs = Vec::new();
        if self.peek() == open {
            self.pop()?;
            self.inner();
            while self.peek() != close {
                xs.push(self.id()?);
                if xs.len() > 16 {
                    return self.err("At most 16 generic parameters");
                }
                self.inner();
                if self.peek() != "," {
                    break;
                }
                self.pop()?;
                self.inner()
            }
            self.take(close)?
        }
        if !unique(xs.clone()) {
            return self.err("Duplicate generic parameter");
        }
        Ok(xs)
    }
    pub(super) fn function_generics(&mut self, open: &str, close: &str) -> R<(Vec<String>, J)> {
        let mut names = Vec::new();
        let mut bounds = value_codec::Map::new();
        if self.peek() == open {
            self.pop()?;
            self.inner();
            while self.peek() != close {
                let name = self.id()?;
                if names.contains(&name) {
                    return self.err("Duplicate generic parameter");
                }
                names.push(name.clone());
                if names.len() > 16 {
                    return self.err("At most 16 generic parameters");
                }
                let mut required = Vec::new();
                if self.peek() == ":" {
                    self.pop()?;
                    loop {
                        self.inner();
                        let bound = self.id()?;
                        if required.contains(&bound) {
                            return self.err("Duplicate trait constraint");
                        }
                        required.push(bound);
                        if required.len() > 16 {
                            return self.err("At most 16 trait constraints");
                        }
                        if self.peek() != "+" {
                            break;
                        }
                        self.pop()?;
                    }
                }
                bounds.insert(name, json!(required));
                self.inner();
                if self.peek() != "," {
                    break;
                }
                self.pop()?;
                self.inner();
            }
            self.take(close)?;
        }
        Ok((names, J::Object(bounds)))
    }
    pub(super) fn trait_members(&mut self, implementation: bool) -> R<J> {
        self.inner();
        self.take("{")?;
        self.commas();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut associated = Vec::new();
        let mut names = Vec::new();
        while self.peek() != "}" {
            let base = self.indent();
            let kind = self.peek().to_owned();
            if kind == "fn" || kind == "type" {
                self.pop()?;
            }
            let name = self.id()?;
            if names.contains(&name) {
                return self.err(format!("Duplicate trait member {name}"));
            }
            names.push(name.clone());
            if names.len() > 64 {
                return self.err("At most 64 trait members");
            }
            match kind.as_str() {
                "type" => {
                    let mut entry = json!({"name":name});
                    if implementation {
                        self.take("=")?;
                        entry["type"] = self.ty(0)?;
                    }
                    associated.push(entry);
                }
                "fn" => {
                    let start = self.p;
                    let inputs = if self.next() == ":" {
                        self.layout_fields(&base, true)?
                    } else {
                        Vec::new()
                    };
                    let outputs = if self.peek() == "->" {
                        self.pop()?;
                        vec![json!({"name":"value","type":self.ty(0)?})]
                    } else {
                        Vec::new()
                    };
                    let end = self.p;
                    self.inner();
                    let body = ["ret", "=>"].contains(&self.peek())
                        || (self.p != end && self.indent().len() > base.len());
                    self.p = if body { start } else { end };
                    let mut method = if body {
                        self.layout(&base)?
                    } else {
                        json!({"kind":"function","generics":[],"inputs":inputs,"outputs":outputs,"singleResult":true,"required":true})
                    };
                    if !implementation && method["outputs"].as_array().is_none_or(Vec::is_empty) {
                        return self.err("Trait methods require an explicit result type");
                    }
                    if implementation && !body {
                        return self.err("impl methods require a body");
                    }
                    method["name"] = json!(name);
                    methods.push(method);
                }
                _ => {
                    if implementation {
                        return self.err("impl accepts only type and fn members");
                    }
                    self.take(":")?;
                    fields.push(json!({"name":name,"type":self.ty(0)?}));
                }
            }
            if self.peek() != "}" && !["\n", ",", ";"].contains(&self.peek()) {
                return self.err("Expected trait member separator");
            }
            self.commas();
        }
        self.take("}")?;
        Ok(json!({"fields":fields,"methods":methods,"associated":associated}))
    }
    pub(super) fn fields(&mut self, end: &str, alt: &str, defaults: bool) -> R<Vec<J>> {
        let mut xs = Vec::new();
        self.commas();
        while self.peek() != end
            && self.peek() != alt
            && self.peek() != "ret"
            && !(defaults && self.peek() == "{")
        {
            let name = self.id()?;
            self.take(":")?;
            let mut field = json!({"name":&name,"type":self.ty(0)?});
            if defaults && self.peek() == "=" {
                self.pop()?;
                field["default"] = self.expr(2)?;
            }
            xs.push(field);
            if xs.len() > 32 {
                return self.err("At most 32 fields or parameters");
            }
            if self.peek() != end
                && self.peek() != alt
                && self.peek() != "ret"
                && !(defaults && self.peek() == "{")
                && !["\n", ",", ";"].contains(&self.peek())
            {
                return self.err(format!("Expected a field separator or {end}"));
            }
            self.commas()
        }
        self.unique_fields(&xs)?;
        Ok(xs)
    }
    pub(super) fn unique_fields(&self, xs: &[J]) -> R<()> {
        if !unique(xs.iter().map(|x| x["name"].as_str().unwrap_or("").into())) {
            return self.err("Duplicate field or parameter");
        }
        Ok(())
    }
    pub(super) fn record(&mut self) -> R<J> {
        self.take("{")?;
        self.commas();
        let mut xs = Vec::new();
        while self.peek() != "}" {
            let name = self.id()?;
            let value = if self.peek() == ":" {
                self.pop()?;
                self.inner();
                self.expr(0)?
            } else {
                json!({"kind":"name","value":&name})
            };
            xs.push(json!({"name":&name,"value":value}));
            if xs.len() > 32 {
                return self.err("At most 32 record fields");
            }
            if self.peek() != "}" && !["\n", ",", ";"].contains(&self.peek()) {
                return self.err("Expected record field separator");
            }
            self.commas()
        }
        self.take("}")?;
        if self.unique_fields(&xs).is_err() {
            return self.err("Duplicate record field");
        }
        Ok(json!({"kind":"record","args":xs}))
    }
    pub(super) fn pattern(&mut self) -> R<J> {
        let list = self.peek() == "[";
        let close = if list { "]" } else { "}" };
        self.take(if list { "[" } else { "{" })?;
        self.commas();
        let mut xs = Vec::new();
        while self.peek() != close {
            xs.push(json!({"kind":"name","value":self.id()?}));
            if xs.len() > 32 {
                return self.err("At most 32 destructured fields");
            }
            if self.peek() != close && !["\n", ","].contains(&self.peek()) {
                return self.err("Expected pattern separator");
            }
            self.commas()
        }
        self.take(close)?;
        if xs.is_empty() || !unique(xs.iter().map(|x| x["value"].as_str().unwrap().into())) {
            return self.err("Empty or duplicate destructuring pattern");
        }
        Ok(json!({"kind":if list {"list_pattern"} else {"pattern"},"items":xs}))
    }
    pub(super) fn binding(&mut self) -> R<J> {
        if ["{", "["].contains(&self.peek()) {
            self.pattern()
        } else {
            Ok(json!({"kind":"name","value":self.id()?}))
        }
    }
    pub(super) fn destructuring_ahead(&self) -> bool {
        if !["[", "{"].contains(&self.peek()) {
            return false;
        }
        let close = if self.peek() == "[" { "]" } else { "}" };
        self.tokens[self.p..]
            .array_windows()
            .take(100)
            .find(|[a, _]| a.text == close)
            .is_some_and(|[_, b]| b.text == "=")
    }
    pub(super) fn check_statement(&mut self) -> R<J> {
        let kind = self.pop()?;
        Ok(json!({"kind":kind,"left":self.expr(0)?}))
    }
    pub(super) fn function(&mut self) -> R<J> {
        let (gs, bounds) = self.function_generics("<", ">")?;
        self.inner();
        let inputs = self.fields("->", "=>", true)?;
        let inferred = self.peek() != "->";
        if !inferred {
            self.take("->")?;
            self.inner();
        }
        let single = inferred || self.next() != ":";
        let outputs = if inferred {
            Vec::new()
        } else if single {
            vec![json!({"name":"value","type":self.ty(0)?})]
        } else {
            self.fields("{", "=>", false)?
        };
        if !inferred && outputs.is_empty() {
            return self.err("Function requires a result type");
        }
        self.inner();
        let mut items = Vec::new();
        let result;
        if ["=>", "ret"].contains(&self.peek()) {
            self.pop()?;
            self.inner();
            result = self.expr(0)?
        } else {
            self.take("{")?;
            self.skip();
            while self.peek() != "ret" {
                if self.peek() == "}" {
                    return self.err("Function requires ret");
                }
                if ["assert", "validate"].contains(&self.peek()) {
                    items.push(self.check_statement()?);
                } else {
                    let binding = self.binding()?;
                    self.take("=")?;
                    items.push(json!({"kind":"binding","left":binding,"right":self.expr(0)?}));
                }
                if items.len() > 64 {
                    return self.err("At most 64 function statements");
                }
                if !["\n", ";"].contains(&self.peek()) {
                    return self.err("Expected statement separator before ret");
                }
                self.skip()
            }
            self.take("ret")?;
            self.inner();
            result = self.expr(0)?;
            self.skip();
            self.take("}")?
        }
        Ok(
            json!({"kind":"function","generics":gs,"bounds":bounds,"inputs":inputs,"outputs":outputs,"singleResult":single,"inferResult":inferred,"items":items,"left":result}),
        )
    }
    pub(super) fn indent(&self) -> String {
        let start = self.tokens[self.p].start;
        let line = self.source[..start].rfind('\n').map_or(0, |p| p + 1);
        self.source[line..start]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }
    pub(super) fn continuation(&mut self, base: &str, msg: &str) -> R<Option<String>> {
        if self.peek() != "\n" {
            return Ok(None);
        }
        self.inner();
        let indent = self.indent();
        if indent.len() <= base.len() || !indent.starts_with(base) {
            return self.err(msg);
        }
        if indent.contains(' ') && indent.contains('\t') {
            return self.err("Do not mix tabs and spaces in indentation");
        }
        Ok(Some(indent))
    }
    pub(super) fn layout_fields(&mut self, base: &str, defaults: bool) -> R<Vec<J>> {
        let mut xs = Vec::new();
        loop {
            let name = self.id()?;
            self.take(":")?;
            let mut field = json!({"name":&name,"type":self.ty(0)?});
            if defaults && self.peek() == "=" {
                self.pop()?;
                field["default"] = self.expr(2)?;
            }
            xs.push(field);
            if xs.len() > 32 {
                return self.err("At most 32 fields or parameters");
            }
            if self.peek() != "," {
                break;
            }
            self.pop()?;
            self.continuation(base, "Expected indented signature continuation")?;
        }
        self.unique_fields(&xs)?;
        Ok(xs)
    }
}
