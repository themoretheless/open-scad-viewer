//! Native translation patterns, with the complete batch returned atomically.
use super::{Result, Value, cad_body_affine, encode, field, input};
use polygon_core::Mesh;
// All pattern instances have the same serialized shape: affine translation
// changes numeric values only. Measure one prepared group before expansion.
fn footprint(value: &Value) -> (usize, usize, usize) {
    match value {
        Value::Null | Value::Bool(_) => (1, 1, 0),
        Value::Number(_) => (9, 1, 0),
        Value::String(s) => (5usize.saturating_add(s.len()), 1, 0),
        Value::Array(values) => values
            .iter()
            .fold((5usize, 1usize, 0usize), |(b, n, d), v| {
                let (vb, vn, vd) = footprint(v);
                (b.saturating_add(vb), n.saturating_add(vn), d.max(vd + 1))
            }),
        Value::Object(values) => {
            values
                .iter()
                .fold((5usize, 1usize, 0usize), |(b, n, d), (k, v)| {
                    let (vb, vn, vd) = footprint(v);
                    (
                        b.saturating_add(5)
                            .saturating_add(k.len())
                            .saturating_add(vb),
                        n.saturating_add(1).saturating_add(vn),
                        d.max(vd + 1),
                    )
                })
        }
    }
}
fn admit_expansion(group: &Value, count: usize) -> Result<()> {
    let (bytes, nodes, depth) = footprint(group);
    // Reserve space for the outer group array, wire header and response envelope.
    if bytes.saturating_mul(count) > 32 * 1024 * 1024 - 1024
        || nodes.saturating_mul(count) > 4_000_000 - 32
        || depth > 125
    {
        return Err(input(
            "Pattern expansion exceeds binary transport capacity.",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn footprint_matches_wire_and_counts_object_keys() {
        let value = value_codec::json!({"name":"тело","data":[1.,true,Value::Null]});
        let (bytes, nodes, depth) = footprint(&value);
        assert_eq!(bytes + 4, value_codec::encode_binary(&value).unwrap().len());
        assert_eq!(nodes, 8);
        assert_eq!(depth, 2);
        assert!(admit_expansion(&value, usize::MAX).is_err());
    }
}
pub fn pattern(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    let count: usize = field(&v, "count")?;
    if !(2..=100).contains(&count) {
        return Err(input("Pattern count must be 2–100."));
    }
    let meshes = bodies
        .iter()
        .map(|b| field::<Mesh>(b, "mesh"))
        .collect::<Result<Vec<_>>>()?;
    let positions = meshes
        .iter()
        .map(|m| m.positions.clone())
        .collect::<Vec<_>>();
    let (min, max) = polygon_core::scene_flatten::bounds(&positions)?;
    let center: [f64; 3] = std::array::from_fn(|k| min[k] * 0.5 + max[k] * 0.5);
    let axis: [f64; 3] = field(&v, "axis")?;
    let amount: f64 = field(&v, "amount")?;
    if !axis.iter().chain([&amount]).all(|x| x.is_finite()) {
        return Err(input("Invalid pattern parameters."));
    }
    let magnitude = axis.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude == 0. {
        return Err(input("Zero direction"));
    }
    let n = axis.map(|x| x / magnitude);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    let n = n.map(|x| x / length);
    let path = if let Some(sketch) = v.get("path").filter(|x| !x.is_null()) {
        let points: Vec<[f64; 2]> = field(sketch, "points")?;
        let (origin, u, w): ([f64; 3], [f64; 3], [f64; 3]) = match sketch.get("plane") {
            Some(p) => (field(p, "origin")?, field(p, "u")?, field(p, "v")?),
            None => ([0.; 3], [1., 0., 0.], [0., 1., 0.]),
        };
        let points: Vec<[f64; 3]> = points
            .iter()
            .map(|p| std::array::from_fn(|k| origin[k] + p[0] * u[k] + p[1] * w[k]))
            .collect();
        if points.len() < 2 {
            return Err(input("A path requires at least two points."));
        }
        if !points.iter().flatten().all(|x| x.is_finite()) {
            return Err(input("Nonfinite pattern path."));
        }
        let lengths = points
            .windows(2)
            .map(|p| {
                (p[1][0] - p[0][0])
                    .hypot(p[1][1] - p[0][1])
                    .hypot(p[1][2] - p[0][2])
            })
            .collect::<Vec<_>>();
        let total = lengths.iter().sum::<f64>();
        if !total.is_finite() || total < 1e-8 {
            return Err(input("Zero-length or nonfinite path."));
        }
        Some((points, lengths, total))
    } else {
        None
    };
    let mut groups = Vec::with_capacity(count);
    for i in 0..count {
        let delta = if let Some((points, lengths, total)) = &path {
            let mut distance = i as f64 / (count - 1) as f64 * total;
            let mut point = *points.last().unwrap();
            for (j, &length) in lengths.iter().enumerate() {
                if distance <= length || j == lengths.len() - 1 {
                    let t = if length > 0. { distance / length } else { 0. };
                    point = std::array::from_fn(|k| {
                        points[j][k] + (points[j + 1][k] - points[j][k]) * t
                    });
                    break;
                }
                distance -= length;
            }
            std::array::from_fn(|k| point[k] - center[k])
        } else {
            n.map(|x| x * amount * i as f64)
        };
        let mut matrix = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        for k in 0..3 {
            matrix[k][3] = delta[k];
        }
        let group = cad_body_affine::apply(bodies.clone(), meshes.clone(), matrix)?;
        if i == 0 {
            admit_expansion(&group, count)?;
        }
        groups.push(group);
    }
    encode(groups)
}
