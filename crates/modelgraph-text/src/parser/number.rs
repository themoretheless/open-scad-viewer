use super::*;

pub fn number(s: &str) -> R<(f64, String)> {
    let unit = ["mm", "cm", "in", "deg", "rad", "m"]
        .into_iter()
        .find(|u| s.ends_with(u))
        .unwrap_or("");
    let num = s[..s.len() - unit.len()]
        .parse::<f64>()
        .map_err(|_| crate::error("Invalid number"))?;
    if !num.is_finite() {
        return Err(crate::error("Numeric literal must be finite"));
    }
    Ok((num, unit.into()))
}
pub fn parse(source: &str) -> R<Vec<Statement>> {
    let mut p = Parser {
        match_indent: None,
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
            "trait" => {
                p.pop()?;
                let name = p.id()?;
                let mut result = p.trait_members(false)?;
                result["kind"] = json!("trait");
                result["name"] = json!(name);
                result
            }
            "impl" => {
                p.pop()?;
                let name = p.id()?;
                p.take("for")?;
                let target = p.ty(0)?;
                let mut result = p.trait_members(true)?;
                result["kind"] = json!("impl");
                result["name"] = json!(name);
                result["target"] = target;
                result
            }
            "struct" => {
                let kind = p.pop()?;
                let name = p.id()?;
                let gs = if kind == "struct" {
                    p.generics("<", ">")?
                } else {
                    Vec::new()
                };
                p.inner();
                p.take("{")?;
                let fields = p.fields("}", "", false)?;
                p.take("}")?;
                json!({"kind":kind,"name":&name,"generics":gs,"fields":fields})
            }
            "{" | "[" => {
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
            "foreach" => {
                p.pop()?;
                json!({"kind":"show","value":p.foreach()?})
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
