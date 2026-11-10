//! Explicit redundant coplanar seams: Boolean output may legally coalesce them,
//! while push-face still has to accept imported/retained partitioned supports.
use super::*;
pub(super) fn cap_fixture(model: &Model) -> Model {
    let mut polygons = Vec::new();
    for u in &model.shells[model.bodies[0].outer_shell].faces {
        let mut polygon = loop_vertices(model, model.faces[u.face].outer)
            .unwrap()
            .into_iter()
            .map(|v| model.vertices[v].point)
            .collect::<Vec<_>>();
        if polygon.iter().all(|p| p[2] == 10.) {
            continue;
        }
        if u.reversed {
            polygon.reverse();
        }
        polygons.push(polygon);
    }
    for [lo, hi] in [[0., 8.], [8., 12.], [12., 20.]] {
        polygons.push(vec![
            [lo, 0., 10.],
            [hi, 0., 10.],
            [hi, 10., 10.],
            [lo, 10., 10.],
        ]);
    }
    let mut result = model_from_polygons(polygons, model.tolerance_mm).unwrap();
    result.inherit_topology_ids(&[model]);
    result.validate().unwrap();
    result
}
