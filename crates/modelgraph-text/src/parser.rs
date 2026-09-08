use crate::value::json;
use serde_json::Value as J;
type R<T> = Result<T, String>;
#[derive(Clone)]
struct Token {
    text: String,
    start: usize,
    end: usize,
}
pub struct Statement {
    pub node: J,
    pub line: usize,
}
struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    p: usize,
    depth: usize,
    line_starts: Vec<usize>,
}
fn ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|x| x.is_ascii_alphabetic() || x == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}
fn unique(xs: impl IntoIterator<Item = String>) -> bool {
    let mut set = std::collections::HashSet::new();
    xs.into_iter().all(|x| set.insert(x))
}
fn lex(source: &str) -> R<Vec<Token>> {
    if source.encode_utf16().count() > 262144 {
        return Err("ModelGraph Text exceeds 256 KiB.".into());
    }
    let b = source.as_bytes();
    let mut p = 0;
    let mut out = Vec::new();
    while p < b.len() {
        let start = p;
        if b[p] == b' ' || b[p] == b'\t' || b[p] == b'\r' {
            p += 1;
            continue;
        }
        if source[p..].starts_with("//") {
            while p < b.len() && b[p] != b'\n' {
                p += 1
            }
            continue;
        }
        if source[p..].starts_with("/*") {
            if let Some(end) = source[p + 2..].find("*/") {
                p += end + 4;
                continue;
            }
        }
        if b[p] == b'"' {
            p += 1;
            let mut closed = false;
            while p < b.len() {
                if b[p] == b'\n' {
                    break;
                }
                if b[p] == b'"' {
                    p += 1;
                    closed = true;
                    break;
                }
                if b[p] == b'\\' {
                    p += 1
                }
                if p < b.len() {
                    p += source[p..].chars().next().unwrap().len_utf8()
                }
            }
            if !closed {
                return Err(format!("Unexpected character at {start}: \""));
            }
        } else if b[p].is_ascii_digit()
            || (b[p] == b'.' && b.get(p + 1).is_some_and(u8::is_ascii_digit))
        {
            if b[p] == b'.' {
                p += 1;
                while p < b.len() && b[p].is_ascii_digit() {
                    p += 1
                }
            } else {
                while p < b.len() && b[p].is_ascii_digit() {
                    p += 1
                }
                if b.get(p) == Some(&b'.')
                    && !b
                        .get(p + 1)
                        .is_some_and(|c| *c == b'.' || c.is_ascii_alphabetic() || *c == b'_')
                {
                    p += 1;
                    while p < b.len() && b[p].is_ascii_digit() {
                        p += 1
                    }
                }
            }
            if b.get(p).is_some_and(|c| *c == b'e' || *c == b'E') {
                let old = p;
                p += 1;
                if b.get(p).is_some_and(|c| *c == b'+' || *c == b'-') {
                    p += 1
                }
                let digits = p;
                while p < b.len() && b[p].is_ascii_digit() {
                    p += 1
                }
                if p == digits {
                    p = old
                }
            }
            for unit in ["mm", "cm", "in", "deg", "rad", "m"] {
                if source[p..].starts_with(unit) {
                    p += unit.len();
                    break;
                }
            }
        } else if b[p].is_ascii_alphabetic() || b[p] == b'_' {
            p += 1;
            while p < b.len() && (b[p].is_ascii_alphanumeric() || b[p] == b'_') {
                p += 1
            }
        } else if let Some(op) = ["|>", "->", "=>", "..<", "..", "**", "==", "!=", "<=", ">="]
            .iter()
            .find(|op| source[p..].starts_with(**op))
        {
            p += op.len()
        } else if b"\n{}()[],.?:;=+*/%<>-".contains(&b[p]) {
            p += 1
        } else {
            return Err(format!(
                "Unexpected character at {}: {}",
                source[..p].encode_utf16().count(),
                source[p..].chars().next().unwrap()
            ));
        }
        out.push(Token {
            text: source[start..p].into(),
            start,
            end: p,
        });
    }
    out.push(Token {
        text: "EOF".into(),
        start: p,
        end: p,
    });
    Ok(out)
}
impl Parser<'_> {
    fn peek(&self) -> &str {
        &self.tokens[self.p].text
    }
    fn next(&self) -> &str {
        self.tokens
            .get(self.p + 1)
            .map_or("EOF", |t| t.text.as_str())
    }
    fn err<T>(&self, m: impl AsRef<str>) -> R<T> {
        Err(format!(
            "ModelGraph Text line {}: {}",
            self.line(),
            m.as_ref()
        ))
    }
    fn line(&self) -> usize {
        self.line_starts
            .partition_point(|start| *start <= self.tokens[self.p].start)
    }
    fn pop(&mut self) -> R<String> {
        if self.peek() == "EOF" {
            return self.err("Unexpected end of source");
        }
        let s = self.peek().to_owned();
        self.p += 1;
        Ok(s)
    }
    fn take(&mut self, s: &str) -> R<()> {
        if self.peek() != s {
            return self.err(format!("Expected {s}, got {}", self.peek()));
        }
        self.pop()?;
        Ok(())
    }
    fn inner(&mut self) {
        while self.peek() == "\n" {
            self.p += 1
        }
    }
    fn skip(&mut self) {
        while ["\n", ";"].contains(&self.peek()) {
            self.p += 1
        }
    }
    fn commas(&mut self) {
        while ["\n", ",", ";"].contains(&self.peek()) {
            self.p += 1
        }
    }
    fn id(&mut self) -> R<String> {
        let s = self.pop()?;
        if !ident(&s) {
            return self.err("Expected an identifier");
        }
        Ok(s)
    }
    fn ty(&mut self, depth: usize) -> R<J> {
        if depth > 16 {
            return self.err("Type nesting exceeds 16");
        }
        let name = self.id()?;
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
    fn generics(&mut self, open: &str, close: &str) -> R<Vec<String>> {
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
    fn fields(&mut self, end: &str, alt: &str) -> R<Vec<J>> {
        let mut xs = Vec::new();
        self.commas();
        while self.peek() != end && self.peek() != alt {
            let name = self.id()?;
            self.take(":")?;
            xs.push(json!({"name":&name,"type":self.ty(0)?}));
            if xs.len() > 32 {
                return self.err("At most 32 fields or parameters");
            }
            if self.peek() != end && self.peek() != alt && !["\n", ",", ";"].contains(&self.peek())
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
    fn pattern(&mut self) -> R<J> {
        self.take("{")?;
        self.commas();
        let mut xs = Vec::new();
        while self.peek() != "}" {
            xs.push(json!({"kind":"name","value":self.id()?}));
            if xs.len() > 32 {
                return self.err("At most 32 destructured fields");
            }
            if self.peek() != "}" && !["\n", ","].contains(&self.peek()) {
                return self.err("Expected pattern separator");
            }
            self.commas()
        }
        self.take("}")?;
        if xs.is_empty() || !unique(xs.iter().map(|x| x["value"].as_str().unwrap().into())) {
            return self.err("Empty or duplicate destructuring pattern");
        }
        Ok(json!({"kind":"pattern","items":xs}))
    }
    fn binding(&mut self) -> R<J> {
        if self.peek() == "{" {
            self.pattern()
        } else {
            Ok(json!({"kind":"name","value":self.id()?}))
        }
    }
    fn function(&mut self) -> R<J> {
        let gs = self.generics("<", ">")?;
        self.inner();
        let inputs = self.fields("->", "")?;
        self.take("->")?;
        self.inner();
        let single = self.next() != ":";
        let outputs = if single {
            vec![json!({"name":"value","type":self.ty(0)?})]
        } else {
            self.fields("{", "=>")?
        };
        if outputs.is_empty() {
            return self.err("Function requires a result type");
        }
        self.inner();
        let mut items = Vec::new();
        let result;
        if self.peek() == "=>" {
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
                let binding = self.binding()?;
                self.take("=")?;
                items.push(json!({"kind":"binding","left":binding,"right":self.expr(0)?}));
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
            json!({"kind":"function","generics":gs,"inputs":inputs,"outputs":outputs,"singleResult":single,"items":items,"left":result}),
        )
    }
    fn indent(&self) -> String {
        let start = self.tokens[self.p].start;
        let line = self.source[..start].rfind('\n').map_or(0, |p| p + 1);
        self.source[line..start]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }
    fn continuation(&mut self, base: &str, msg: &str) -> R<Option<String>> {
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
    fn layout_fields(&mut self, base: &str) -> R<Vec<J>> {
        let mut xs = Vec::new();
        loop {
            let name = self.id()?;
            self.take(":")?;
            xs.push(json!({"name":&name,"type":self.ty(0)?}));
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
    fn layout(&mut self, base: &str) -> R<J> {
        let gs = self.generics("[", "]")?;
        self.inner();
        let inputs = if self.peek() == "->" || self.next() != ":" {
            Vec::new()
        } else {
            self.layout_fields(base)?
        };
        let signature_end = self.p;
        self.inner();
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
            self.layout_fields(base)?
        };
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
            if self.next() == "(" {
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
        let result;
        if implicit {
            result = json!({"kind":"record","args":[]})
        } else {
            if self.indent() != indent {
                return self.err("Expected ret at function body indentation");
            }
            self.take("ret")?;
            if void {
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
            json!({"kind":"function","generics":gs,"inputs":inputs,"outputs":outputs,"singleResult":single,"voidResult":void,"items":items,"left":result}),
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
    fn expr(&mut self, min: u8) -> R<J> {
        self.depth += 1;
        if self.depth > 64 {
            return self.err("Expression nesting exceeds 64");
        }
        let t = self.pop()?;
        let mut a = match t.as_str() {
            "fn" => self.function()?,
            "{" => {
                self.p -= 1;
                self.record()?
            }
            "-" => json!({"kind":"neg","left":self.expr(25)?}),
            "(" => {
                self.inner();
                let x = self.expr(0)?;
                self.inner();
                self.take(")")?;
                x
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
                    json!({"kind":"string","value":serde_json::from_str::<J>(&t).map_err(|e|e.to_string())?})
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
        while self.peek() == "." || (self.peek() == "\n" && self.next() == ".") {
            if self.peek() == "\n" {
                self.pop()?;
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
            let op = self.peek().to_owned();
            let prec = match op.as_str() {
                "==" | "!=" | "<" | "<=" | ">" | ">=" => 3,
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
        if min <= 2 && self.peek() == "?" {
            self.pop()?;
            self.inner();
            let yes = self.expr(0)?;
            self.inner();
            self.take(":")?;
            self.inner();
            let no = self.expr(0)?;
            a = json!({"kind":"conditional","left":a,"items":[yes,no]})
        }
        self.depth -= 1;
        Ok(a)
    }
    fn literal(&mut self) -> R<(f64, String, usize, usize)> {
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
            format!(
                "ModelGraph Text line {}: Parameter default must be a numeric literal",
                self.line()
            )
        })?;
        Ok((
            sign * v,
            u.clone(),
            self.source[..start].encode_utf16().count(),
            self.source[..t.end - u.len()].encode_utf16().count(),
        ))
    }
}
pub fn number(s: &str) -> R<(f64, String)> {
    let unit = ["mm", "cm", "in", "deg", "rad", "m"]
        .into_iter()
        .find(|u| s.ends_with(u))
        .unwrap_or("");
    let num = s[..s.len() - unit.len()]
        .parse::<f64>()
        .map_err(|_| "Invalid number".to_string())?;
    if !num.is_finite() {
        return Err("Numeric literal must be finite".into());
    }
    Ok((num, unit.into()))
}
pub fn parse(source: &str) -> R<Vec<Statement>> {
    let mut p = Parser {
        source,
        tokens: lex(source)?,
        p: 0,
        depth: 0,
        line_starts: std::iter::once(0)
            .chain(
                source
                    .bytes()
                    .enumerate()
                    .filter_map(|(i, b)| (b == b'\n').then_some(i + 1)),
            )
            .collect(),
    };
    let mut out = Vec::new();
    p.skip();
    while p.peek() != "EOF" {
        let node = match p.peek() {
            "fn" => {
                let base = p.indent();
                p.pop()?;
                let name = p.id()?;
                json!({"kind":"bind","name":&name,"value":p.layout(&base)?})
            }
            "struct" => {
                p.pop()?;
                let name = p.id()?;
                let gs = p.generics("<", ">")?;
                p.inner();
                p.take("{")?;
                let fields = p.fields("}", "")?;
                p.take("}")?;
                json!({"kind":"struct","name":&name,"generics":gs,"fields":fields})
            }
            "{" => {
                let pat = p.pattern()?;
                p.take("=")?;
                json!({"kind":"destructure","pattern":pat,"value":p.expr(0)?})
            }
            "validate" | "assert" => {
                let kind = p.pop()?;
                json!({"kind":kind,"value":p.expr(0)?})
            }
            "segments" if p.next() != "=" => {
                p.pop()?;
                let s = p.pop()?;
                let n = s.parse::<u32>().ok();
                if !s.bytes().all(|b| b.is_ascii_digit())
                    || !n.is_some_and(|n| (12..=128).contains(&n))
                {
                    return p.err("segments requires an integer from 12 to 128");
                }
                json!({"kind":"segments","value":n})
            }
            "param" => {
                p.pop()?;
                let name = p.pop()?;
                if !["=", ":"].contains(&p.peek()) {
                    return p.err("Expected : or = after parameter name");
                }
                p.pop()?;
                let (v, u, start, end) = p.literal()?;
                let mut param = json!({"id":&name,"value":v});
                let mut control = json!({"name":&name,"label":if u.is_empty(){name.clone()}else{format!("{name} ({u})")},"value":v,"valueStart":start,"valueEnd":end});
                if !u.is_empty() {
                    param["unit"] = json!(&u)
                }
                if p.peek() == "range" {
                    p.pop()?;
                    let (min, mu, _, _) = p.literal()?;
                    p.take("..")?;
                    let (max, xu, _, _) = p.literal()?;
                    if mu != u || xu != u {
                        return p.err("Range units must match parameter units");
                    }
                    if min > max || v < min || v > max {
                        return p.err("Parameter default must lie in its range");
                    }
                    param["min"] = json!(min);
                    param["max"] = json!(max);
                    control["min"] = json!(min);
                    control["max"] = json!(max)
                }
                json!({"kind":"param","name":&name,"parameter":param,"control":control})
            }
            "show" => {
                p.pop()?;
                json!({"kind":"show","value":p.expr(0)?})
            }
            _ => {
                if p.next() == "(" {
                    json!({"kind":"statement","value":p.expr(0)?})
                } else {
                    let name = p.pop()?;
                    p.take("=")?;
                    json!({"kind":"bind","name":&name,"value":p.expr(0)?})
                }
            }
        };
        out.push(Statement {
            node,
            line: p.line(),
        });
        if !["EOF", "\n", ";"].contains(&p.peek()) {
            return p.err(format!("Unexpected {}", p.peek()));
        }
        p.skip()
    }
    Ok(out)
}
