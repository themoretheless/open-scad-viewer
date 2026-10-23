//! Display-only refinement. Working topology and document geometry are never edited.
use super::{Result, Value, brep, encode, field, input};
use polygon_core::Mesh;
use std::collections::HashMap;

type V3 = [f64; 3];
fn length(p: V3) -> f64 {
    p[0].hypot(p[1]).hypot(p[2])
}
fn stats(mesh: &Mesh) -> Result<(Vec<V3>, Vec<V3>)> {
    let mut centers = Vec::new();
    let mut normals = Vec::new();
    for t in mesh.indices.chunks_exact(3) {
        let [a, b, c] = [mesh.point(t[0])?, mesh.point(t[1])?, mesh.point(t[2])?];
        let u: V3 = std::array::from_fn(|i| b[i] - a[i]);
        let v: V3 = std::array::from_fn(|i| c[i] - a[i]);
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = length(n);
        centers.push(std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.));
        normals.push(n.map(|x| x / if len == 0. { 1. } else { len }));
    }
    if centers
        .iter()
        .chain(&normals)
        .flatten()
        .any(|v| !v.is_finite())
    {
        return Err(input("Display geometry exceeds finite numeric range."));
    }
    Ok((centers, normals))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FixedKey {
    Decimal(bool, u128),
    Large(u64),
}
/// Equality of ECMAScript toFixed(5), using the exact binary rational before
/// decimal rounding. Multiplying f64 by 100000 first changes half-way cases.
fn fixed_key(x: f64) -> FixedKey {
    if x.abs() >= 1e21 {
        return FixedKey::Large(x.to_bits());
    }
    let bits = x.abs().to_bits();
    let e = ((bits >> 52) & 2047) as i32;
    let m = (bits & ((1u64 << 52) - 1)) | if e == 0 { 0 } else { 1u64 << 52 };
    let exponent = if e == 0 { -1074 } else { e - 1023 - 52 };
    let numerator = m as u128 * 100_000;
    let rounded = if exponent >= 0 {
        numerator << exponent
    } else {
        let shift = (-exponent) as u32;
        if shift >= 128 {
            0
        } else {
            let floor = numerator >> shift;
            let remainder = numerator & ((1u128 << shift) - 1);
            floor + u128::from(remainder >= 1u128 << (shift - 1))
        }
    };
    FixedKey::Decimal(x < 0., rounded)
}
fn smooth(mesh: &Mesh, flat: &[V3]) -> Vec<V3> {
    let keys: Vec<[FixedKey; 3]> = mesh
        .positions
        .chunks_exact(3)
        .map(|p| [fixed_key(p[0]), fixed_key(p[1]), fixed_key(p[2])])
        .collect();
    // Keep incident normals separate so a sharp crease cannot tilt a planar cap.
    let mut incident: HashMap<[FixedKey; 3], Vec<V3>> = HashMap::new();
    for (t, n) in mesh.indices.chunks_exact(3).zip(flat) {
        for &index in t {
            incident.entry(keys[index]).or_default().push(*n);
        }
    }
    mesh.indices
        .chunks_exact(3)
        .zip(flat)
        .map(|(t, n)| {
            let mut sum = [0.; 3];
            for &index in t {
                for candidate in &incident[&keys[index]] {
                    // Smooth only within 60 degrees of this triangle's geometric normal.
                    if (0..3).map(|i| candidate[i] * n[i]).sum::<f64>() > 0.5 {
                        for i in 0..3 {
                            sum[i] += candidate[i];
                        }
                    }
                }
            }
            let len = length(sum);
            if len > 1e-9 { sum.map(|x| x / len) } else { *n }
        })
        .collect()
}
fn result(
    mesh: Mesh,
    map: Option<Vec<usize>>,
    normals: Vec<V3>,
    closed: Option<Vec<bool>>,
    work_closed: Option<Vec<bool>>,
) -> Result<Value> {
    let mut value = value_codec::Map::new();
    value.insert("closed".into(), encode(closed)?);
    value.insert("workClosed".into(), encode(work_closed)?);
    value.insert("mesh".into(), encode(mesh)?);
    value.insert("map".into(), encode(map)?);
    value.insert("normals".into(), encode(normals)?);
    Ok(Value::Object(value))
}
pub fn prepare(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let segments: usize = field(&v, "segments")?;
    let budget: usize = field(&v, "maxTriangles")?;
    if !(1..=32).contains(&segments) || !(1..=4000).contains(&budget) {
        return Err(input("Invalid display refinement budget."));
    }
    let (work_centers, work_normals) = stats(&mesh)?;
    let model: Option<brep_core::Model> = if v.get("brep").is_some() {
        Some(field(&v, "brep")?)
    } else {
        None
    };
    let mut work_closed = None;
    if let Some(model) = model {
        // Only exact reproduction establishes ownership of an older working mesh.
        // Do not use the approximate picking correspondence for visibility.
        if model.shells.iter().any(|s| s.closed) && model.shells.iter().any(|s| !s.closed) {
            for detail in [segments, 4, 1].into_iter().chain(2..=32) {
                if let Ok(tess) = brep::nurbs(&model, detail) {
                    if tess.built.mesh.positions == mesh.positions
                        && tess.built.mesh.indices == mesh.indices
                    {
                        work_closed = tess.closed_triangles;
                        break;
                    }
                }
            }
        }
        // Refinement failure keeps the working mesh, just like the existing UI.
        if let Ok(dense) = brep::nurbs(&model, segments) {
            let closed = dense.closed_triangles;
            let dense = dense.built.mesh;
            let count = dense.indices.len() / 3;
            if count <= budget
                && !work_centers.is_empty()
                && count.saturating_mul(work_centers.len()) <= 16_000_000
            {
                let (centers, normals) = stats(&dense)?;
                let map = centers
                    .iter()
                    .zip(&normals)
                    .map(|(c, n)| {
                        let mut best = 0;
                        let mut score = f64::INFINITY;
                        for (i, (w, wn)) in work_centers.iter().zip(&work_normals).enumerate() {
                            let distance = length(std::array::from_fn(|k| c[k] - w[k]));
                            let facing = 1. - (n[0] * wn[0] + n[1] * wn[1] + n[2] * wn[2]);
                            let candidate = distance * (1. + facing * 4.);
                            if candidate < score {
                                score = candidate;
                                best = i;
                            }
                        }
                        best
                    })
                    .collect();
                let normals = smooth(&dense, &normals);
                return result(dense, Some(map), normals, closed, work_closed);
            }
        }
    }
    result(mesh, None, work_normals, work_closed.clone(), work_closed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    #[test]
    fn mixed_shell_display_keeps_exact_triangle_ownership() {
        let mut model = brep_core::step_interchange_v3::import_step_v9(include_str!(
            "../../../tests/fixtures/step-v6/self-authored-mixed-unit-product-assembly.step"
        ))
        .unwrap()
        .0;
        model.bodies.pop();
        model.1.bodies.pop();
        model.shells[1].closed = false;
        let tess = brep::nurbs(&model, 1).unwrap();
        let expected: Vec<bool> = tess
            .face_ids
            .iter()
            .map(|face| model.shells[0].faces.iter().any(|f| f.face == *face))
            .collect();
        assert!(expected.contains(&true) && expected.contains(&false));
        let output =
            prepare(json!({"mesh":tess.built.mesh,"brep":model,"segments":12,"maxTriangles":4000}))
                .unwrap();
        assert_eq!(field::<Vec<bool>>(&output, "closed").unwrap(), expected);
        assert_eq!(field::<Vec<bool>>(&output, "workClosed").unwrap(), expected);
        let mut limited =
            json!({"mesh":tess.built.mesh,"brep":model,"segments":12,"maxTriangles":1});
        let fallback = prepare(limited.clone()).unwrap();
        assert!(fallback["map"].is_null());
        assert_eq!(field::<Vec<bool>>(&fallback, "closed").unwrap(), expected);
        limited["mesh"]["positions"][0] = json!(0.123);
        let unrelated = prepare(limited).unwrap();
        assert!(unrelated["closed"].is_null());
        assert!(unrelated["workClosed"].is_null());
    }
    #[test]
    fn fixed_keys_follow_binary_rational_decimal_rounding() {
        assert_eq!(fixed_key(0.000005), FixedKey::Decimal(false, 1));
        assert_eq!(fixed_key(1.000005), FixedKey::Decimal(false, 100001));
        assert_eq!(fixed_key(1.234565), FixedKey::Decimal(false, 123456));
        assert_eq!(fixed_key(-0.), fixed_key(0.));
        assert_ne!(fixed_key(-0.0000001), fixed_key(0.));
        assert_eq!(fixed_key(f64::from_bits(1)), FixedKey::Decimal(false, 0));
        assert_ne!(fixed_key(1e21), fixed_key(-1e21));
    }
    #[test]
    fn coarse_normals_preserve_degenerate_triangles_and_refuse_bad_indices() {
        let mut v = json!({"mesh":{"positions":[0.,0.,0.,1.,0.,0.,0.,1.,0.],"indices":[0,1,2,0,0,0]},"segments":12,"maxTriangles":4000});
        let output = prepare(v.clone()).unwrap();
        assert_eq!(
            field::<Vec<V3>>(&output, "normals").unwrap(),
            vec![[0., 0., 1.], [0.; 3]]
        );
        assert!(output["map"].is_null());
        v["mesh"]["indices"] = json!([0, 1, 3]);
        assert!(prepare(v).is_err());
    }
    #[test]
    fn planar_box_preserves_working_triangle_correspondence() {
        let model = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
        let mesh = brep::nurbs(&model, 1).unwrap().built.mesh;
        let before = encode(&mesh).unwrap();
        let count = mesh.indices.len() / 3;
        let output = prepare(
            value_codec::json!({"mesh":mesh,"brep":model,"segments":12,"maxTriangles":4000}),
        )
        .unwrap();
        assert_eq!(output["mesh"]["positions"], before["positions"]);
        assert_eq!(output["mesh"]["indices"], before["indices"]);
        let original: Mesh = field(&json!({"mesh":before}), "mesh").unwrap();
        assert_eq!(
            field::<Vec<V3>>(&output, "normals").unwrap(),
            stats(&original).unwrap().1
        );
        assert_eq!(
            field::<Vec<usize>>(&output, "map").unwrap(),
            (0..count).collect::<Vec<_>>()
        );
    }
    #[test]
    fn cylinder_caps_stay_flat_while_walls_smooth() {
        let model = brep_core::analytic::cylinder(5., 8.).unwrap();
        let mesh = brep::nurbs(&model, 3).unwrap().built.mesh;
        let output =
            prepare(json!({"mesh":mesh,"brep":model,"segments":12,"maxTriangles":4000})).unwrap();
        let dense: Mesh = field(&output, "mesh").unwrap();
        let normals: Vec<V3> = field(&output, "normals").unwrap();
        let (centers, geometric) = stats(&dense).unwrap();
        let mut caps = 0;
        let mut walls = 0;
        for ((center, flat), normal) in centers.iter().zip(&geometric).zip(normals) {
            if flat[2].abs() > 0.99 {
                caps += 1;
                assert!(normal[0].abs() < 1e-12 && normal[1].abs() < 1e-12);
                assert!((normal[2] - flat[2]).abs() < 1e-12);
            } else {
                walls += 1;
                assert!(normal[2].abs() < 1e-12);
                let radial = center[0].hypot(center[1]);
                assert!((normal[0] * center[0] + normal[1] * center[1]) / radial > 0.99);
            }
        }
        assert!(caps > 0 && walls > 0);
    }
    #[test]
    fn sphere_refinement_maps_to_working_triangles_and_honors_budget() {
        let model = brep_core::analytic::sphere(5.).unwrap();
        let mesh = brep::nurbs(&model, 3).unwrap().built.mesh;
        let count = mesh.indices.len() / 3;
        let v = json!({"mesh":mesh,"brep":model,"segments":6,"maxTriangles":4000});
        let output = prepare(v.clone()).unwrap();
        let dense: Mesh = field(&output, "mesh").unwrap();
        let map: Vec<usize> = field(&output, "map").unwrap();
        let normals: Vec<V3> = field(&output, "normals").unwrap();
        assert_eq!(map.len(), dense.indices.len() / 3);
        assert_eq!(normals.len(), map.len());
        assert!(map.iter().all(|&i| i < count));
        assert!(normals.iter().all(|n| (length(*n) - 1.).abs() < 1e-12));
        let mut limited = v;
        limited["maxTriangles"] = json!(1);
        let fallback = prepare(limited).unwrap();
        assert!(fallback["map"].is_null());
        assert_eq!(
            field::<Mesh>(&fallback, "mesh").unwrap().indices.len() / 3,
            count
        );
    }
}
