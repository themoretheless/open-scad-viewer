use super::*;

pub(super) struct DirectBuildOptions {
    pub(super) analytic_surfaces: bool,
    pub(super) ap242_composition: bool,
    pub(super) allow_degenerate: bool,
    pub(super) whole_domain_proofs: bool,
    pub(super) interior_point_selectors: bool,
    pub(super) allow_open_shells: bool,
}

pub(super) fn refuse(message: impl Into<String>) -> Error {
    Error::new("BREP_STEP_V3_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct StepV3Report {
    pub identity: crate::step_interchange::StepIdentityReport,
    pub ignored_entities: Vec<String>,
    pub metadata_loss: Vec<String>,
    pub instance_count: usize,
    pub reachable_count: usize,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub external_references: Vec<String>,
}

/// Lossless `/10` product envelope. The parsed B-rep is a derived editing
/// projection; `source` remains authoritative for occurrence and presentation
/// graphs that cannot be represented by `Model`.
#[derive(Clone, Debug)]
pub struct StepV10Document {
    pub source: String,
    pub graph_identity: String,
    pub definition_identities: Vec<String>,
    pub occurrence_identities: Vec<String>,
    pub product_hierarchy: Vec<String>,
    pub operator_identities: Vec<String>,
    pub metadata_loss: Vec<String>,
}

/// Algebraic, whole-parameter-domain evidence carried by STEP `/8`.
///
/// `identity` names the exact rational tensor identity checked by the kernel;
/// it is deliberately not a sampling result.  A zero Jacobian lower bound is
/// permitted only for an explicitly named collapsed boundary.  In that case
/// `regular_open_domain` certifies the punctured parameter neighbourhood and
/// topology validation certifies that the whole boundary is one pole vertex.
#[derive(Clone, Debug)]
pub struct StepRegularityEvidence {
    pub carrier: &'static str,
    pub parameter_u: [f64; 2],
    pub parameter_v: [f64; 2],
    pub lifted_periods: [f64; 2],
    pub denominator_lower_bound: f64,
    pub jacobian_lower_bound: f64,
    pub collapsed_boundaries: Vec<&'static str>,
    pub regular_open_domain: bool,
    pub identity: &'static str,
}

#[derive(Clone, Debug)]
pub struct StepV8Certificate {
    pub capability: &'static str,
    pub complete: bool,
    pub regularity: Vec<StepRegularityEvidence>,
    pub sense_layers: [&'static str; 7],
    pub coupled_sense_cases: usize,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    String(String),
    Number(f64),
    Integer(i64),
    Ref(usize),
    Enum(String),
    Omitted,
    Derived,
    List(Vec<Value>),
    Call(String, Vec<Value>),
}
pub(super) fn render_value(v: &Value) -> String {
    match v {
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Number(n) => format!("{n:.17e}").replace('e', "E"),
        Value::Integer(n) => n.to_string(),
        Value::Ref(id) => format!("#{id}"),
        Value::Enum(e) => format!(".{e}."),
        Value::Omitted => "$".into(),
        Value::Derived => "*".into(),
        Value::List(v) => format!(
            "({})",
            v.iter().map(render_value).collect::<Vec<_>>().join(",")
        ),
        Value::Call(n, v) => format!(
            "{n}({})",
            v.iter().map(render_value).collect::<Vec<_>>().join(",")
        ),
    }
}
pub(super) fn render_entity_value(v: &Value) -> String {
    if let Value::List(parts) = v
        && parts.iter().all(|part| matches!(part, Value::Call(_, _)))
    {
        return format!(
            "({})",
            parts.iter().map(render_value).collect::<Vec<_>>().join("")
        );
    }
    render_value(v)
}

#[derive(Clone, Debug)]
pub(super) struct Entity {
    pub(super) value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Tok {
    Hash(usize),
    Ident(String),
    String(String),
    Number(String),
    LParen,
    RParen,
    Comma,
    Eq,
    Semi,
    Dollar,
    Star,
    Enum(String),
}

pub(super) fn lex(text: &str) -> Result<Vec<Tok>> {
    if text.len() > MAX_BYTES {
        return Err(refuse("STEP /3 payload exceeds 16 MiB"));
    }
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            let mut closed = false;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return Err(refuse("Unterminated Part 21 comment"));
            }
            continue;
        }
        match bytes[i] {
            b'#' => {
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if start == i {
                    return Err(refuse("Malformed Part 21 reference"));
                }
                out.push(Tok::Hash(
                    text[start..i]
                        .parse()
                        .map_err(|_| refuse("Invalid instance number"))?,
                ));
            }
            b'\'' => {
                i += 1;
                let mut value = String::new();
                loop {
                    if i >= bytes.len() {
                        return Err(refuse("Unterminated Part 21 string"));
                    }
                    if bytes[i] == b'\'' {
                        if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                            value.push('\'');
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    let ch = text[i..]
                        .chars()
                        .next()
                        .ok_or_else(|| refuse("Invalid UTF-8 string"))?;
                    value.push(ch);
                    i += ch.len_utf8();
                }
                out.push(Tok::String(value));
            }
            b'.' if i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() => {
                i += 1;
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                if i >= bytes.len() || bytes[i] != b'.' {
                    return Err(refuse(format!(
                        "Malformed STEP enumeration .{} at offset {start}",
                        &text[start..i]
                    )));
                }
                out.push(Tok::Enum(text[start..i].to_ascii_uppercase()));
                i += 1;
            }
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'=' => {
                out.push(Tok::Eq);
                i += 1;
            }
            b';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            b'$' => {
                out.push(Tok::Dollar);
                i += 1;
            }
            b'*' => {
                out.push(Tok::Star);
                i += 1;
            }
            b'+' | b'-' | b'0'..=b'9' | b'.' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && matches!(
                        bytes[i],
                        b'0'..=b'9' | b'.' | b'+' | b'-' | b'E' | b'e' | b'D' | b'd'
                    )
                {
                    i += 1;
                }
                out.push(Tok::Number(text[start..i].to_string()));
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Ident(text[start..i].to_ascii_uppercase()));
            }
            _ => return Err(refuse(format!("Unexpected Part 21 byte at offset {i}"))),
        }
    }
    Ok(out)
}

