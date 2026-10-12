//! Bridge operations of the `mesh` family.
use crate::*;

const OPS: &[&str] = &[
    "mesh_picking",
    "mesh_export_file",
    "mesh_to_nurbs_brep",
    "mesh_to_sdf",
    "mesh_to_subdivision",
    "mesh_to_nurbs",
    "mesh_spatial_lattice",
    "mesh_shell_adaptive",
    "mesh_shell_sampled",
    "mesh_boolean",
    "mesh_section",
    "mesh_inspect",
    "mesh_build_surfaces",
    "mesh_thicken",
    "mesh_transform",
    "mesh_boundary_loops",
    "mesh_boundary_curves",
    "mesh_export_stl",
    "mesh_export_format",
    "mesh_export_3mf_model",
    "mesh_export_3mf",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "mesh_picking" => mesh_picking::dispatch(v),
        "mesh_export_file" => mesh_export_file::dispatch(v),
        "mesh_to_nurbs_brep" => encode(reconstruction::nurbs_brep_from_mesh(&field(&v, "mesh")?)?),
        "mesh_to_sdf" => encode(sdf_core::Field::from_triangles(
            triangles_from_mesh(&polygon_core::solid::proximity::valid_source(
                &field(&v, "mesh")?,
                4096,
            )?),
            field(&v, "signed")?,
        )?),
        "mesh_to_subdivision" => encode(reconstruction::mesh_to_subdivision(
            &field(&v, "mesh")?,
            field(&v, "iterations")?,
        )?),
        "mesh_to_nurbs" => encode(reconstruction::nurbs_from_mesh(
            &field(&v, "mesh")?,
            field(&v, "mode")?,
            field(&v, "maxDeviationMm")?,
        )?),
        "mesh_spatial_lattice" => encode(mesh_shell::lattice(
            &field(&v, "mesh")?,
            field(&v, "nodes")?,
            field(&v, "edges")?,
            field(&v, "radius")?,
            field(&v, "skin")?,
            field(&v, "step")?,
            field(&v, "organic")?,
            field(&v, "openTop")?,
            v.get("wallDepth").and_then(|x| x.as_f64()).unwrap_or(0.),
            v.get("keepCore").and_then(|x| x.as_bool()).unwrap_or(false),
        )?),
        "mesh_shell_adaptive" => encode(mesh_shell::shell_options(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "openings")?,
            field(&v, "thickness")?,
            field(&v, "step")?,
            true,
        )?),
        "mesh_shell_sampled" => encode(mesh_shell::shell(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "openings")?,
            field(&v, "thickness")?,
            field(&v, "step")?,
        )?),
        "mesh_boolean" => encode(polygon_core::solid::boolean::boolean(
            &field::<Mesh>(&v, "a")?,
            &field::<Mesh>(&v, "b")?,
            field(&v, "operation")?,
            &match v.get("options") {
                Some(options) => {
                    value_codec::from_value(options.clone()).map_err(|e| input(e.to_string()))?
                }
                None => polygon_core::solid::boolean::Options::default(),
            },
        )?),
        // Mesh sections, planned toolpaths and validated G-code previews.
        "mesh_section" => {
            let mesh: Mesh = field(&v, "mesh")?;
            let z_mm: f64 = field(&v, "z")?;
            let section =
                polygon_core::solid::section::MeshSectionIndex::new(&mesh)?.section(z_mm)?;
            Ok(json!({
                "z_mm": section.z_mm,
                "candidateTriangles": section.candidate_triangles,
                "contours": section.contours.iter().map(|contour| json!({
                    "points": contour.points,
                    "sourceTriangles": contour.source_triangles,
                })).collect::<Vec<_>>(),
            }))
        }
        "mesh_inspect" => encode(field::<Mesh>(&v, "mesh")?.inspect()?),
        "mesh_build_surfaces" => print_geometry::dispatch(v),
        "mesh_thicken" => encode(field::<Mesh>(&v, "mesh")?.thicken(field(&v, "vector")?)?),
        "mesh_transform" => {
            let mesh = field::<Mesh>(&v, "mesh")?.transform(field(&v, "matrix")?)?;
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "mesh_boundary_loops" => encode(field::<Mesh>(&v, "mesh")?.boundary_loops()?),
        "mesh_boundary_curves" => encode(boundary_curves(&field(&v, "mesh")?)?),
        "mesh_export_stl" => encode(field::<Mesh>(&v, "mesh")?.export_stl()?),
        "mesh_export_format" => {
            let bytes = polygon_core::mesh_export::export(
                &field(&v, "mesh")?,
                &field::<String>(&v, "format")?,
            )?;
            encode(mesh_analysis::store(
                mesh_analysis::AnalysisBuffers::Bytes { bytes },
            ))
        }
        "mesh_export_3mf_model" => {
            let bytes = polygon_core::model_3mf::export(
                &field(&v, "mesh")?,
                &field::<Vec<Mesh>>(&v, "parts")?,
            )?;
            encode(mesh_analysis::store(
                mesh_analysis::AnalysisBuffers::Bytes { bytes },
            ))
        }
        "mesh_export_3mf" => {
            let bytes = polygon_core::package_3mf::export(
                &field(&v, "mesh")?,
                &field::<Vec<Mesh>>(&v, "parts")?,
                field(&v, "compressed")?,
            )?;
            encode(mesh_analysis::store(
                mesh_analysis::AnalysisBuffers::Bytes { bytes },
            ))
        }
        _ => unreachable!("route admits only OPS"),
    }
}
