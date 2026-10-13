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
            "ModelGraph Text line {}: {}",
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
    fn commas(&mut self) {
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
    fn function_generics(&mut self, open: &str, close: &str) -> R<(Vec<String>, J)> {
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
    fn unique_fields(&self, xs: &[J]) -> R<()> {
        if !unique(xs.iter().map(|x| x["name"].as_str().unwrap_or("").into())) {
            return self.err("Duplicate field or parameter");
        }
        Ok(())
    }
    fn record(&mut self) -> R<J> {
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
    fn function(&mut self) -> R<J> {
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
    fn layout_fields(&mut self, base: &str, defaults: bool) -> R<Vec<J>> {
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
    pub(super) fn layout(&mut self, base: &str) -> R<J> {
        let (gs, bounds) = self.function_generics("[", "]")?;
        self.inner();
        let inputs = if self.peek() == "->" || self.next() != ":" {
            Vec::new()
        } else {
            self.layout_fields(base, true)?
        };
        let signature_end = self.p;
        self.inner();
        if self.peek() == "=>"
            || (self.peek() == "ret" && !["\n", "EOF"].contains(&self.next()))
            || (self.peek() == "ret" && self.p == signature_end)
        {
            if self.peek() == "ret" && self.p != signature_end && self.indent().len() <= base.len()
            {
                return self.err("Expected indented function body");
            }
            self.pop()?;
            self.continuation(base, "Expected indented return expression")?;
            return Ok(
                json!({"kind":"function","generics":gs,"bounds":bounds,"inputs":inputs,"outputs":[],"singleResult":true,"inferResult":true,"items":[],"left":self.expr(0)?}),
            );
        }
        let void = self.peek() != "->";
        if void {
            self.p = signature_end
        } else {
            self.pop()?;
            self.continuation(base, "Expected indented signature continuation")?;
        }
        let single = !void && self.next() != ":";
        let outputs = if void {
            Vec::new()
        } else if single {
            vec![json!({"name":"value","type":self.ty(0)?})]
        } else {
            self.layout_fields(base, false)?
        };
        if !void && ["=>", "ret"].contains(&self.peek()) {
            self.pop()?;
            self.inner();
            return Ok(
                json!({"kind":"function","generics":gs,"bounds":bounds,"inputs":inputs,"outputs":outputs,"singleResult":single,"items":[],"left":self.expr(0)?}),
            );
        }
        if self.peek() != "\n" {
            return self.err("Expected newline after function signature");
        }
        self.inner();
        let indent = self.indent();
        if indent.len() <= base.len() || !indent.starts_with(base) {
            return self.err("Expected indented function body");
        }
        if indent.contains(' ') && indent.contains('\t') {
            return self.err("Do not mix tabs and spaces in indentation");
        }
        let mut items = Vec::new();
        let mut body_end = self.p;
        let mut implicit = false;
        while self.peek() != "ret" {
            if void && (self.peek() == "EOF" || self.indent().len() <= base.len()) {
                self.p = body_end;
                implicit = true;
                break;
            }
            if self.peek() == "EOF" || self.indent() != indent {
                return self.err("Expected ret at function body indentation");
            }
            if ["assert", "validate"].contains(&self.peek()) {
                items.push(self.check_statement()?)
            } else if self.next() == "(" {
                items.push(json!({"kind":"statement","left":self.expr(0)?}))
            } else {
                let binding = self.binding()?;
                if self.peek() != "=" {
                    return self.err("Expected = for local binding");
                }
                self.pop()?;
                items.push(json!({"kind":"binding","left":binding,"right":self.expr(0)?}))
            }
            if items.len() > 64 {
                return self.err("At most 64 function statements");
            }
            if !["\n", "EOF"].contains(&self.peek()) {
                return self.err("Expected newline after local binding");
            }
            body_end = self.p;
            self.inner()
        }
        let mut inferred = false;
        let result;
        if implicit {
            result = json!({"kind":"record","args":[]})
        } else {
            if self.indent() != indent {
                return self.err("Expected ret at function body indentation");
            }
            self.take("ret")?;
            if void && !["\n", "EOF"].contains(&self.peek()) {
                inferred = true;
                result = self.expr(0)?
            } else if void {
                result = json!({"kind":"record","args":[]})
            } else if single {
                self.continuation(&indent, "Expected indented return expression")?;
                result = self.expr(0)?
            } else {
                let mut field_indent =
                    self.continuation(&indent, "Expected indented return fields")?;
                let mut args = Vec::new();
                loop {
                    let name = self.id()?;
                    self.take(":")?;
                    args.push(json!({"name":&name,"value":self.expr(0)?}));
                    if args.len() > 32 {
                        return self.err("At most 32 record fields");
                    }
                    if self.peek() != "," {
                        break;
                    }
                    self.pop()?;
                    if let Some(i) =
                        self.continuation(&indent, "Expected indented return fields")?
                    {
                        if field_indent.as_ref().is_some_and(|old| *old != i) {
                            return self.err("Return fields must have matching indentation");
                        }
                        field_indent = Some(i)
                    }
                }
                if self.unique_fields(&args).is_err() {
                    return self.err("Duplicate record field");
                }
                result = json!({"kind":"record","args":args})
            }
            if !["\n", "EOF"].contains(&self.peek()) {
                return self.err("Expected end of return");
            }
            let end = self.p;
            self.inner();
            if self.peek() != "EOF" && self.indent().len() > base.len() {
                return self.err("Unexpected statement after ret or missing comma");
            }
            self.p = end;
        }
        Ok(
            json!({"kind":"function","generics":gs,"bounds":bounds,"inputs":inputs,"outputs":outputs,"singleResult":single,"voidResult":void && !inferred,"inferResult":inferred,"items":items,"left":result}),
        )
    }
    fn args(&mut self) -> R<Vec<J>> {
        self.take("(")?;
        self.inner();
        let mut xs = Vec::new();
        while self.peek() != ")" {
            let name = if self.next() == ":" {
                let n = self.pop()?;
                self.take(":")?;
                Some(n)
            } else {
                None
            };
            let mut arg = json!({"value":self.expr(0)?});
            if let Some(n) = name {
                arg["name"] = json!(n)
            }
            xs.push(arg);
            self.inner();
            if self.peek() != "," {
                break;
            }
            self.pop()?;
            self.inner()
        }
        self.take(")")?;
        Ok(xs)
    }
    fn type_args(&self) -> bool {
        if self.peek() != "<" {
            return false;
        }
        let mut depth = 0;
        for i in self.p..self.tokens.len() {
            let t = self.tokens[i].text.as_str();
            if t == "<" {
                depth += 1
            } else if t == ">" {
                depth -= 1;
                if depth == 0 {
                    return self
                        .tokens
                        .get(i + 1)
                        .is_some_and(|t| ["(", "{"].contains(&t.text.as_str()));
                }
            } else if !ident(t) && ![",", "\n"].contains(&t) {
                return false;
            }
        }
        false
    }
    // A foreach is a value-producing block. Its body has lexical bindings and
    // ends in yield; nested foreach expressions compose without mutation.
    pub(super) fn foreach(&mut self) -> R<J> {
        let base = self.indent();
        let mut names = Vec::new();
        if self.peek() == "(" {
            self.pop()?;
            loop {
                names.push(json!({"kind":"name","value":self.id()?}));
                if self.peek() != "," {
                    break;
                }
                self.pop()?;
            }
            self.take(")")?;
        } else {
            names.push(json!({"kind":"name","value":self.id()?}));
        }
        if !unique(names.iter().map(|n| n["value"].as_str().unwrap().into())) {
            return self.err("Duplicate foreach binding");
        }
        self.take("in")?;
        let mut clauses = vec![json!({"kind":"for","items":names,"left":self.expr(2)?})];
        while ["where", "let"].contains(&self.peek()) {
            let kind = self.pop()?;
            if kind == "where" {
                clauses.push(json!({"kind":kind,"left":self.expr(2)?}));
            } else {
                let pattern = self.binding()?;
                self.take("=")?;
                clauses.push(json!({"kind":"let","pattern":pattern,"left":self.expr(2)?}));
            }
        }
        let body;
        if self.peek() == "=>" {
            self.pop()?;
            self.inner();
            body = self.expr(0)?;
        } else {
            self.take("\n")?;
            self.inner();
            let indent = self.indent();
            if indent.len() <= base.len() || !indent.starts_with(&base) {
                return self.err("Expected indented foreach body");
            }
            loop {
                if self.peek() == "EOF" || self.indent() != indent {
                    return self.err("Expected yield at foreach body indentation");
                }
                if self.peek() == "yield" {
                    self.pop()?;
                    body = self.expr(0)?;
                    break;
                }
                if ["assert", "validate"].contains(&self.peek()) {
                    clauses.push(self.check_statement()?);
                } else if ["where", "continue", "break"].contains(&self.peek()) {
                    let kind = self.pop()?;
                    if kind != "where" {
                        self.take("if")?;
                    }
                    let mut condition = self.expr(0)?;
                    if kind != "where" {
                        condition = json!({"kind":"binary","value":"==","left":condition,"right":{"kind":"number","value":"0"}});
                    }
                    clauses.push(
                        json!({"kind":if kind=="break"{"while"}else{"where"},"left":condition}),
                    );
                } else {
                    let pattern = self.binding()?;
                    self.take("=")?;
                    clauses.push(json!({"kind":"let","pattern":pattern,"left":self.expr(0)?}));
                }
                if clauses.len() > 64 {
                    return self.err("At most 64 foreach clauses");
                }
                self.take("\n")?;
                self.inner();
            }
        }
        Ok(json!({"kind":"comprehension","items":clauses,"left":body}))
    }
    pub(super) fn expr(&mut self, min: u8) -> R<J> {
        self.depth += 1;
        if self.depth > 64 {
            return self.err("Expression nesting exceeds 64");
        }
        let t = self.pop()?;
        let mut a = match t.as_str() {
            "fn" => self.function()?,
            "foreach" => self.foreach()?,
            "match" => self.match_expression()?,
            "{" => {
                self.p -= 1;
                self.record()?
            }
            "!" => json!({"kind":"not","left":self.expr(25)?}),
            "-" => json!({"kind":"neg","left":self.expr(25)?}),
            "(" => {
                self.inner();
                let start = self.p;
                let mut names = Vec::new();
                while ident(self.peek()) && self.peek() != "EOF" {
                    names.push(self.id()?);
                    if self.peek() != "," {
                        break;
                    }
                    self.pop()?;
                    self.inner();
                }
                if self.peek() == ")" && self.next() == "=>" {
                    if names.is_empty() || names.len() > 8 || !unique(names.clone()) {
                        return self.err("Lambda requires 1..8 distinct parameters");
                    }
                    self.pop()?;
                    self.pop()?;
                    self.inner();
                    json!({"kind":"lambda","value":&names[0],"parameters":names,"left":self.expr(0)?})
                } else {
                    self.p = start;
                    let x = self.expr(0)?;
                    self.inner();
                    self.take(")")?;
                    x
                }
            }
            "[" => {
                self.inner();
                if self.peek() == "for" {
                    let mut clauses = Vec::new();
                    while ["for", "let", "where"].contains(&self.peek()) {
                        let kind = self.pop()?;
                        if kind == "where" {
                            clauses.push(json!({"kind":kind,"left":self.expr(2)?}))
                        } else {
                            let mut names = Vec::new();
                            if kind == "for" && self.peek() == "(" {
                                self.pop()?;
                                loop {
                                    names.push(json!({"kind":"name","value":self.pop()?}));
                                    if self.peek() != "," {
                                        break;
                                    }
                                    self.pop()?;
                                }
                                self.take(")")?;
                            } else {
                                names.push(json!({"kind":"name","value":self.pop()?}))
                            }
                            if names.iter().any(|x| !ident(x["value"].as_str().unwrap()))
                                || !unique(
                                    names.iter().map(|x| x["value"].as_str().unwrap().into()),
                                )
                            {
                                return self.err("Invalid or duplicate binding");
                            }
                            self.take(if kind == "for" { "in" } else { "=" })?;
                            clauses.push(json!({"kind":kind,"items":names,"left":self.expr(2)?}))
                        }
                        self.inner()
                    }
                    self.take("=>")?;
                    self.inner();
                    let x = json!({"kind":"comprehension","items":clauses,"left":self.expr(0)?});
                    self.inner();
                    self.take("]")?;
                    x
                } else {
                    let mut items = Vec::new();
                    while self.peek() != "]" {
                        items.push(self.expr(0)?);
                        self.inner();
                        if self.peek() != "," {
                            break;
                        }
                        self.pop()?;
                        self.inner()
                    }
                    self.take("]")?;
                    json!({"kind":"array","items":items})
                }
            }
            _ => {
                if t.starts_with('"') {
                    json!({"kind":"string","value":value_codec::from_str::<J>(&t).map_err(|e| crate::error(e.to_string()))?})
                } else if t.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
                    json!({"kind":"number","value":t})
                } else if ident(&t) && t != "EOF" {
                    if self.peek() == "=>" && min < 2 {
                        self.pop()?;
                        self.inner();
                        json!({"kind":"lambda","value":t,"left":self.expr(0)?})
                    } else if self.peek() == "(" {
                        json!({"kind":"call","value":t,"args":self.args()?})
                    } else if self.peek() == "{" || self.type_args() {
                        let mut types = Vec::new();
                        if self.peek() == "<" {
                            self.pop()?;
                            self.inner();
                            while self.peek() != ">" {
                                types.push(self.ty(0)?);
                                self.inner();
                                if self.peek() != "," {
                                    break;
                                }
                                self.pop()?;
                                self.inner()
                            }
                            self.take(">")?;
                        }
                        if self.peek() == "{" {
                            let mut r = self.record()?;
                            r["value"] = json!(t);
                            r["types"] = json!(types);
                            r
                        } else if self.peek() == "(" {
                            json!({"kind":"call","value":t,"types":types,"args":self.args()?})
                        } else {
                            return self.err("Expected constructor or generic function call");
                        }
                    } else {
                        json!({"kind":"name","value":t})
                    }
                } else {
                    return self.err(format!("Expected expression, got {t}"));
                }
            }
        };
        while self.peek() == "."
            || self.peek() == "["
            || (self.peek() == "\n" && self.next() == ".")
        {
            if self.peek() == "[" {
                self.pop()?;
                self.inner();
                let index = self.expr(0)?;
                self.inner();
                self.take("]")?;
                a = json!({"kind":"index","left":a,"right":index});
                continue;
            }
            if self.peek() == "\n" {
                let end = self.p;
                self.pop()?;
                if self
                    .match_indent
                    .is_some_and(|indent| self.indent().len() < indent)
                {
                    self.p = end;
                    break;
                }
            }
            self.pop()?;
            let name = self.pop()?;
            a = if self.peek() == "(" {
                json!({"kind":"pipe","left":a,"right":{"kind":"call","value":name,"args":self.args()?}})
            } else {
                json!({"kind":"member","left":a,"value":&name})
            }
        }
        loop {
            if self.peek() == "|>" {
                return self.err("Use .method(...) instead of |>");
            }
            if self.peek() == "with" && min <= 2 {
                self.pop()?;
                self.inner();
                a = json!({"kind":"with","left":a,"right":self.record()?});
                continue;
            }
            let op = self.peek().to_owned();
            let prec = match op.as_str() {
                "||" => 2,
                "&&" => 3,
                "==" | "!=" | "<" | "<=" | ">" | ">=" => 4,
                ".." | "..<" => 5,
                "+" | "-" => 10,
                "*" | "/" | "%" => 20,
                "**" => 25,
                _ => 0,
            };
            if prec == 0 || prec < min {
                break;
            }
            self.pop()?;
            self.inner();
            if op == ".." || op == "..<" {
                let end = self.expr(6)?;
                let mut args = Vec::new();
                if ["by", "count"].contains(&self.peek()) {
                    let mode = self.pop()?;
                    args.push(json!({"name":mode,"value":self.expr(6)?}));
                }
                if ["by", "count"].contains(&self.peek()) {
                    return self.err("Use either by or count, never both");
                }
                a = json!({"kind":"interval","value":&op,"left":a,"right":end,"args":args})
            } else {
                a = json!({"kind":"binary","value":&op,"left":a,"right":self.expr(if op=="**"{prec}else{prec+1})?})
            }
        }
        // A leading ? explicitly continues the preceding condition across
        // lines. Preserve the newline if the next token starts a statement.
        if min <= 2 && self.peek() == "\n" {
            let end = self.p;
            self.inner();
            if self.peek() != "?" {
                self.p = end;
            }
        }
        if min <= 2 && self.peek() == "?" {
            self.pop()?;
            self.inner();
            let branch_min = if min >= 2 { 2 } else { 0 };
            let yes = self.expr(branch_min)?;
            self.inner();
            self.take(":")?;
            self.inner();
            let no = self.expr(branch_min)?;
            a = json!({"kind":"conditional","left":a,"items":[yes,no]})
        }
        self.depth -= 1;
        Ok(a)
    }
    pub(super) fn literal(&mut self) -> R<(f64, String, usize, usize)> {
        let start = self.tokens[self.p].start;
        let sign = if self.peek() == "-" {
            self.pop()?;
            -1.0
        } else {
            1.0
        };
        let t = self.tokens[self.p].clone();
        self.pop()?;
        let (v, u) = number(&t.text).map_err(|_| {
            crate::error(format!(
                "ModelGraph Text line {}: Parameter default must be a numeric literal",
                self.line()
            ))
        })?;
        Ok((
            sign * v,
            u.clone(),
            self.source[..start].encode_utf16().count(),
            self.source[..t.end - u.len()].encode_utf16().count(),
        ))
    }
}