pub(super) struct Parser {
    pub(super) tokens: Vec<Tok>,
    pub(super) at: usize,
}
impl Parser {
    pub(super) fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_PARSE_DEPTH {
            return Err(refuse("Part 21 aggregate depth exceeds 32"));
        }
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or_else(|| refuse("Unexpected end of Part 21 value"))?;
        self.at += 1;
        Ok(match token {
            Tok::String(v) => Value::String(v),
            Tok::Hash(v) => Value::Ref(v),
            Tok::Dollar => Value::Omitted,
            Tok::Star => Value::Derived,
            Tok::Enum(v) => Value::Enum(v),
            Tok::Number(v) => {
                let normalized = v.replace(['D', 'd'], "E");
                if !normalized.contains(['.', 'E', 'e']) {
                    Value::Integer(
                        normalized
                            .parse()
                            .map_err(|_| refuse("Invalid STEP integer"))?,
                    )
                } else {
                    let n: f64 = normalized
                        .parse()
                        .map_err(|_| refuse("Invalid STEP real"))?;
                    if !n.is_finite() {
                        return Err(refuse("Non-finite STEP real"));
                    }
                    Value::Number(n)
                }
            }
            Tok::LParen => Value::List(self.list(depth + 1)?),
            Tok::Ident(name) => {
                if self.tokens.get(self.at) != Some(&Tok::LParen) {
                    return Err(refuse("Bare STEP identifier is not admitted"));
                }
                self.at += 1;
                Value::Call(name, self.list(depth + 1)?)
            }
            _ => return Err(refuse("Unexpected token in Part 21 value")),
        })
    }
    pub(super) fn list(&mut self, depth: usize) -> Result<Vec<Value>> {
        let mut values = Vec::new();
        if self.tokens.get(self.at) == Some(&Tok::RParen) {
            self.at += 1;
            return Ok(values);
        }
        loop {
            values.push(self.value(depth)?);
            match self.tokens.get(self.at) {
                Some(Tok::Comma) => self.at += 1,
                Some(Tok::RParen) => {
                    self.at += 1;
                    break;
                }
                // Part 21 complex entity components are adjacent calls rather
                // than comma-separated aggregate members.
                Some(Tok::Ident(_)) => {}
                other => {
                    return Err(refuse(format!(
                        "Malformed Part 21 argument list near token {}: {other:?}",
                        self.at
                    )));
                }
            }
        }
        Ok(values)
    }
}

