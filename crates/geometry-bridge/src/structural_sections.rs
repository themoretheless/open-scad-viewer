//! Geometric section properties of the supplied final mesh, never its bounding-box graph.
use crate::{Result, Value, field, input, json, require_exact_fields};
use polygon_core::{Mesh, solid::section::MeshSectionIndex};

pub fn inspect(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "mesh", "axis", "stations"],
        "structural sections",
    )?;
    let raw = &v["mesh"];
    if raw["positions"]
        .as_array()
        .is_none_or(|a| a.len() > 900_000)
        || raw["indices"].as_array().is_none_or(|a| a.len() > 300_000)
    {
        return Err(input(
            "Structural sections require at most 100000 triangles / 300000 vertices",
        ));
    }
    let mut mesh: Mesh = field(&v, "mesh")?;
    let axis: String = field(&v, "axis")?;
    let permutation = match axis.as_str() {
        "x" => [1, 2, 0],
        "y" => [2, 0, 1],
        "z" => [0, 1, 2],
        _ => return Err(input("Section axis must be x, y or z")),
    };
    let stations = v["stations"]
        .as_array()
        .ok_or_else(|| input("stations must be an array"))?;
    if stations.is_empty() || stations.len() > 64 {
        return Err(input("Provide 1-64 section positions"));
    }
    let stations: Vec<f64> = field(&v, "stations")?;
    if stations.iter().any(|v| !v.is_finite()) || stations.windows(2).any(|w| w[0] >= w[1]) {
        return Err(input(
            "Section positions must be finite and strictly increasing",
        ));
    }
    let topology = mesh.inspect()?;
    if !topology.closed || topology.degenerate_triangles > 0 || topology.signed_volume_mm3 <= 0. {
        return Err(input(
            "Structural sections require a closed, positively oriented, nondegenerate mesh",
        ));
    }
    let mut connectivity = boundary_connectivity(&mesh)?;
    let material_audit = match polygon_core::solid::boolean::audit_material(&mesh) {
        Ok(audit) => {
            connectivity["materialConnectivity"] = json!("classified-at-tolerance");
            let shells: Vec<_> = audit.shells.iter().enumerate().map(|(id, shell)|
                json!({"id":id,"firstTriangle":shell.first_triangle,"parent":shell.parent,
                    "depth":shell.depth,"kind":if shell.depth % 2 == 0 {"material"} else {"cavity"}})).collect();
            json!({"status":"classified","toleranceMm":audit.tolerance_mm,
                "materialRegions":audit.material_regions,"shells":shells})
        }
        Err(error) => json!({"status":"unresolved","code":error.code,"message":error.message}),
    };
    let intersection_status = if material_audit["status"] == "classified" {
        "checked-at-tolerance"
    } else {
        "not-checked"
    };
    for p in mesh.positions.chunks_exact_mut(3) {
        let original = [p[0], p[1], p[2]];
        for k in 0..3 {
            p[k] = original[permutation[k]];
        }
    }
    let index = MeshSectionIndex::new(&mesh)?;
    let mut sections = Vec::with_capacity(stations.len());
    for station in stations {
        let section = index.section(station)?;
        let contours: Vec<_> = section.contours.iter().map(|c| c.points.clone()).collect();
        let sources: Vec<_> = section
            .contours
            .iter()
            .map(|c| c.source_triangles.clone())
            .collect();
        if contours.is_empty() {
            sections.push(json!({"positionMm":station,"material":false,"contours":[],"boundaryContours":[],"sourceTriangles":sources,"properties":Value::Null}));
            continue;
        }
        // Bounded planar validation refuses crossings; normalization only resolves winding/holes.
        planar_geometry::rings::validate_contours(&contours)?;
        let boundary_contours = contours;
        let contours = planar_geometry::rings::nonzero(&boundary_contours)?;
        if contours.is_empty() {
            return Err(input(
                "Section has no material after winding classification",
            ));
        }
        let report = mechanics_core::analyze_section(
            &mechanics_core::LayerSection {
                z_mm: station,
                contours: contours.clone(),
            },
            &mechanics_core::LoadCase {
                moment_x_nmm: 0.,
                moment_y_nmm: 0.,
                axial_n: 0.,
                allowable_mpa: 1.,
            },
        )?;
        sections.push(json!({"positionMm":station,"material":true,"contours":contours,"boundaryContours":boundary_contours,
            "sourceTriangles":sources,"properties":{"areaMm2":report.area_mm2,"centroidMm":report.centroid,
            "iuuMm4":report.ixx_mm4,"ivvMm4":report.iyy_mm4,"iuvMm4":report.ixy_mm4}}));
    }
    Ok(
        json!({"modelKind":"finished-mesh-sections-v1","sourceMesh":raw,"axis":axis,"planeAxes":permutation[..2],
        "volumeMm3":topology.signed_volume_mm3,"triangleCount":topology.triangle_count,
        "selfIntersections":intersection_status,"connectivity":connectivity,"materialAudit":material_audit,"sections":sections}),
    )
}

