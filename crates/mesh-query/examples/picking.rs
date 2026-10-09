use mesh_query::{MeshView, Query, build_mesh_bvh, closest_point, raycast};
use std::collections::BTreeSet;

fn main() {
    let vertices = [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.];
    let indices = [0_u32, 1, 2];
    let tree = build_mesh_bvh(&vertices, &indices, 3, 8);
    let excluded = BTreeSet::new();
    let query = Query {
        origin: [0.25, 0.25, 2.],
        direction: [0., 0., -1.],
        min_t: 0.,
        max_t: f64::INFINITY,
        local_from_world: None,
        excluded: &excluded,
    };
    let hit = raycast(&tree, &vertices, &indices, 3, &query)
        .unwrap()
        .unwrap();
    assert_eq!(hit.triangle, 0);
    assert_eq!(hit.world_point, [0.25, 0.25, 0.]);
    let positions = vertices.map(f64::from);
    let triangles = indices.map(|i| i as usize);
    let mesh = MeshView::new(&positions, &triangles);
    mesh.validate().unwrap();
    let (point, distance) = closest_point(&mesh, query.origin);
    assert_eq!(point, hit.world_point);
    println!(
        "triangle={}, point={point:?}, distance={distance}",
        hit.triangle
    );
}