pub(super) fn parse(text: &str) -> Result<BTreeMap<usize, Entity>> {
    let tokens = lex(text)?;
    let mut parser = Parser { tokens, at: 0 };
    let mut entities = BTreeMap::new();
    while parser.at < parser.tokens.len() {
        let Some(Tok::Hash(id)) = parser.tokens.get(parser.at).cloned() else {
            parser.at += 1;
            continue;
        };
        if parser.tokens.get(parser.at + 1) != Some(&Tok::Eq) {
            parser.at += 1;
            continue;
        }
        parser.at += 2;
        let value = parser.value(0)?;
        if parser.tokens.get(parser.at) != Some(&Tok::Semi) {
            return Err(refuse("STEP instance is missing semicolon"));
        }
        parser.at += 1;
        if entities.insert(id, Entity { value }).is_some() {
            return Err(refuse("Duplicate Part 21 instance number"));
        }
        if entities.len() > MAX_INSTANCES {
            return Err(refuse("STEP /3 instance count exceeds 65536"));
        }
    }
    if entities.is_empty() {
        return Err(refuse("STEP /3 contains no instances"));
    }
    Ok(entities)
}

pub(super) fn call(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<(&str, &[Value])> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse(format!("Missing reference #{id}")))?;
    match &entity.value {
        Value::Call(name, args) => Ok((name, args)),
        Value::List(parts) => {
            // Complex instances are parsed and bounded, but geometry complexes
            // are resolved only when an explicitly supported component exists.
            for part in parts {
                if let Value::Call(name, args) = part
                    && matches!(
                        name.as_str(),
                        "B_SPLINE_CURVE_WITH_KNOTS"
                            | "B_SPLINE_SURFACE_WITH_KNOTS"
                            | "SI_UNIT"
                            | "CONVERSION_BASED_UNIT"
                            | "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                            | "GEOMETRIC_REPRESENTATION_CONTEXT"
                    )
                {
                    return Ok((name, args));
                }
            }
            Err(refuse(format!(
                "Reachable complex instance #{id} has no admitted component"
            )))
        }
        _ => Err(refuse(format!("Reference #{id} is not an entity call"))),
    }
}
pub(super) fn components(entities: &BTreeMap<usize, Entity>, id: usize) -> Result<Vec<(&str, &[Value])>> {
    let entity = entities
        .get(&id)
        .ok_or_else(|| refuse(format!("Missing reference #{id}")))?;
    match &entity.value {
        Value::Call(name, args) => Ok(vec![(name.as_str(), args.as_slice())]),
        Value::List(parts) => parts
            .iter()
            .map(|part| match part {
                Value::Call(name, args) => Ok((name.as_str(), args.as_slice())),
                _ => Err(refuse(format!(
                    "Malformed complex entity component in #{id}"
                ))),
            })
            .collect(),
        _ => Err(refuse(format!("Reference #{id} is not an entity call"))),
    }
}
pub(super) fn component<'a>(parts: &[(&'a str, &'a [Value])], ty: &str) -> Option<&'a [Value]> {
    parts
        .iter()
        .find_map(|(name, args)| (*name == ty).then_some(*args))
}
pub(super) fn refs(value: &Value, out: &mut Vec<usize>) {
    match value {
        Value::Ref(id) => out.push(*id),
        Value::List(v) | Value::Call(_, v) => {
            for x in v {
                refs(x, out);
            }
        }
        _ => {}
    }
}
pub(super) fn one_ref(v: &Value, what: &str) -> Result<usize> {
    if let Value::Ref(id) = v {
        Ok(*id)
    } else {
        Err(refuse(format!("{what} must be a reference")))
    }
}
pub(super) fn list<'a>(v: &'a Value, what: &str) -> Result<&'a [Value]> {
    if let Value::List(items) = v {
        Ok(items)
    } else {
        Err(refuse(format!("{what} must be an aggregate")))
    }
}
pub(super) fn number(v: &Value, what: &str) -> Result<f64> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::Integer(n) => Ok(*n as f64),
        _ => Err(refuse(format!("{what} must be numeric"))),
    }
}
pub(super) fn string_value<'a>(v: &'a Value, what: &str) -> Result<&'a str> {
    if let Value::String(value) = v {
        Ok(value)
    } else {
        Err(refuse(format!("{what} must be a string")))
    }
}
pub(super) fn usize_value(v: &Value, what: &str) -> Result<usize> {
    match v {
        Value::Integer(n) if *n >= 0 => Ok(*n as usize),
        _ => Err(refuse(format!("{what} must be a nonnegative integer"))),
    }
}
pub(super) fn boolean(v: &Value, what: &str) -> Result<bool> {
    match v {
        Value::Enum(v) if v == "T" => Ok(true),
        Value::Enum(v) if v == "F" => Ok(false),
        _ => Err(refuse(format!("{what} must be .T. or .F."))),
    }
}
pub(super) fn name(args: &[Value]) -> Option<&str> {
    match args.first() {
        Some(Value::String(v)) => Some(v),
        _ => None,
    }
}

