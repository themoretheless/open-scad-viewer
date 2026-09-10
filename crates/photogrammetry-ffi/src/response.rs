//! Borrowed photo output adapter for the existing MGV1 protocol.
//!
//! Large numeric arrays bypass the intermediate `Value` tree. Small metadata is
//! still encoded by value-codec. Preflight includes the envelope and all keys,
//! and reserves the exact output size before writing any geometry.
use super::{
    input, ResponseBudget, Result, Value, DEPTH_LIMIT, ITEM_LIMIT, LIMIT, TRANSPORT_ERROR,
};
use photogrammetry_core::{dense::Surface, Point, Reconstruction};
use value_codec::json;

enum Field<'a> {
    Value(&'a Value),
    Object(&'a [(&'a str, Field<'a>)]),
    Positions(&'a [[f64; 3]]),
    Colors(&'a [[u8; 3]]),
    Triangles(&'a [[u32; 3]]),
    PointPositions(&'a [Point]),
    PointColors(&'a [Point]),
}

fn reserve_many(
    budget: &mut ResponseBudget,
    depth: usize,
    items: usize,
    bytes: usize,
) -> Result<()> {
    if depth > DEPTH_LIMIT
        || items > ITEM_LIMIT.saturating_sub(budget.items)
        || bytes > LIMIT.saturating_sub(budget.bytes)
    {
        return Err(input(TRANSPORT_ERROR));
    }
    budget.items += items;
    budget.bytes += bytes;
    Ok(())
}

fn triples_budget(
    budget: &mut ResponseBudget,
    depth: usize,
    rows: usize,
    nulls: usize,
) -> Result<()> {
    // Outer array, then N arrays with three numbers each. Nonfinite f64 follows
    // json!'s established representation (null, one byte instead of nine).
    budget.reserve(depth, 5)?;
    if rows == 0 {
        return Ok(());
    }
    let items = rows.checked_mul(4).ok_or(TRANSPORT_ERROR)?;
    let bytes = rows
        .checked_mul(32)
        .and_then(|bytes| {
            nulls
                .checked_mul(8)
                .and_then(|nulls| bytes.checked_sub(nulls))
        })
        .ok_or(TRANSPORT_ERROR)?;
    reserve_many(budget, depth + 2, items, bytes)
}

fn float_budget(
    budget: &mut ResponseBudget,
    depth: usize,
    values: impl ExactSizeIterator<Item = [f64; 3]>,
) -> Result<()> {
    let rows = values.len();
    // Reject oversized arrays before walking their contents.
    if rows > ITEM_LIMIT / 4 {
        return Err(input(TRANSPORT_ERROR));
    }
    let nulls = values.flatten().filter(|v| !v.is_finite()).count();
    triples_budget(budget, depth, rows, nulls)
}

impl Field<'_> {
    fn measure(&self, budget: &mut ResponseBudget, depth: usize) -> Result<()> {
        match self {
            Self::Value(value) => budget.visit(value, depth),
            Self::Object(fields) => {
                budget.reserve(depth, 5)?;
                for (key, field) in *fields {
                    budget.reserve(depth + 1, 5 + key.len())?;
                    field.measure(budget, depth + 1)?;
                }
                Ok(())
            }
            Self::Positions(values) => float_budget(budget, depth, values.iter().copied()),
            Self::Colors(values) => triples_budget(budget, depth, values.len(), 0),
            Self::Triangles(values) => triples_budget(budget, depth, values.len(), 0),
            Self::PointPositions(points) => {
                float_budget(budget, depth, points.iter().map(|p| p.position))
            }
            Self::PointColors(points) => triples_budget(budget, depth, points.len(), 0),
        }
    }

    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        match self {
            Self::Value(value) => {
                // The shared encoder remains authoritative for arbitrary metadata.
                let bytes = value_codec::encode_binary(value).map_err(|e| e.to_string())?;
                out.extend_from_slice(&bytes[4..]);
            }
            Self::Object(fields) => {
                container(out, 6, fields.len());
                for (key, field) in *fields {
                    container(out, 4, key.len());
                    out.extend_from_slice(key.as_bytes());
                    field.write(out)?;
                }
            }
            Self::Positions(values) => float_triples(out, values.iter().copied()),
            Self::Colors(values) => unsigned_triples(out, values.iter().map(|v| v.map(u64::from))),
            Self::Triangles(values) => {
                unsigned_triples(out, values.iter().map(|v| v.map(u64::from)))
            }
            Self::PointPositions(points) => float_triples(out, points.iter().map(|p| p.position)),
            Self::PointColors(points) => {
                unsigned_triples(out, points.iter().map(|p| p.color.map(u64::from)))
            }
        }
        Ok(())
    }
}

fn container(out: &mut Vec<u8>, tag: u8, len: usize) {
    // Preflight bounds every count/string to the 32 MiB / four-million-item caps.
    out.push(tag);
    out.extend_from_slice(&(len as u32).to_le_bytes());
}

fn numeric_triple(tag: u8, values: [u64; 3]) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[0] = 5; // array
    bytes[1] = 3; // three elements, little endian u32
    for (index, value) in values.iter().enumerate() {
        let offset = 5 + index * 9;
        bytes[offset] = tag;
        bytes[offset + 1..offset + 9].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn float_triples(out: &mut Vec<u8>, values: impl ExactSizeIterator<Item = [f64; 3]>) {
    container(out, 5, values.len());
    for triple in values {
        if triple.iter().all(|value| value.is_finite()) {
            out.extend_from_slice(&numeric_triple(3, triple.map(f64::to_bits)));
        } else {
            container(out, 5, 3);
            for value in triple {
                if value.is_finite() {
                    out.push(3);
                    out.extend_from_slice(&value.to_le_bytes());
                } else {
                    out.push(0);
                }
            }
        }
    }
}

fn unsigned_triples(out: &mut Vec<u8>, values: impl ExactSizeIterator<Item = [u64; 3]>) {
    container(out, 5, values.len());
    for triple in values {
        out.extend_from_slice(&numeric_triple(7, triple));
    }
}

fn encode(payload: Field<'_>) -> Result<Vec<u8>> {
    // BTreeMap order of the original json! response: "ok", then "value".
    let fields = [("ok", Field::Value(&Value::Bool(true))), ("value", payload)];
    let response = Field::Object(&fields);
    let mut budget = ResponseBudget { bytes: 4, items: 0 };
    response.measure(&mut budget, 0)?;
    let mut bytes = Vec::with_capacity(budget.bytes);
    bytes.extend_from_slice(b"MGV1");
    response.write(&mut bytes)?;
    debug_assert_eq!(bytes.len(), budget.bytes);
    Ok(bytes)
}

pub(super) fn value(value: &Value) -> Result<Vec<u8>> {
    encode(Field::Value(value))
}

pub(super) fn surface(mesh: &Surface, diagnostics: Option<&Value>) -> Result<Vec<u8>> {
    let fields = [
        ("colors", Field::Colors(&mesh.colors)),
        ("positions", Field::Positions(&mesh.positions)),
        ("triangles", Field::Triangles(&mesh.triangles)),
    ];
    if let Some(diagnostics) = diagnostics {
        let fields = [
            ("colors", Field::Colors(&mesh.colors)),
            ("denseDiagnostics", Field::Value(diagnostics)),
            ("positions", Field::Positions(&mesh.positions)),
            ("triangles", Field::Triangles(&mesh.triangles)),
        ];
        encode(Field::Object(&fields))
    } else {
        encode(Field::Object(&fields))
    }
}

pub(super) fn sparse(reconstruction: &Reconstruction, diagnostics: &Value) -> Result<Vec<u8>> {
    let cameras =
        reconstruction
            .cameras
            .iter()
            .enumerate()
            .filter_map(|(image, camera)| {
                camera.as_ref().map(|camera| json!({
            "image": image, "rotation": camera.rotation, "translation": camera.translation,
            "focal": camera.focal, "cx": camera.cx, "cy": camera.cy,
        }))
            })
            .collect::<Vec<_>>();
    let cameras = Value::Array(cameras);
    let input_images = json!(reconstruction.input_images);
    let rmse = json!(reconstruction.reprojection_rmse);
    let fields = [
        ("cameras", Field::Value(&cameras)),
        ("colors", Field::PointColors(&reconstruction.points)),
        ("diagnostics", Field::Value(diagnostics)),
        ("inputImages", Field::Value(&input_images)),
        ("positions", Field::PointPositions(&reconstruction.points)),
        ("reprojectionRmse", Field::Value(&rmse)),
        ("triangles", Field::Triangles(&[])),
    ];
    encode(Field::Object(&fields))
}

#[cfg(test)]
mod tests;
