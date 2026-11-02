//! Every revision, including hidden redo states, is freshly proved on import.
use super::*;
use value_codec::{Value, json};
const FORMAT: &str = "exact-full-round-history/1";
fn exact(v: &Value, keys: &[&str]) -> Result<()> {
    let o = v
        .as_object()
        .ok_or_else(|| error("Expected exact history object"))?;
    if o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)) {
        return Err(error("Unknown exact history fields"));
    }
    Ok(())
}
fn field<T: for<'a> value_codec::Deserialize<'a>>(v: &Value, key: &str) -> Result<T> {
    value_codec::from_value(
        v.get(key)
            .ok_or_else(|| error("Missing exact history field"))?
            .clone(),
    )
    .map_err(|e| error(e.to_string()))
}
fn bounded(v: &Value, max: usize) -> Result<()> {
    if value_codec::encode_binary(v)
        .map_err(|e| error(e.to_string()))?
        .len()
        > max
    {
        return Err(error("Exact history document exceeds byte budget"));
    }
    Ok(())
}
fn tag(edit: Edit) -> &'static str {
    match edit {
        Edit::Root => "root",
        Edit::RebaseSource => "rebase-source",
        Edit::Reselect => "reselect",
    }
}
pub fn export(history: &History) -> Result<Value> {
    let revisions=history.revisions.iter().map(|r|Ok(json!({"edit":tag(r.edit),"artifact":super::super::transport::export(r.body(),history.budget.max_bytes)?})))
        .collect::<Result<Vec<_>>>()?;
    let document = json!({"format":FORMAT,"cursor":history.cursor,"revisions":revisions});
    bounded(&document, history.budget.max_bytes)?;
    Ok(document)
}
pub fn restore(document: &Value, budget: Budget) -> Result<History> {
    budget.validate()?;
    bounded(document, budget.max_bytes)?;
    exact(document, &["format", "cursor", "revisions"])?;
    if document["format"].as_str() != Some(FORMAT) {
        return Err(error("Unknown exact history version"));
    }
    let rows = document["revisions"]
        .as_array()
        .ok_or_else(|| error("Expected exact history revisions"))?;
    let cursor: usize = field(document, "cursor")?;
    if rows.is_empty() || rows.len() > budget.max_revisions || cursor >= rows.len() {
        return Err(error("Invalid exact history revision count or cursor"));
    }
    let mut revisions: Vec<Revision> = Vec::new();
    let mut work = Work::default();
    for (i, row) in rows.iter().enumerate() {
        exact(row, &["edit", "artifact"])?;
        let edit = match (i, row["edit"].as_str()) {
            (0, Some("root")) => Edit::Root,
            (_, Some("rebase-source")) if i > 0 => Edit::RebaseSource,
            (_, Some("reselect")) if i > 0 => Edit::Reselect,
            _ => return Err(error("Invalid exact history operation order")),
        };
        let artifact = &row["artifact"];
        let l = &artifact["limits"];
        let limits = Limits {
            max_pair_projections: field(l, "maxPairProjections")?,
            max_atoms: field(l, "maxAtoms")?,
            max_ray_cells: field(l, "maxRayCells")?,
            refinements: field(l, "refinements")?,
            radius_halvings: field(l, "radiusHalvings")?,
        };
        if !within(limits, budget.remaining(work)?) {
            return Err(error(
                "Stored proof allowances exceed remaining caller history budget",
            ));
        }
        let body = super::super::transport::restore(artifact, budget.max_bytes)?;
        if let Some(previous) = revisions.last() {
            match edit {
                Edit::RebaseSource if ids(previous.body()) != ids(&body) => {
                    return Err(error("Source rebase changed selected authored face IDs"));
                }
                Edit::Reselect if previous.body().source() != body.source() => {
                    return Err(error("Selection edit changed its original source model"));
                }
                _ => {}
            }
        }
        work = add(work, used(&body))?;
        budget.remaining(work)?;
        revisions.push(Revision {
            edit,
            body: Arc::new(body),
        });
    }
    Ok(History {
        revisions,
        cursor,
        budget,
        work,
    })
}