pub(super) fn reachable(entities: &BTreeMap<usize, Entity>, roots: &[usize]) -> Result<BTreeSet<usize>> {
    pub(super) fn visit(
        id: usize,
        depth: usize,
        entities: &BTreeMap<usize, Entity>,
        active: &mut BTreeSet<usize>,
        done: &mut BTreeSet<usize>,
    ) -> Result<()> {
        if depth > MAX_GRAPH_DEPTH {
            return Err(refuse("STEP reachable graph depth exceeds 64"));
        }
        if done.contains(&id) {
            return Ok(());
        }
        if !active.insert(id) {
            return Err(refuse("Cycle in reachable STEP graph"));
        }
        let entity = entities
            .get(&id)
            .ok_or_else(|| refuse(format!("Missing reachable reference #{id}")))?;
        let mut dependencies = Vec::new();
        refs(&entity.value, &mut dependencies);
        for dependency in dependencies {
            visit(dependency, depth + 1, entities, active, done)?;
        }
        active.remove(&id);
        done.insert(id);
        Ok(())
    }
    let mut done = BTreeSet::new();
    for root in roots {
        visit(*root, 0, entities, &mut BTreeSet::new(), &mut done)?;
    }
    Ok(done)
}

pub(super) fn values(v: &Value, what: &str) -> Result<Vec<f64>> {
    list(v, what)?.iter().map(|v| number(v, what)).collect()
}
pub(super) fn ints(v: &Value, what: &str) -> Result<Vec<usize>> {
    list(v, what)?
        .iter()
        .map(|v| usize_value(v, what))
        .collect()
}
pub(super) fn expand(mults: &[usize], knots: &[f64]) -> Result<Vec<f64>> {
    if mults.len() != knots.len() {
        return Err(refuse("Knot multiplicity count mismatch"));
    }
    let mut out = Vec::new();
    for (&m, &k) in mults.iter().zip(knots) {
        out.extend(std::iter::repeat_n(k, m));
    }
    Ok(out)
}
pub(super) fn point(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "CARTESIAN_POINT" || args.len() != 2 {
        return Err(refuse("Expected CARTESIAN_POINT"));
    }
    let mut p = values(&args[1], "CARTESIAN_POINT coordinates")?;
    if p.len() != dim {
        return Err(refuse("CARTESIAN_POINT dimension mismatch"));
    }
    if dim == 3 {
        for x in &mut p {
            *x *= scale;
        }
    }
    Ok(p)
}
pub(super) fn direction(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "DIRECTION" || args.len() != 2 {
        return Err(refuse("Expected DIRECTION"));
    }
    let d = values(&args[1], "DIRECTION ratios")?;
    if d.len() != dim {
        return Err(refuse("DIRECTION dimension mismatch"));
    }
    let n = d.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 1e-12 {
        return Err(refuse("Degenerate DIRECTION"));
    }
    Ok(d.into_iter().map(|x| x / n).collect())
}
