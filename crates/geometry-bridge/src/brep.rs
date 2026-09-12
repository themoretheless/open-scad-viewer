//! Tessellation and interchange preserve topological face identity.
use super::*;
use std::collections::BTreeMap;

pub struct Tessellation {
    pub built: BuiltMesh,
    pub face_ids: Vec<usize>,
    pub topology_face_ids: Option<Vec<String>>,
}
impl value_codec::Serialize for Tessellation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if let value_codec::Value::Object(fields) = value_codec::Serialize::to_value(&self.built) {
            object.extend(fields);
        }
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        if let Some(ids) = &self.topology_face_ids {
            object.insert(
                "topologyFaceIds".into(),
                value_codec::Serialize::to_value(ids),
            );
        }
        value_codec::Value::Object(object)
    }
}
pub(crate) fn weld(mesh: Mesh, tolerance: f64) -> Result<Mesh> {
    let mut positions = Vec::<f64>::new();
    let mut cells = BTreeMap::<[i64; 3], Vec<usize>>::new();
    let mut remap = vec![];
    for p in mesh.positions.as_chunks::<3>().0 {
        let cell = [
            (p[0] / tolerance).floor() as i64,
            (p[1] / tolerance).floor() as i64,
            (p[2] / tolerance).floor() as i64,
        ];
        let mut found = None;
        'search: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(ids) = cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &id in ids {
                            if (0..3)
                                .map(|a| (positions[3 * id + a] - p[a]).powi(2))
                                .sum::<f64>()
                                .sqrt()
                                <= tolerance
                            {
                                found = Some(id);
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
        let id = found.unwrap_or_else(|| {
            let id = positions.len() / 3;
            positions.extend(p);
            cells.entry(cell).or_default().push(id);
            id
        });
        remap.push(id);
    }
    let out = Mesh {
        positions,
        indices: mesh.indices.iter().map(|i| remap[*i]).collect(),
        uv: None,
    };
    out.validate()?;
    Ok(out)
}
fn finish(
    mesh: Mesh,
    face_ids: Vec<usize>,
    topology_face_ids: Option<Vec<String>>,
    tolerance: f64,
    closed: bool,
) -> Result<Tessellation> {
    let mesh = weld(mesh, tolerance)?;
    let report = mesh.inspect()?;
    if report.degenerate_triangles > 0
        || report.non_manifold_edges > 0
        || report.orientation_conflicts > 0
        || closed && !report.closed
    {
        return Err(input(
            "B-rep tessellation does not preserve manifold seams at this resolution/tolerance",
        ));
    }
    Ok(Tessellation {
        built: BuiltMesh { mesh, report },
        face_ids,
        topology_face_ids,
    })
}
pub fn nurbs(model: &brep_core::Model, segments: usize) -> Result<Tessellation> {
    model.validate()?;
    if !(1..=32).contains(&segments) {
        return Err(input("B-rep tessellation segments must be 1..32"));
    }
    let mut mesh = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    let mut face_ids = vec![];
    for shell in &model.shells {
        for u in &shell.faces {
            let f = &model.faces[u.face];
            // Planar polygon faces produced by brep-core operations should not
            // be resampled through a rectangular UV grid. Triangulating their
            // authored boundary directly preserves every shared topological
            // edge (including short chamfer/fillet facets) before welding.
            if f.surface.degree_u == 1
                && f.surface.degree_v == 1
                && f.holes.is_empty()
                && model.loops[f.outer]
                    .coedges
                    .iter()
                    .all(|c| model.edges[c.edge].curve.degree == 1)
            {
                let coedges = &model.loops[f.outer].coedges;
                let base = mesh.positions.len() / 3;
                for coedge in coedges {
                    let edge = &model.edges[coedge.edge];
                    let vertex = edge.vertices[usize::from(coedge.reversed)];
                    mesh.positions.extend(model.vertices[vertex].point);
                }
                for i in 1..coedges.len() - 1 {
                    let triangle = if u.reversed {
                        [base, base + i + 1, base + i]
                    } else {
                        [base, base + i, base + i + 1]
                    };
                    mesh.indices.extend(triangle);
                    face_ids.push(u.face);
                }
                continue;
            }
            let trim = tessellation::Trim {
                outer: model.loop_uv(f.outer, segments)?,
                holes: f
                    .holes
                    .iter()
                    .map(|l| model.loop_uv(*l, segments))
                    .collect::<nurbs_core::Result<_>>()?,
            };
            let built = tessellate_nurbs(
                &f.surface,
                &Options {
                    segments_u: segments,
                    segments_v: segments,
                    trim: Some(trim),
                    max_triangles: Some(20000),
                },
            )?;
            let base = mesh.positions.len() / 3;
            mesh.positions.extend(&built.mesh.positions);
            for t in built.mesh.indices.as_chunks::<3>().0 {
                let t = if u.reversed {
                    [t[0], t[2], t[1]]
                } else {
                    [t[0], t[1], t[2]]
                };
                mesh.indices.extend(t.map(|i| base + i));
                face_ids.push(u.face);
            }
            if face_ids.len() > 20000 {
                return Err(input("B-rep tessellation exceeds 20000 triangles"));
            }
        }
    }
    let topology_face_ids = face_ids
        .iter()
        .map(|&face| model.1.faces[face].clone())
        .collect();
    finish(
        mesh,
        face_ids,
        Some(topology_face_ids),
        model.tolerance_mm,
        model.shells.iter().all(|s| s.closed),
    )
}
pub fn polygons(model: &polygon_core::solid::brep::Model) -> Result<Tessellation> {
    let t = polygon_core::solid::brep::tessellate(model)?;
    finish(
        t.mesh,
        t.face_ids,
        None,
        model.tolerance_mm,
        model.shells.iter().all(|s| s.closed),
    )
}
