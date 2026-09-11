//! Mesh-native edits; topology checks do not certify absence of self-intersections.
use crate::{BuiltMesh, Error, Mesh, Result};
use std::collections::{BTreeMap, BTreeSet};
fn finish(mesh: Mesh) -> Result<BuiltMesh> {
    let report = mesh.inspect()?;
    if report.degenerate_triangles > 0
        || report.non_manifold_edges > 0
        || report.orientation_conflicts > 0
    {
        return Err(Error::new("Edit creates invalid triangle topology"));
    }
    Ok(BuiltMesh { mesh, report })
}
pub fn deform(mesh: &Mesh, operation: &geometry_ops::Deformation) -> Result<BuiltMesh> {
    mesh.validate()?;
    operation.validate().map_err(Error::new)?;
    let mut result = mesh.clone();
    result.positions = mesh
        .positions
        .chunks_exact(3)
        .map(|p| operation.apply([p[0], p[1], p[2]]).map_err(Error::new))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    finish(result)
}
pub fn brush(mesh: &Mesh, brush: &geometry_ops::Brush) -> Result<BuiltMesh> {
    mesh.validate()?;
    brush.validate().map_err(Error::new)?;
    let mut result = mesh.clone();
    result.positions = mesh
        .positions
        .chunks_exact(3)
        .map(|p| brush.apply([p[0], p[1], p[2]]).map_err(Error::new))
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
        return Err(Error::new(
            "Face extrusion requires manifold oriented input",
        ));
    }
    if triangles.is_empty()
        || triangles.iter().any(|&i| i >= mesh.indices.len() / 3)
        || vector.iter().any(|v| !v.is_finite())
        || vector.iter().map(|v| v * v).sum::<f64>() <= 1e-24
    {
        return Err(Error::new("Invalid face extrusion selection/vector"));
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
        return Err(Error::new("Face selection has no extrusion boundary"));
    }
    if mesh.indices.len() / 3 + boundary.len() * 2 > 20_000 {
        return Err(Error::new("Extrusion triangle budget exceeded"));
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
    for (i, t) in mesh.indices.chunks_exact(3).enumerate() {
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
        return Err(Error::new(
            "Face extrusion did not preserve closed topology",
        ));
    }
    Ok(result)
}
