//! Display-only planar grouping; this does not construct authored B-rep topology.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
use std::collections::{BTreeMap, BTreeSet, HashMap};
struct Face {
    triangles: Vec<usize>,
    normal: [f64; 3],
    offset: f64,
    vertices: Vec<usize>,
    center: [f64; 3],
}
struct Edge {
    a: usize,
    b: usize,
    faces: Vec<usize>,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
// Fixed-seven decimal rounding, with JavaScript's ties-away rule for the
// legacy display seam key. Multiplying in binary64 first would lose ties.
fn seam_coordinate(x: f64) -> String {
    if x.abs() >= 1e21 {
        return format!("{x:.7}");
    }
    let bits = x.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let power = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    };
    let numerator = mantissa as u128 * 10_000_000;
    let rounded = if power >= 0 {
        numerator << power
    } else {
        let shift = (-power) as u32;
        if shift >= 128 {
            0
        } else {
            (numerator + (1u128 << (shift - 1))) >> shift
        }
    };
    format!(
        "{}{}.{:07}",
        if x < 0. { "-" } else { "" },
        rounded / 10_000_000,
        rounded % 10_000_000
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_seam_ties_follow_legacy_rounding() {
        assert_eq!(seam_coordinate(1. / 256.), "0.0039063");
        assert_eq!(seam_coordinate(-1. / 256.), "-0.0039063");
        assert_eq!(seam_coordinate(-0.), "0.0000000");
        assert_eq!(seam_coordinate(-f64::from_bits(1)), "-0.0000000");
        assert_eq!(seam_coordinate(123.5), "123.5000000");
    }
}
pub fn topology(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let points = mesh
        .positions
        .chunks_exact(3)
        .map(|p| [p[0], p[1], p[2]])
        .collect::<Vec<_>>();
    let mut faces: Vec<Face> = Vec::new();
    let mut triangle_faces = vec![None; mesh.indices.len() / 3];
    let mut comparisons = 0usize;
    // Planes are looked up through a grid on (normal, offset) so a curved
    // body, where nearly every triangle is its own plane, stays linear: two
    // planes within the match tolerance (1e-8 on the normal dot, 1e-6 on the
    // offset) always land in the same or an adjacent cell, so the candidate
    // scan over the 3^4 neighbouring cells is exhaustive.
    const NORMAL_CELL: f64 = 1e-3;
    const OFFSET_CELL: f64 = 1e-4;
    let cell = |normal: [f64; 3], offset: f64| -> [i64; 4] {
        [
            (normal[0] / NORMAL_CELL).floor() as i64,
            (normal[1] / NORMAL_CELL).floor() as i64,
            (normal[2] / NORMAL_CELL).floor() as i64,
            (offset / OFFSET_CELL).floor() as i64,
        ]
    };
    let mut grid: HashMap<[i64; 4], Vec<usize>> = HashMap::new();
    for (i, ids) in mesh.indices.chunks_exact(3).enumerate() {
        let a = points[ids[0]];
        let b = points[ids[1]];
        let c = points[ids[2]];
        let u: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        let w: [f64; 3] = std::array::from_fn(|k| c[k] - a[k]);
        let raw = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let length = raw[0].hypot(raw[1]).hypot(raw[2]);
        if !length.is_finite() {
            return Err(input("Nonfinite display face normal."));
        }
        if length < 1e-9 {
            continue;
        }
        let normal = raw.map(|x| x / length);
        let offset = dot(normal, a);
        if !offset.is_finite() {
            return Err(input("Nonfinite display plane."));
        }
        let home = cell(normal, offset);
        let mut found = None;
        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    for dw in -1..=1 {
                        let key = [home[0] + dx, home[1] + dy, home[2] + dz, home[3] + dw];
                        for &j in grid.get(&key).map(Vec::as_slice).unwrap_or(&[]) {
                            comparisons += 1;
                            if comparisons > 20_000_000 {
                                return Err(input(
                                    "Display topology exceeds 20000000 plane comparisons.",
                                ));
                            }
                            let f = &faces[j];
                            if dot(f.normal, normal) > 1. - 1e-8 && (f.offset - offset).abs() < 1e-6
                            {
                                found = Some(j);
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
        let index = found.unwrap_or_else(|| {
            faces.push(Face {
                triangles: Vec::new(),
                normal,
                offset,
                vertices: Vec::new(),
                center: [0.; 3],
            });
            grid.entry(home).or_default().push(faces.len() - 1);
            faces.len() - 1
        });
        faces[index].triangles.push(i);
        faces[index].vertices.extend(ids);
        triangle_faces[i] = Some(index);
    }
    for face in &mut faces {
        let mut seen = BTreeSet::new();
        face.vertices.retain(|v| seen.insert(*v));
        face.center = std::array::from_fn(|k| {
            face.vertices.iter().map(|&i| points[i][k]).sum::<f64>() / face.vertices.len() as f64
        });
        if !face.center.iter().all(|x| x.is_finite()) {
            return Err(input("Nonfinite display face center."));
        }
    }
    // Keep the legacy decimal seam grouping for display only, with first-seen order.
    let keys = points
        .iter()
        .map(|p| {
            p.iter()
                .map(|&x| seam_coordinate(x))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>();
    let mut lookup = BTreeMap::new();
    let mut edges: Vec<Edge> = Vec::new();
    for (i, ids) in mesh.indices.chunks_exact(3).enumerate() {
        let Some(face) = triangle_faces[i] else {
            continue;
        };
        for k in 0..3 {
            let (mut a, mut b) = (ids[k], ids[(k + 1) % 3]);
            if keys[a] > keys[b] {
                std::mem::swap(&mut a, &mut b);
            }
            let key = (keys[a].clone(), keys[b].clone());
            let index = *lookup.entry(key).or_insert_with(|| {
                edges.push(Edge {
                    a,
                    b,
                    faces: Vec::new(),
                });
                edges.len() - 1
            });
            edges[index].faces.push(face);
        }
    }
    let faces=faces.into_iter().map(|f|value_codec::json!({"triangles":f.triangles,"normal":f.normal,"offset":f.offset,"vertices":f.vertices,"center":f.center})).collect::<Vec<_>>();
    let edges = edges
        .into_iter()
        .filter(|e| e.faces.len() == 2 && e.faces[0] != e.faces[1])
        .map(|e| value_codec::json!({"a":e.a,"b":e.b,"faces":e.faces}))
        .collect::<Vec<_>>();
    encode(value_codec::json!({"faces":faces,"edges":edges}))
}

pub fn face_plane(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let face: Value = field(&v, "face")?;
    let vertices: Vec<usize> = field(&face, "vertices")?;
    let normal: [f64; 3] = field(&face, "normal")?;
    if vertices.is_empty() || vertices.iter().any(|&i| i >= mesh.positions.len() / 3) {
        return Err(input("Invalid face vertices."));
    }
    let point = |i: usize| -> [f64; 3] { std::array::from_fn(|k| mesh.positions[i * 3 + k]) };
    let origin = point(vertices[0]);
    let mut direction = None;
    for &i in &vertices {
        let p = point(i);
        let d: [f64; 3] = std::array::from_fn(|k| p[k] - origin[k]);
        if d[0].hypot(d[1]).hypot(d[2]) > 1e-7 {
            direction = Some(d);
            break;
        }
    }
    let unit = |p: [f64; 3]| -> Result<[f64; 3]> {
        let length = p[0].hypot(p[1]).hypot(p[2]);
        if !length.is_finite() || length < 1e-9 {
            return Err(input("Degenerate face workplane."));
        }
        Ok(p.map(|x| x / length))
    };
    let normal = unit(normal)?;
    let u = unit(direction.ok_or_else(|| input("Degenerate face workplane."))?)?;
    if dot(normal, u).abs() > 1e-6 {
        return Err(input("Face direction is not tangent to its support."));
    }
    let w = unit([
        normal[1] * u[2] - normal[2] * u[1],
        normal[2] * u[0] - normal[0] * u[2],
        normal[0] * u[1] - normal[1] * u[0],
    ])?;
    for &i in &vertices {
        let p = point(i);
        let d: [f64; 3] = std::array::from_fn(|k| p[k] - origin[k]);
        let residual = dot(normal, d);
        if !residual.is_finite() || residual.abs() > 1e-6 {
            return Err(input("Face vertices are not coplanar."));
        }
    }
    encode(value_codec::json!({"origin":origin,"u":u,"v":w}))
}
