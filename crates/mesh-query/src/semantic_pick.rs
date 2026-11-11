//! Ranked semantic ray picking layered on the existing triangle BVH.
use crate::{Error, MeshBvh, Query, Result, raycast};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriangleSemantics {
    pub body: u64,
    pub face: u64,
    pub edge: Option<u64>,
    pub vertex: Option<u64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticHit {
    pub body: u64,
    pub face: u64,
    pub edge: Option<u64>,
    pub vertex: Option<u64>,
    pub depth: f64,
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub triangle: u32,
}
/// A pick ray: hits are reported for depths in `[0, max_depth]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickRay {
    pub origin: [f64; 3],
    pub direction: [f64; 3],
    pub max_depth: f64,
}
/// Return all hits up to `max_hits`, ordered by ray depth then stable IDs.
/// UV are the first two barycentric coordinates, not an invented surface UV.
pub fn semantic_ray_pick(
    bvh: &MeshBvh,
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
    ray: PickRay,
    semantics: &[TriangleSemantics],
    max_hits: usize,
) -> Result<Vec<SemanticHit>> {
    let PickRay {
        origin,
        direction,
        max_depth,
    } = ray;
    if origin.iter().chain(&direction).any(|v| !v.is_finite())
        || direction == [0.; 3]
        || !max_depth.is_finite()
        || max_depth < 0.
        || max_hits == 0
    {
        return Err(Error::new(
            "MESH_QUERY_INVALID_INPUT",
            "Invalid semantic pick ray or hit budget",
        ));
    }
    if semantics.len() != indices.len() / 3 {
        return Err(Error::new(
            "MESH_QUERY_INVALID_INPUT",
            "One semantic record per triangle is required",
        ));
    }
    let mut excluded = BTreeSet::new();
    let mut hits = vec![];
    loop {
        let query = Query {
            origin,
            direction,
            min_t: 0.,
            max_t: max_depth,
            local_from_world: None,
            excluded: &excluded,
        };
        let Some(hit) = raycast(bvh, vertices, indices, stride, &query)
            .map_err(|_| Error::new("MESH_QUERY_INVALID_INPUT", "Invalid picking BVH"))?
        else {
            break;
        };
        excluded.insert(hit.triangle);
        let s = semantics[hit.triangle as usize];
        hits.push(SemanticHit {
            body: s.body,
            face: s.face,
            edge: s.edge,
            vertex: s.vertex,
            depth: hit.t,
            uv: [hit.barycentric[0], hit.barycentric[1]],
            point: hit.local_point,
            triangle: hit.triangle,
        });
        if hits.len() == max_hits {
            break;
        }
    }
    hits.sort_by(|a, b| {
        a.depth
            .total_cmp(&b.depth)
            .then(a.body.cmp(&b.body))
            .then(a.face.cmp(&b.face))
            .then(a.triangle.cmp(&b.triangle))
    });
    Ok(hits)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_mesh_bvh;
    #[test]
    fn returns_ranked_body_face_depth_and_barycentric_uv() {
        let p = vec![
            0., 0., 2., 1., 0., 2., 0., 1., 2., 0., 0., 1., 1., 0., 1., 0., 1., 1.,
        ];
        let i = vec![0, 1, 2, 3, 4, 5];
        let b = build_mesh_bvh(&p, &i, 3, 1);
        let semantics = [
            TriangleSemantics {
                body: 2,
                face: 20,
                edge: None,
                vertex: None,
            },
            TriangleSemantics {
                body: 1,
                face: 10,
                edge: None,
                vertex: None,
            },
        ];
        let h = semantic_ray_pick(
            &b,
            &p,
            &i,
            3,
            PickRay {
                origin: [0.1, 0.1, 0.],
                direction: [0., 0., 1.],
                max_depth: 10.,
            },
            &semantics,
            8,
        )
        .unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].body, 1);
        assert_eq!(h[0].depth, 1.);
        assert_eq!(h[1].face, 20);
        assert!(h.iter().all(|x| x.uv[0] > 0.));
    }
}