/// Indexed, edge-connected boundary shells. This deliberately does not infer solid
/// connectivity: separate shells can enclose cavities or overlap geometrically.
fn boundary_connectivity(mesh: &Mesh) -> Result<Value> {
    use std::collections::{BTreeMap, BTreeSet};
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let triangles = mesh.indices.as_chunks::<3>().0;
    let mut parent: Vec<_> = (0..triangles.len()).collect();
    let mut edges = BTreeMap::new();
    for (i, t) in triangles.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            if let Some(j) = edges.insert((a.min(b), a.max(b)), i) {
                let r = root(&mut parent, i);
                let s = root(&mut parent, j);
                parent[r] = s;
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for i in 0..triangles.len() {
        groups.entry(root(&mut parent, i)).or_default().push(i);
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by_key(|g| g[0]);
    let mut memberships = vec![BTreeSet::new(); mesh.positions.len() / 3];
    let mut components = Vec::new();
    for (id, ids) in groups.into_iter().enumerate() {
        let origin = mesh.point(triangles[ids[0]][0])?;
        let mut lo = origin;
        let mut hi = origin;
        let mut sum = 0.;
        let mut correction = 0.;
        for &i in &ids {
            let mut p = [[0.; 3]; 3];
            for k in 0..3 {
                let vertex = triangles[i][k];
                memberships[vertex].insert(id);
                let point = mesh.point(vertex)?;
                for d in 0..3 {
                    lo[d] = lo[d].min(point[d]);
                    hi[d] = hi[d].max(point[d]);
                    p[k][d] = point[d] - origin[d];
                }
            }
            let [a, b, c] = p;
            let term = (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
                - correction;
            let next = sum + term;
            correction = (next - sum) - term;
            sum = next;
        }
        if !sum.is_finite() {
            return Err(input("Boundary component volume exceeded finite bounds"));
        }
        components.push(json!({"id":id,"sourceTriangles":ids,
            "signedVolumeMm3":sum,"boundsMm":[lo,hi]}));
    }
    let shared_vertices: Vec<_> = memberships.iter().enumerate()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(vertex, ids)| json!({"sourceVertex":vertex,"components":ids.iter().copied().collect::<Vec<_>>()}))
        .collect();
    Ok(json!({"modelKind":"indexed-edge-boundary-components-v1",
        "materialConnectivity":"not-established", "components":components,
        "sharedVertices":shared_vertices}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use polygon_core::solid::modeling::{Profile, extrude};
    fn block(hollow: bool) -> Mesh {
        extrude(
            &Profile {
                outer: vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
                holes: if hollow {
                    vec![vec![[2., 2.], [8., 2.], [8., 8.], [2., 8.]]]
                } else {
                    vec![]
                },
            },
            [0., 0., 10.],
        )
        .unwrap()
        .mesh
    }
    #[test]
    fn same_bounds_do_not_hide_material_removed_by_a_hole() {
        let solid = inspect(
            json!({"op":"structural_sections","mesh":block(false),"axis":"z","stations":[5.]}),
        )
        .unwrap();
        let hollow = inspect(
            json!({"op":"structural_sections","mesh":block(true),"axis":"z","stations":[5.]}),
        )
        .unwrap();
        let a = &solid["sections"][0]["properties"];
        let b = &hollow["sections"][0]["properties"];
        assert!(hollow["sourceMesh"]["indices"].as_array().is_some());
        assert!((a["areaMm2"].as_f64().unwrap() - 100.).abs() < 1e-9);
        assert!((b["areaMm2"].as_f64().unwrap() - 64.).abs() < 1e-9);
        assert!((b["iuuMm4"].as_f64().unwrap() - (10000. - 1296.) / 12.).abs() < 1e-8);
        assert_eq!(b["centroidMm"], json!([5., 5.]));
        assert_eq!(
            hollow["sections"][0]["contours"].as_array().unwrap().len(),
            2
        );
    }
    #[test]
    fn transverse_sections_preserve_disconnected_material_and_empty_stations() {
        let v=inspect(json!({"op":"structural_sections","mesh":block(true),"axis":"x","stations":[-1.,5.,10.]})).unwrap();
        assert_eq!(v["planeAxes"], json!([1, 2]));
        assert_eq!(v["sections"][0]["material"], json!(false));
        assert_eq!(v["sections"][2]["properties"], Value::Null);
        assert!((v["sections"][1]["properties"]["areaMm2"].as_f64().unwrap() - 40.).abs() < 1e-9);
    }
    #[test]
    fn refuses_open_mesh_and_invalid_station_requests() {
        let mut mesh = block(false);
        mesh.indices.truncate(mesh.indices.len() - 3);
        assert!(
            inspect(json!({"op":"structural_sections","mesh":mesh,"axis":"z","stations":[5.]}))
                .is_err()
        );
        for stations in [vec![], vec![5., 5.], vec![6., 5.], vec![1.; 65]] {
            assert!(inspect(json!({"op":"structural_sections","mesh":block(false),"axis":"z","stations":stations})).is_err());
        }
    }
    fn append(a: &mut Mesh, b: Mesh, offset: [f64; 3], weld: bool) {
        let mut mapping = Vec::new();
        for p in b.positions.chunks_exact(3) {
            let p: Vec<_> = (0..3).map(|k| p[k] + offset[k]).collect();
            let existing = weld
                .then(|| a.positions.chunks_exact(3).position(|q| q == p))
                .flatten();
            mapping.push(existing.unwrap_or_else(|| {
                let index = a.positions.len() / 3;
                a.positions.extend(p);
                index
            }));
        }
        a.indices.extend(b.indices.iter().map(|&i| mapping[i]));
    }
    #[test]
    fn edge_components_do_not_merge_at_a_shared_point() {
        let mut mesh = block(false);
        append(&mut mesh, block(false), [10., 10., 10.], true);
        assert!(mesh.inspect().unwrap().closed);
        let v = boundary_connectivity(&mesh).unwrap();
        assert_eq!(v["components"].as_array().unwrap().len(), 2);
        assert_eq!(v["sharedVertices"].as_array().unwrap().len(), 1);
        assert_eq!(v["sharedVertices"][0]["components"], json!([0, 1]));
        let count: usize = v["components"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["sourceTriangles"].as_array().unwrap().len())
            .sum();
        assert_eq!(count, mesh.indices.len() / 3);
    }
    #[test]
    fn cavities_are_not_reported_as_disconnected_material() {
        let mut mesh = block(false);
        let mut cavity = block(false);
        for p in &mut cavity.positions {
            *p *= 0.6;
        }
        for t in cavity.indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
        append(&mut mesh, cavity, [2., 2., 2.], false);
        let v = inspect(json!({"op":"structural_sections","mesh":mesh,"axis":"z","stations":[5.]}))
            .unwrap();
        let audit = &v["connectivity"];
        assert_eq!(audit["materialConnectivity"], "classified-at-tolerance");
        assert_eq!(v["materialAudit"]["materialRegions"], 1);
        assert_eq!(v["materialAudit"]["shells"][1]["kind"], "cavity");
        assert_eq!(audit["components"].as_array().unwrap().len(), 2);
        assert!((audit["components"][1]["signedVolumeMm3"].as_f64().unwrap() + 216.).abs() < 1e-9);
        assert_eq!(audit["sharedVertices"], json!([]));
    }
    #[test]
    fn through_hole_is_one_boundary_and_separate_blocks_are_two() {
        let v = boundary_connectivity(&block(true)).unwrap();
        assert_eq!(v["components"].as_array().unwrap().len(), 1);
        let mut mesh = block(false);
        append(&mut mesh, block(false), [20., 0., 0.], false);
        let v = boundary_connectivity(&mesh).unwrap();
        assert_eq!(v["components"].as_array().unwrap().len(), 2);
        assert_eq!(v["sharedVertices"], json!([]));
        assert_eq!(
            v["components"][1]["boundsMm"],
            json!([[20., 0., 0.], [30., 10., 10.]])
        );
    }
    #[test]
    fn unresolved_contact_preserves_geometric_report_without_connectivity_claim() {
        let mut mesh = block(false);
        append(&mut mesh, block(false), [10.; 3], true);
        let v =
            inspect(json!({"op":"structural_sections","mesh":mesh,"axis":"z","stations":[-1.]}))
                .unwrap();
        assert_eq!(v["materialAudit"]["status"], "unresolved");
        assert_eq!(v["connectivity"]["materialConnectivity"], "not-established");
        assert_eq!(v["selfIntersections"], "not-checked");
        assert_eq!(v["materialAudit"]["code"], "POLYGON_BOOLEAN_INVALID_SOLID");
    }
}
