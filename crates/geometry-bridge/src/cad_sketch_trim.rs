//! Record adapters for typed local-coordinate sketch trim and extension.
use super::{Result, Value, encode, field, input};
use planar_geometry::sketch_trim::{self, Boundary, End};
fn boundaries(v: &Value) -> Result<Vec<Boundary>> {
    field::<Vec<Value>>(v, "boundaries")?
        .into_iter()
        .map(|s| {
            Ok(Boundary {
                id: field(&s, "id")?,
                points: field(&s, "points")?,
                closed: field(&s, "closed")?,
            })
        })
        .collect()
}
pub fn trim(v: Value) -> Result<Value> {
    let sketch: Value = field(&v, "sketch")?;
    let id: String = field(&sketch, "id")?;
    let chains = sketch_trim::trim(
        &field::<Vec<[f64; 2]>>(&sketch, "points")?,
        field(&sketch, "closed")?,
        &id,
        field(&v, "edge")?,
        field(&v, "at")?,
        &boundaries(&v)?,
    )?;
    let mut result = Vec::with_capacity(chains.len());
    for (i, points) in chains.into_iter().enumerate() {
        let mut s = sketch
            .as_object()
            .ok_or_else(|| input("Expected sketch record"))?
            .clone();
        s.remove("analytic");
        s.remove("dimensions");
        s.insert(
            "id".into(),
            encode(if i == 0 {
                id.clone()
            } else {
                format!("{id}-trim")
            })?,
        );
        s.insert("closed".into(), encode(false)?);
        s.insert("points".into(), encode(points)?);
        result.push(Value::Object(s));
    }
    encode(result)
}
pub fn extend(v: Value) -> Result<Value> {
    let mut sketch: Value = field(&v, "sketch")?;
    if field::<bool>(&sketch, "closed")? || sketch.get("analytic").is_some() {
        return Err(input("Extend requires an open polyline."));
    }
    let end: String = field(&v, "end")?;
    let end = match end.as_str() {
        "start" => End::Start,
        "end" => End::End,
        _ => return Err(input("Select a polyline endpoint.")),
    };
    let points = sketch_trim::extend(
        &field::<Vec<[f64; 2]>>(&sketch, "points")?,
        &field::<String>(&sketch, "id")?,
        end,
        &boundaries(&v)?,
    )?;
    sketch["points"] = encode(points)?;
    Ok(sketch)
}
