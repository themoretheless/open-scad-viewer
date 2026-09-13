//! Immutable native picking snapshots. Upload once, query repeatedly, dispose.
//! Handles are monotonic within one runtime and never identify a reused slot.
use super::{Result, Value, field, input};
use polygon_core::solid::{bvh, bvh_query};
use std::collections::{BTreeMap, BTreeSet};
use value_codec::json;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_MESHES: usize = 64;
struct Mesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    tree: bvh::MeshBvh,
    stride: usize,
    charge: usize,
}
#[derive(Default)]
pub struct Registry {
    meshes: BTreeMap<u64, Mesh>,
    next: u64,
    bytes: usize,
}
impl Registry {
    pub fn insert(
        &mut self,
        vertices: Vec<f32>,
        indices: Vec<u32>,
        stride: usize,
        leaf: usize,
    ) -> Result<u64> {
        if !(3..=256).contains(&stride) || !(1..=64).contains(&leaf) {
            return Err(input("Invalid picking mesh stride or leaf size"));
        }
        // The builder reserves a power-of-two leaf capacity, which can exceed
        // the number of live nodes. Charge capacities, including caller buffers.
        let triangles = indices.len() / 3;
        let nodes = if triangles == 0 {
            Some(0)
        } else {
            triangles
                .div_ceil(leaf)
                .checked_next_power_of_two()
                .and_then(|n| n.checked_mul(2))
                .and_then(|n| n.checked_sub(1))
        };
        let charge = vertices
            .capacity()
            .checked_add(indices.capacity())
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| {
                nodes
                    .and_then(|b| b.checked_mul(32))
                    .and_then(|b| n.checked_add(b))
            })
            .and_then(|n| triangles.checked_mul(4).and_then(|b| n.checked_add(b)))
            .ok_or_else(|| input("Picking mesh size overflow"))?;
        if self.meshes.len() >= MAX_MESHES || charge > MAX_BYTES.saturating_sub(self.bytes) {
            return Err(input("Native picking snapshot budget exceeded"));
        }
        let handle = self
            .next
            .checked_add(1)
            .ok_or_else(|| input("Picking handle space exhausted"))?;
        let tree = bvh::build_mesh_bvh(&vertices, &indices, stride, leaf);
        self.meshes.insert(
            handle,
            Mesh {
                vertices,
                indices,
                tree,
                stride,
                charge,
            },
        );
        self.bytes += charge;
        self.next = handle;
        Ok(handle)
    }
    pub fn dispose(&mut self, handle: u64) -> Result<()> {
        let mesh = self
            .meshes
            .remove(&handle)
            .ok_or_else(|| input("Unknown or disposed picking snapshot"))?;
        self.bytes -= mesh.charge;
        Ok(())
    }
    pub fn query(
        &self,
        handle: u64,
        query: &bvh_query::Query<'_>,
    ) -> Result<Option<bvh_query::Hit>> {
        let mesh = self
            .meshes
            .get(&handle)
            .ok_or_else(|| input("Unknown or disposed picking snapshot"))?;
        bvh_query::raycast(
            &mesh.tree,
            &mesh.vertices,
            &mesh.indices,
            mesh.stride,
            query,
        )
        .map_err(|_| input("Invalid native picking tree"))
    }
}

