//! Bridge operations of the `general` family.
use crate::*;

const OPS: &[&str] = &[
    "frame_solve",
    "frame_envelope",
    "frame_diagnose",
    "frame_buckling",
    "frame_modal",
    "frame_collapse",
    "frame_influence",
    "scene_picking",
    "viewport",
    "camera_gesture",
    "cad",
    "mesh",
    "path2d",
    "subdivision_extrude",
    "subdivision_loft",
    "subdivision_sweep",
    "subdivision_revolve",
    "sketch_solve_diagnostics",
    "sketch_solve",
    "polygon_deform",
    "polygon_sculpt",
    "subdivision_sculpt",
    "polygon_brush",
    "polygon_extrude_faces",
    "subdivision_deform",
    "subdivision_brush",
    "sdf_deform",
    "sdf_sculpt",
    "sdf_sculpt_sphere",
    "polygon_extrude",
    "polygon_revolve",
    "polygon_loft",
    "polygon_sweep",
    "nurbs_patches_tessellate",
    "subdivision_refine",
    "subdivision_tessellate",
    "sdf_evaluate",
    "sdf_tessellate",
    "sdf_prepare",
    "sdf_prepare_batch",
    "sdf_finish",
    "surface_tessellate",
    "stl_decode_binary",
    "mesh_soup_render",
    "mesh_import",
    "mesh_import_finalize",
    "truss_force_markers",
    "affine_matrix",
    "planar_rectangle_corners",
    "nurbs_circle_quadrants",
    "scene_flatten",
    "body_bounds",
    "solid_program_execute_report",
    "solid_program_execute",
    "transparent_bsp_build",
    "transparent_triangle_split",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "frame_solve" => frame::solve(v),
        "frame_envelope" => frame::envelope(v),
        "frame_diagnose" => frame::diagnose(v),
        "frame_buckling" => frame::buckling(v),
        "frame_modal" => frame::modal(v),
        "frame_collapse" => frame::collapse(v),
        "frame_influence" => frame::influence(v),
        "scene_picking" => scene_picking::dispatch(v),
        "viewport" => viewport::dispatch(v),
        "camera_gesture" => camera_gestures::dispatch(v),
        "cad" | "mesh" => mesh::dispatch(v),
        "path2d" => path2d::dispatch(v),
        "subdivision_extrude" => encode(subdivision_core::Cage::extrude(
            &field::<Vec<[f64; 3]>>(&v, "profile")?,
            field(&v, "vector")?,
        )?),
        "subdivision_loft" => encode(subdivision_core::Cage::loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
            field(&v, "caps")?,
        )?),
        "subdivision_sweep" => encode(subdivision_core::Cage::sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
            field(&v, "caps")?,
        )?),
        "subdivision_revolve" => encode(subdivision_core::Cage::revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "segments")?,
        )?),
        "sketch_solve_diagnostics" => {
            let (solution, diagnostics) = sketch_core::solve_with_diagnostics_options(
                &field(&v, "sketch")?,
                sketch_core::SolverOptions {
                    tolerance: field(&v, "tolerance")?,
                    max_iterations: 64,
                },
            )?;
            encode(json!({"solution": solution, "diagnostics": {
                "redundantEquations": diagnostics.redundant_equations,
                "degenerateConstraints": diagnostics.degenerate_constraints,
                "constraintResiduals": diagnostics.constraint_residuals,
                "inconsistent": diagnostics.inconsistent,
            }}))
        }
        "sketch_solve" => encode(sketch_core::solve(
            &field(&v, "sketch")?,
            field(&v, "tolerance")?,
        )?),
        "polygon_deform" => encode(polygon_core::solid::edit::deform(
            &field(&v, "mesh")?,
            &field(&v, "deformation")?,
        )?),
        "polygon_sculpt" => encode(polygon_core::solid::edit::sculpt(
            &field(&v, "mesh")?,
            &field(&v, "brush")?,
        )?),
        "subdivision_sculpt" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.sculpt(&field(&v, "brush")?)?)
        }
        "polygon_brush" => encode(polygon_core::solid::edit::brush(
            &field(&v, "mesh")?,
            &field(&v, "brush")?,
        )?),
        "polygon_extrude_faces" => encode(polygon_core::solid::edit::extrude_faces(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "triangles")?,
            field(&v, "vector")?,
        )?),
        "subdivision_deform" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.deform(&field(&v, "deformation")?)?)
        }
        "subdivision_brush" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.brush(&field(&v, "brush")?)?)
        }
        "sdf_deform" => {
            encode(field::<sdf_core::Field>(&v, "field")?.deform(field(&v, "deformation")?)?)
        }
        "sdf_sculpt" => encode(field::<sdf_core::Field>(&v, "field")?.sculpt(&field::<
            sdf_core::SdfStroke,
        >(
            &v, "stroke"
        )?)?),
        "sdf_sculpt_sphere" => encode(field::<sdf_core::Field>(&v, "field")?.sculpt_sphere(
            field(&v, "center")?,
            field(&v, "radius")?,
            field(&v, "remove")?,
        )?),
        "polygon_extrude" => encode(polygon_core::solid::modeling::extrude(
            &field(&v, "profile")?,
            field(&v, "vector")?,
        )?),
        "polygon_revolve" => encode(polygon_core::solid::modeling::revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "angle")?,
            field(&v, "segments")?,
            field(&v, "caps")?,
        )?),
        "polygon_loft" => encode(polygon_core::solid::modeling::loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
            field(&v, "caps")?,
        )?),
        "polygon_sweep" => encode(polygon_core::solid::modeling::sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
            field(&v, "caps")?,
        )?),
        "nurbs_patches_tessellate" => encode(reconstruction::tessellate_surface_set(
            &field(&v, "patches")?,
            field(&v, "segments")?,
        )?),
        "subdivision_refine" => {
            encode(field::<subdivision_core::Cage>(&v, "cage")?.subdivide(field(&v, "levels")?)?)
        }
        "subdivision_tessellate" => {
            let refined =
                field::<subdivision_core::Cage>(&v, "cage")?.subdivide(field(&v, "levels")?)?;
            let (triangles, face_ids) = refined.triangulate()?;
            let mesh = mesh_from_triangles(triangles);
            let report = mesh.inspect()?;
            let mut value = encode(BuiltMesh { mesh, report })?;
            value["faceIds"] = json!(face_ids);
            Ok(value)
        }
        "sdf_evaluate" => {
            encode(field::<sdf_core::Field>(&v, "field")?.evaluate(field(&v, "point")?)?)
        }
        "sdf_tessellate" => {
            let mesh = mesh_from_triangles(sdf_core::polygonize(
                &field(&v, "field")?,
                &field(&v, "grid")?,
            )?);
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "sdf_prepare" => sdf_gpu::prepare(&v),
        "sdf_prepare_batch" => sdf_gpu::prepare_batch(&v),
        "sdf_finish" => sdf_gpu::finish(&v),
        "surface_tessellate" => encode(tessellate_nurbs(
            &field(&v, "surface")?,
            &field(&v, "options")?,
        )?),
        "stl_decode_binary" | "mesh_soup_render" | "mesh_import" | "mesh_import_finalize" => {
            mesh_import::dispatch(v)
        }
        "truss_force_markers" => encode(geometry_ops::force_markers::build(
            &field::<Vec<[f64; 3]>>(&v, "nodes")?,
            &field::<Vec<[usize; 2]>>(&v, "members")?,
            &field::<Vec<f64>>(&v, "forces")?,
            field(&v, "marker")?,
        )?),
        "affine_matrix" => {
            let operation: String = field(&v, "operation")?;
            let vector = field(&v, "vector")?;
            let matrix = match operation.as_str() {
                "translate" => Some(math_core::affine::translation(vector)),
                "scale" => Some(math_core::affine::scaling(vector)),
                "rotate" => Some(math_core::affine::euler_degrees(vector)),
                "mirror" => math_core::affine::reflection(vector),
                _ => return Err(input("Unknown affine operation")),
            };
            encode(matrix)
        }
        "planar_rectangle_corners" => encode(planar_geometry::primitives::rectangle_corners(
            field(&v, "size")?,
            field(&v, "center")?,
        )?),
        "nurbs_circle_quadrants" => encode(nurbs_core::primitives::circle_quadrants(field(
            &v, "radius",
        )?)?),
        "scene_flatten" => {
            let meshes = field::<Vec<Value>>(&v, "meshes")?
                .into_iter()
                .map(|m| {
                    Ok(polygon_core::scene_flatten::Input {
                        vertices: field(&m, "vertices")?,
                        indices: field(&m, "indices")?,
                        transform: field(&m, "transform")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            encode(polygon_core::scene_flatten::flatten(&meshes)?)
        }
        "body_bounds" => {
            let (min, max) =
                polygon_core::scene_flatten::bounds(&field::<Vec<Vec<f64>>>(&v, "positions")?)?;
            encode(json!({"min":min,"max":max}))
        }
        "solid_program_execute_report" => {
            let program: geometry_ops::solid_program::Program = field(&v, "program")?;
            let report = polygon_core::solid::program::execute_report(&program, 200_000)?;
            let mut warnings = report
                .slice_reductions
                .iter()
                .map(|reduction| {
                    format!(
                        "linear_extrude slices were clamped to the engine limit {}",
                        reduction.maximum
                    )
                })
                .collect::<Vec<_>>();
            warnings.extend(report.profile_diagnostics.iter().map(|diagnostic| {
                match diagnostic.issue {
                    polygon_core::solid::profile_program::Issue::EmptyContours => {
                        "polygon() outlines produced an empty cross-section".to_string()
                    }
                    polygon_core::solid::profile_program::Issue::InvalidContours => {
                        "polygon() outlines did not produce a valid cross-section".to_string()
                    }
                }
            }));
            warnings.extend(
                report
                    .mesh_failures
                    .iter()
                    .map(|_| "polyhedron() topology did not produce a manifold solid".to_string()),
            );
            warnings.extend(
                report
                    .resize_diagnostics
                    .iter()
                    .map(|event| event.message()),
            );
            warnings.extend(
                report
                    .sweep_warnings
                    .iter()
                    .map(|event| event.warning.message(event.maximum)),
            );
            let reduced = !report.slice_reductions.is_empty()
                || report.sweep_warnings.iter().any(|event| {
                    matches!(
                        event.warning.kind,
                        geometry_ops::fragment_resolution::WarningKind::Clamped
                    )
                });
            Ok(json!({"meshes":report.meshes,"warnings":warnings,"reduced":reduced}))
        }
        "solid_program_execute" => {
            let program: geometry_ops::solid_program::Program = field(&v, "program")?;
            encode(polygon_core::solid::program::execute(&program, 200_000)?)
        }
        "transparent_bsp_build" => transparent_bsp::build(v),
        "transparent_triangle_split" => transparent_bsp::split(v),
        _ => unreachable!("route admits only OPS"),
    }
}
