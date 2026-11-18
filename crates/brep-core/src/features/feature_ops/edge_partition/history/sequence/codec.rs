//! Versioned strict mixed-step transport, including legacy /1 replay migration.
use super::*;
fn command(c: &Command) -> Value {
    let operation = match &c.operation {
        Operation::Split { cuts } => value_codec::json!({"kind":"split","cuts":cuts}),
        Operation::Relocate { parameter } => {
            value_codec::json!({"kind":"relocate","parameter":parameter})
        }
        Operation::Compact => value_codec::json!({"kind":"compact"}),
        Operation::Reclock { domain } => value_codec::json!({"kind":"reclock","domain":domain}),
    };
    value_codec::json!({"parent":c.parent.to_string(),"key":c.key,"uvTolerance":c.uv_tolerance,"operation":operation})
}
fn parse_command(v: &Value) -> Result<Command> {
    let c = wire::object(v, &["parent", "key", "uvTolerance", "operation"])?;
    let parent = TopoId::parse(
        c["parent"]
            .as_str()
            .ok_or_else(|| error("Mixed parent must be a topology ID"))?,
    )
    .map_err(error)?;
    let key = c["key"]
        .as_str()
        .ok_or_else(|| error("Mixed key must be a string"))?
        .to_owned();
    let uv_tolerance = c["uvTolerance"]
        .as_f64()
        .ok_or_else(|| error("Mixed UV tolerance must be numeric"))?;
    let raw = &c["operation"];
    let kind = raw
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| error("Mixed operation kind must be a string"))?;
    let operation = match kind {
        "split" => {
            let o = wire::object(raw, &["kind", "cuts"])?;
            let cuts = o["cuts"]
                .as_array()
                .filter(|c| (1..=16).contains(&c.len()))
                .ok_or_else(|| error("Mixed split needs 1..16 cuts"))?;
            Operation::Split {
                cuts: cuts
                    .iter()
                    .map(|v| v.as_f64().ok_or_else(|| error("Mixed cut must be numeric")))
                    .collect::<Result<Vec<_>>>()?,
            }
        }
        "relocate" => {
            let o = wire::object(raw, &["kind", "parameter"])?;
            Operation::Relocate {
                parameter: o["parameter"]
                    .as_f64()
                    .ok_or_else(|| error("Mixed seam parameter must be numeric"))?,
            }
        }
        "reclock" => {
            let o = wire::object(raw, &["kind", "domain"])?;
            let d = o["domain"]
                .as_array()
                .filter(|v| v.len() == 2)
                .ok_or_else(|| error("Mixed edge clock needs two endpoints"))?;
            Operation::Reclock {
                domain: [
                    d[0].as_f64()
                        .ok_or_else(|| error("Mixed clock endpoint must be numeric"))?,
                    d[1].as_f64()
                        .ok_or_else(|| error("Mixed clock endpoint must be numeric"))?,
                ],
            }
        }
        "compact" => {
            wire::object(raw, &["kind"])?;
            Operation::Compact
        }
        _ => return Err(error("Unknown mixed operation kind")),
    };
    Ok(Command {
        parent,
        operation,
        key,
        uv_tolerance,
    })
}
fn step(s: &Step) -> Value {
    let l = s.limits;
    value_codec::json!({"command":command(&s.command),"limits":{"cells":l.max_cells,"pairs":l.max_pairs,"domain":l.max_domain_cells,"mass":l.mass_evaluations},"model":s.model(),"maps":maps::value(s)})
}
pub(super) fn document(h: &Sequence, extra: Option<&Step>) -> Value {
    value_codec::json!({"schema":"brep-edge-history/2","root":h.root,"steps":h.steps.iter().chain(extra).map(step).collect::<Vec<_>>()})
}
impl Sequence {
    pub fn to_json(&self, b: Budget) -> Result<String> {
        self.check_budget(b)?;
        let v = document(self, None);
        wire::bounded(&v, b.max_bytes)?;
        Ok(v.to_string())
    }
    pub fn from_json(text: &str, b: Budget) -> Result<Self> {
        b.validate()?;
        if text.len() > b.max_bytes {
            return Err(error("Mixed history document exceeds byte allowance"));
        }
        let v: Value = value_codec::from_str_strict(text)
            .map_err(|e| error(format!("Mixed history JSON: {e}")))?;
        wire::bounded(&v, b.max_bytes)?;
        let o = wire::object(&v, &["schema", "root", "steps"])?;
        match o["schema"].as_str() {
            Some("brep-edge-history/1") => {
                // Existing /1 owner performs the ONLY geometry replay. Move
                // its admitted actual steps and shared work into typed /2.
                let h = History::from_json(text, b)?;
                let next = Self {
                    root: h.root,
                    work: h.work,
                    steps: h
                        .steps
                        .into_iter()
                        .map(|s| {
                            let command = Command {
                                parent: s.source.1.edges[s.edge],
                                operation: Operation::Split {
                                    cuts: s.cuts.clone(),
                                },
                                key: s.key.clone(),
                                uv_tolerance: s.uv_tolerance,
                            };
                            Step {
                                command,
                                limits: s.limits,
                                applied: Applied::Split(s),
                            }
                        })
                        .collect(),
                };
                wire::bounded(&document(&next, None), b.max_bytes)?;
                return Ok(next);
            }
            Some("brep-edge-history/2") => {}
            _ => return Err(error("Unknown mixed edge-history schema")),
        }
        let steps = o["steps"]
            .as_array()
            .filter(|s| s.len() <= b.max_steps)
            .ok_or_else(|| error("Mixed history steps exceed allowance or are not an array"))?;
        let root = Model::from_value(o["root"].clone())
            .map_err(|e| error(format!("Mixed original model: {e}")))?;
        root.validate()?;
        if !wire::same(&root.to_value(), &o["root"]) {
            return Err(error(
                "Mixed root requires canonical complete identity tables",
            ));
        }
        let mut next = Self::new(&root, b)?;
        for entry in steps {
            let s = wire::object(entry, &["command", "limits", "model", "maps"])?;
            let caps = wire::limits(&s["limits"])?;
            if !wire::within(caps, next.allowance(b)?) {
                return Err(error(
                    "Stored mixed limits cannot increase caller allowances",
                ));
            }
            let command = parse_command(&s["command"])?;
            let actual = next.construct(command, caps)?;
            if !wire::same(&actual.model().to_value(), &s["model"])
                || !wire::same(&maps::value(&actual), &s["maps"])
            {
                return Err(error(
                    "Saved mixed model or ownership maps differ from fresh reconstruction",
                ));
            }
            next.accept(actual, b)?;
        }
        Ok(next)
    }
}
