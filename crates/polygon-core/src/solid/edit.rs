//! Mesh-native edits; topology checks do not certify absence of self-intersections.
use crate::{BuiltMesh, Mesh, Result, error};
use std::collections::{BTreeMap, BTreeSet};
fn finish(mesh: Mesh) -> Result<BuiltMesh> {
    let report = mesh.inspect()?;
    if report.degenerate_triangles > 0
        || report.non_manifold_edges > 0
        || report.orientation_conflicts > 0
    {
        return Err(error("Edit creates invalid triangle topology"));
    }
    Ok(BuiltMesh { mesh, report })
}
pub fn deform(mesh: &Mesh, operation: &geometry_ops::Deformation) -> Result<BuiltMesh> {
    mesh.validate()?;
    operation.validate()?;
    let mut result = mesh.clone();
    result.positions = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| operation.apply([p[0], p[1], p[2]]))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    finish(result)
}
pub fn brush(mesh: &Mesh, brush: &geometry_ops::Brush) -> Result<BuiltMesh> {
    mesh.validate()?;
    brush.validate()?;
    let mut result = mesh.clone();
    result.positions = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| brush.apply([p[0], p[1], p[2]]))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    finish(result)
}
/// Extrudes selected triangles together, retaining neighboring faces and adding
/// walls around every boundary loop. Selecting the entire closed mesh is rejected.
pub fn extrude_faces(mesh: &Mesh, triangles: &[usize], vector: [f64; 3]) -> Result<BuiltMesh> {
    let mesh = crate::solid::proximity::valid_source(mesh, 10_000)?;
    let source = mesh.inspect()?;
    if source.non_manifold_edges > 0 || source.orientation_conflicts > 0 {
        return Err(error("Face extrusion requires manifold oriented input"));
    }
    if triangles.is_empty()
        || triangles.iter().any(|&i| i >= mesh.indices.len() / 3)
        || vector.iter().any(|v| !v.is_finite())
        || vector.iter().map(|v| v * v).sum::<f64>() <= 1e-24
    {
        return Err(error("Invalid face extrusion selection/vector"));
    }
    let selected: BTreeSet<_> = triangles.iter().copied().collect();
    let mut edges: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
    let mut vertices: BTreeSet<usize> = BTreeSet::new();
    for &i in &selected {
        let t = &mesh.indices[3 * i..3 * i + 3];
        vertices.extend(t.iter().copied());
        for k in 0..3 {
            let a = t[k];
            let b = t[(k + 1) % 3];
            edges.entry((a.min(b), a.max(b))).or_default().push((a, b));
        }
    }
    let boundary: Vec<_> = edges
        .values()
        .filter(|uses| uses.len() == 1)
        .map(|u| u[0])
        .collect();
    if boundary.is_empty() {
        return Err(error("Face selection has no extrusion boundary"));
    }
    if mesh.indices.len() / 3 + boundary.len() * 2 > 20_000 {
        return Err(error("Extrusion triangle budget exceeded"));
    }
    let mut positions = mesh.positions.clone();
    let mut remap = BTreeMap::new();
    for i in vertices {
        remap.insert(i, positions.len() / 3);
        for (k, v) in vector.iter().enumerate() {
            positions.push(mesh.positions[i * 3 + k] + v);
        }
    }
    let mut indices = Vec::new();
    for (i, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        for v in t {
            indices.push(if selected.contains(&i) { remap[v] } else { *v });
        }
    }
    for (a, b) in boundary {
        let c = remap[&b];
        let d = remap[&a];
        indices.extend([a, b, c, a, c, d]);
    }
    let result = finish(crate::solid::proximity::weld_exact(&Mesh {
        positions,
        indices,
        uv: None,
    })?)?;
    if source.closed && !result.report.closed {
        return Err(error("Face extrusion did not preserve closed topology"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::modeling::{Profile, extrude};
    fn cube() -> BuiltMesh {
        let p = Profile {
            outer: vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
            holes: vec![],
        };
        extrude(&p, [0., 0., 2.]).unwrap()
    }
    fn max_z(mesh: &Mesh) -> f64 {
        mesh.positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p[2])
            .fold(f64::MIN, f64::max)
    }
    #[test]
    fn brush_moves_only_vertices_inside_radius_and_keeps_topology() {
        let cube = cube();
        let b = geometry_ops::Brush {
            center: [1., 1., 2.],
            radius: 0.5,
            displacement: [0., 0., 1.],
        };
        let out = brush(&cube.mesh, &b).unwrap();
        assert_eq!(
            out.mesh.indices, cube.mesh.indices,
            "brush must not change topology"
        );
        assert_eq!(out.mesh.positions.len(), cube.mesh.positions.len());
        assert!(out.report.closed);
        assert_eq!(out.report.degenerate_triangles, 0);
        assert!(
            (max_z(&out.mesh) - 3.).abs() < 1e-12,
            "corner under brush center moves by full displacement"
        );
        let moved = cube
            .mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .zip(out.mesh.positions.as_chunks::<3>().0)
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            moved >= 1 && moved < cube.mesh.positions.len() / 3,
            "only the local corner is affected"
        );
        assert!(
            out.report.signed_volume_mm3 > cube.report.signed_volume_mm3,
            "additive brush grows the solid"
        );
    }
    #[test]
    fn brush_with_negative_displacement_carves_and_wide_brush_translates() {
        let cube = cube();
        let miss = geometry_ops::Brush {
            center: [0.5, 0.5, 2.],
            radius: 0.1,
            displacement: [0., 0., -0.5],
        };
        let out = brush(&cube.mesh, &miss).unwrap();
        assert!(out.report.closed);
        assert_eq!(
            out.mesh.positions, cube.mesh.positions,
            "no vertex inside a tiny radius: unchanged"
        );
        let carve = geometry_ops::Brush {
            center: [1., 1., 2.],
            radius: 0.5,
            displacement: [0., 0., -0.5],
        };
        let carved = brush(&cube.mesh, &carve).unwrap();
        assert!(carved.report.closed);
        assert!(
            carved.report.signed_volume_mm3 < cube.report.signed_volume_mm3,
            "subtractive brush shrinks the solid"
        );
        let wide = geometry_ops::Brush {
            center: [0., 0., 1.],
            radius: 1e3,
            displacement: [3., 0., 0.],
        };
        let shifted = brush(&cube.mesh, &wide).unwrap();
        assert!(
            (shifted.report.signed_volume_mm3 - cube.report.signed_volume_mm3).abs() < 1e-3,
            "near-uniform weight ≈ rigid translation"
        );
        for (a, b) in cube
            .mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .zip(shifted.mesh.positions.as_chunks::<3>().0)
        {
            assert!((b[0] - a[0] - 3.).abs() < 1e-4 && b[1] == a[1] && b[2] == a[2]);
        }
        assert!(shifted.report.closed);
    }
    #[test]
    fn brush_rejects_invalid_brush_and_degenerate_results() {
        let cube = cube();
        for radius in [0., -1., f64::NAN] {
            let b = geometry_ops::Brush {
                center: [0.; 3],
                radius,
                displacement: [0., 0., 1.],
            };
            assert!(brush(&cube.mesh, &b).is_err(), "{radius}");
        }
        // Collapsing the whole top face onto the bottom creates degenerate triangles.
        let flatten = geometry_ops::Brush {
            center: [0., 0., 2.],
            radius: 1e-3,
            displacement: [0., 0., -2.],
        };
        let mut flat = cube.mesh.clone();
        for p in flat.positions.as_chunks_mut::<3>().0 {
            if p[2] == 2. {
                *p = [0., 0., 2.];
            }
        }
        assert!(brush(&flat, &flatten).is_err());
    }
}
