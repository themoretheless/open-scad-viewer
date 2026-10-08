//! Transport and composition of mesh and BRep feature targets.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
pub fn geometry(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let topology = mesh_topology::planar::topology(mesh.view()).map_err(|e| input(e.message))?;
    let brep = v
        .get("brep")
        .filter(|v| !v.is_null())
        .map(|_| field::<brep_core::Model>(&v, "brep"))
        .transpose()?;
    let geometry = if let Some(model) = brep {
        let mut out = brep_core::keypoints::geometry(&model)?;
        let accepted = topology
            .faces
            .iter()
            .map(|f| brep_core::keypoints::matches_planar_face(&model, f.center, f.normal))
            .collect::<Vec<_>>();
        mesh_topology::keypoints::append_centers(&mut out, mesh.view(), &topology, &accepted)
            .map_err(|e| input(e.message))?;
        out.bounds_center(&mesh_topology::keypoints::points(mesh.view()));
        out
    } else {
        mesh_topology::keypoints::geometry(mesh.view(), &topology).map_err(|e| input(e.message))?
    };
    let points = geometry
        .points
        .iter()
        .map(|p| value_codec::json!({"point":p.point,"kind":p.kind.name()}))
        .collect::<Vec<_>>();
    let segments = geometry
        .segments
        .iter()
        .map(|s| {
            let mut value = value_codec::json!({"a":s.a,"b":s.b});
            if let Some(i) = &s.interval {
                value["interval"] =
                    value_codec::json!({"curve":i.curve,"start":i.start,"end":i.end});
            }
            value
        })
        .collect::<Vec<_>>();
    encode(value_codec::json!({"points":points,"segments":segments}))
}