thread_local! { static REGISTRY: std::cell::RefCell<Registry> = std::cell::RefCell::new(Registry::default()); }
pub fn create(vertices: Vec<f32>, indices: Vec<u32>, stride: usize, leaf: usize) -> Result<String> {
    REGISTRY.with(|registry| {
        registry
            .borrow_mut()
            .insert(vertices, indices, stride, leaf)
            .map(|id| id.to_string())
    })
}
fn handle(v: &Value) -> Result<u64> {
    let text: String = field(v, "handle")?;
    let id = text
        .parse::<u64>()
        .map_err(|_| input("Invalid picking handle"))?;
    if id == 0 || id.to_string() != text {
        return Err(input("Noncanonical picking handle"));
    }
    Ok(id)
}
fn bound(v: &Value, name: &str, fallback: f64) -> Result<f64> {
    if v.get(name).is_none_or(Value::is_null) {
        Ok(fallback)
    } else {
        field(v, name)
    }
}
pub fn dispatch(v: Value) -> Result<Value> {
    match v["action"].as_str() {
        Some("create") => REGISTRY.with(|registry| {
            let id = registry.borrow_mut().insert(
                field(&v, "vertices")?,
                field(&v, "indices")?,
                field(&v, "stride")?,
                field(&v, "leafSize")?,
            )?;
            Ok(json!({"handle":id.to_string()}))
        }),
        Some("dispose") => REGISTRY.with(|registry| {
            registry.borrow_mut().dispose(handle(&v)?)?;
            Ok(Value::Null)
        }),
        Some("query") => {
            let excluded: Vec<u32> = field(&v, "excludedTriangles")?;
            if excluded.len() > 65536 {
                return Err(input("Picking exclusion budget exceeded"));
            }
            let excluded: BTreeSet<_> = excluded.into_iter().collect();
            let query = bvh_query::Query {
                origin: field(&v, "origin")?,
                direction: field(&v, "direction")?,
                min_t: bound(&v, "minT", 0.)?,
                max_t: bound(&v, "maxT", f64::INFINITY)?,
                local_from_world: if v.get("localFromWorld").is_none_or(Value::is_null) {
                    None
                } else {
                    Some(field(&v, "localFromWorld")?)
                },
                excluded: &excluded,
            };
            REGISTRY.with(|registry| {
                let hit = registry.borrow().query(handle(&v)?,&query)?;
                Ok(match hit {
                    None => Value::Null,
                    Some(h) => json!({"triangleIndex":h.triangle,"triangleVertexIndices":h.vertex_indices,"t":h.t,
                        "barycentric":h.barycentric,"localPoint":h.local_point,"worldPoint":h.world_point,
                        "localNormal":h.local_normal,"worldNormal":h.world_normal,"frontFace":h.front_face}),
                })
            })
        }
        _ => Err(input("Unknown picking snapshot action")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_validates_bounds_and_preserves_snapshot_after_refusal() {
        let created = dispatch(json!({"action":"create","vertices":[-1,-1,0,1,-1,0,0,1,0],"indices":[0,1,2],"stride":3,"leafSize":1})).unwrap();
        let id = created["handle"].clone();
        let mut query = json!({"action":"query","handle":id,"origin":[0,0,2],"direction":[0,0,-1],"excludedTriangles":[],"maxT":"invalid"});
        assert!(dispatch(query.clone()).is_err());
        query
            .as_object_mut()
            .unwrap()
            .insert("maxT".into(), Value::Null);
        assert_eq!(dispatch(query.clone()).unwrap()["t"].as_f64(), Some(2.));
        dispatch(json!({"action":"dispose","handle":id})).unwrap();
        assert!(dispatch(query).is_err());
    }
    #[test]
    fn snapshot_queries_reuse_owned_buffers_and_reject_disposed_handles() {
        let mut registry = Registry::default();
        let id = registry
            .insert(
                vec![-1., -1., 0., 1., -1., 0., 0., 1., 0.],
                vec![0, 1, 2],
                3,
                1,
            )
            .unwrap();
        let pointer = registry.meshes[&id].vertices.as_ptr();
        let excluded = BTreeSet::new();
        let q = bvh_query::Query {
            origin: [0., 0., 2.],
            direction: [0., 0., -1.],
            min_t: 0.,
            max_t: 10.,
            local_from_world: None,
            excluded: &excluded,
        };
        for _ in 0..100 {
            assert_eq!(registry.query(id, &q).unwrap().unwrap().t, 2.);
        }
        assert_eq!(registry.meshes[&id].vertices.as_ptr(), pointer);
        registry.dispose(id).unwrap();
        assert_eq!(registry.bytes, 0);
        assert!(registry.query(id, &q).is_err());
        assert!(registry.dispose(id).is_err());
        let next = registry.insert(vec![], vec![], 3, 1).unwrap();
        assert!(next > id);
    }
    #[test]
    fn bounded_registry_and_failed_admission_do_not_publish() {
        let mut registry = Registry::default();
        assert!(
            registry
                .insert(vec![], vec![0; (MAX_BYTES / 68 + 1) * 3], 3, 1)
                .is_err()
        );
        assert_eq!(registry.bytes, 0);
        assert!(registry.insert(vec![], vec![], 0, 1).is_err());
        assert_eq!(registry.next, 0);
        for _ in 0..MAX_MESHES {
            registry.insert(vec![], vec![], 3, 1).unwrap();
        }
        assert!(registry.insert(vec![], vec![], 3, 1).is_err());
        registry.dispose(1).unwrap();
        assert!(registry.insert(vec![], vec![], 3, 1).unwrap() > 64);
    }
}
