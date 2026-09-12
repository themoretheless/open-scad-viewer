//! Application adapter between two independent Rust geometry libraries.
//! Owns the NURBS-to-polygon sampler and the WASM/JSON transport, not a third
//! geometry representation. Native clients can use the same typed adapters.
//!
//! Default feature `languages` pulls OpenSCAD and ModelGraph. Kernel-only
//! builds: `--no-default-features`.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod brep;
#[cfg(feature = "languages")]
mod languages;
#[cfg(feature = "gpu")]
pub mod lattice_gpu;
mod mesh;
pub mod mesh_analysis;
pub mod mesh_shell;
#[cfg(feature = "languages")]
pub mod openscad;
#[cfg(feature = "languages")]
pub use languages::{
    compile_modelgraph, compile_modelgraph_nurbs, compile_modelgraph_text,
    compile_modelgraph_text_nurbs, execute_modelgraph_text,
};
mod path2d;
pub mod reconstruction;
mod sdf_gpu;
use nurbs_core::{
    curve::Curve,
    surface::{Surface, SurfaceSampler},
};
use polygon_core::{
    BuiltMesh, Mesh, Seams,
    solid::tessellation::{self, Boundary, Options, ParametricSurface},
};
use value_codec::{Deserialize, Serialize};
use value_codec::{Value, json};

pub use math_core::{Error, Result};
fn input(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
fn error_json(error: &Error) -> Value {
    json!({"code": error.code, "message": error.message})
}
pub(crate) fn mesh_from_triangles(t: geometry_ops::Triangles) -> Mesh {
    Mesh {
        positions: t.positions,
        indices: t.indices,
        uv: None,
    }
}
pub(crate) fn triangles_from_mesh(m: &Mesh) -> geometry_ops::Triangles {
    geometry_ops::Triangles {
        positions: m.positions.clone(),
        indices: m.indices.clone(),
    }
}
fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
fn encode(v: impl Serialize) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| input(e.to_string()))
}

fn toolpath_settings(v: &Value) -> Result<slicer_core::ToolpathSettings> {
    let mut settings = slicer_core::ToolpathSettings::default();
    if let Some(value) = v.get("layerHeightMm").and_then(|x| x.as_f64()) {
        settings.layer_height_mm = value;
    }
    if let Some(value) = v.get("lineWidthMm").and_then(|x| x.as_f64()) {
        settings.line_width_mm = value;
    }
    if let Some(value) = v.get("wallCount").and_then(|x| x.as_u64()) {
        settings.wall_count = value as usize;
    }
    if let Some(value) = v.get("infillSpacingMm").and_then(|x| x.as_f64()) {
        settings.infill_spacing_mm = value;
    }
    if let Some(value) = v.get("feedrateMmS").and_then(|x| x.as_f64()) {
        settings.feedrate_mm_s = value;
    }
    if let Some(value) = v.get("travelFeedrateMmS").and_then(|x| x.as_f64()) {
        settings.travel_feedrate_mm_s = value;
    }
    if let Some(value) = v.get("filamentDiameterMm").and_then(|x| x.as_f64()) {
        settings.filament_diameter_mm = value;
    }
    Ok(settings)
}

fn layer_from_mesh_section(
    section: polygon_core::solid::section::MeshSection,
) -> planar_geometry::LayerSection {
    planar_geometry::LayerSection {
        z_mm: section.z_mm,
        contours: section
            .contours
            .into_iter()
            .map(|contour| contour.points)
            .collect(),
    }
}

fn mesh_toolpaths(v: &Value) -> Result<Value> {
    let mesh: Mesh = field(v, "mesh")?;
    let z_min: f64 = field(v, "zMin")?;
    let z_max: f64 = field(v, "zMax")?;
    let settings = toolpath_settings(v)?;
    let index = polygon_core::solid::section::MeshSectionIndex::new(&mesh)?;
    let layers = slicer_core::schedule_layers(
        |z| {
            index
                .section(z)
                .map(layer_from_mesh_section)
                .map_err(|error| slicer_core::Error {
                    code: error.code,
                    message: error.message,
                })
        },
        z_min,
        z_max,
        &settings,
    )?;
    Ok(json!({
        "layers": layers.iter().map(|layer| json!({
            "z_mm": layer.z_mm,
            "paths": layer.paths.iter().map(|path| json!({
                "role": match path.role {
                    slicer_core::PathRole::Outline => "outline",
                    slicer_core::PathRole::Inset => "inset",
                    slicer_core::PathRole::Hatch => "hatch",
                },
                "closed": path.closed,
                "points": path.points,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
}

fn response(result: Result<Value>) -> String {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error_json(&error)}),
    }
    .to_string()
}

pub struct NurbsSurfaceAdapter {
    sampler: SurfaceSampler,
}
impl NurbsSurfaceAdapter {
    pub fn new(surface: &Surface) -> Result<Self> {
        Ok(Self {
            sampler: SurfaceSampler::new(surface)?,
        })
    }
}
impl ParametricSurface for NurbsSurfaceAdapter {
    fn domain(&self) -> [f64; 4] {
        let s = self.sampler.definition();
        [
            s.knots_u[s.degree_u],
            s.knots_u[s.control_points.len()],
            s.knots_v[s.degree_v],
            s.knots_v[s.control_points[0].len()],
        ]
    }
    fn point(&self, u: f64, v: f64) -> polygon_core::Result<[f64; 3]> {
        self.sampler.evaluate(u, v).map(|e| e.point)
    }
    fn boundary(&self) -> Boundary {
        let s = self.sampler.definition();
        let d = self.domain();
        let clamped = |k: &[f64], p: usize, a: f64, b: f64| {
            k[..=p].iter().all(|v| *v == a) && k[k.len() - p - 1..].iter().all(|v| *v == b)
        };
        let cu = clamped(&s.knots_u, s.degree_u, d[0], d[1]);
        let cv = clamped(&s.knots_v, s.degree_v, d[2], d[3]);
        let fu = s.control_points[0].clone();
        let lu = s.control_points.last().unwrap().clone();
        let fv: Vec<_> = s.control_points.iter().map(|r| r[0].clone()).collect();
        let lv: Vec<_> = s
            .control_points
            .iter()
            .map(|r| r.last().unwrap().clone())
            .collect();
        let equal = |a: &[Vec<f64>], b: &[Vec<f64>], wa: &[f64], wb: &[f64]| {
            let ratio = wb[0] / wa[0];
            a.iter()
                .enumerate()
                .all(|(i, p)| *p == b[i] && (wb[i] / wa[i] - ratio).abs() <= ratio.abs() * 1e-12)
        };
        let wfv: Vec<_> = s.weights.iter().map(|r| r[0]).collect();
        let wlv: Vec<_> = s.weights.iter().map(|r| *r.last().unwrap()).collect();
        let seams = Seams {
            u: s.periodic_u || (cu && equal(&fu, &lu, &s.weights[0], s.weights.last().unwrap())),
            v: s.periodic_v || (cv && equal(&fv, &lv, &wfv, &wlv)),
        };
        let rows = [&fu, &lu, &fv, &lv];
        let clamps = [cu, cu, cv, cv];
        let collapsed = std::array::from_fn(|i| {
            if clamps[i] && rows[i].iter().all(|p| *p == rows[i][0]) {
                Some([rows[i][0][0], rows[i][0][1], rows[i][0][2]])
            } else {
                None
            }
        });
        Boundary { seams, collapsed }
    }
}
/// NURBS definitions remain owned by the caller. The result is a derived mesh
/// with sampled UV correspondence; no exact/global error certificate is implied.
pub fn tessellate_nurbs(surface: &Surface, options: &Options) -> Result<BuiltMesh> {
    Ok(tessellation::tessellate(
        &NurbsSurfaceAdapter::new(surface)?,
        options,
    )?)
}
/// Exact piecewise-linear NURBS curves from ordered mesh boundary vertices.
/// This transfers polygon data into the spline library; it does not infer the
/// original smooth surface. Closed loops are clamped curves, not periodic data.
pub fn boundary_curves(mesh: &Mesh) -> Result<Vec<Curve>> {
    mesh.boundary_loops()?
        .iter()
        .map(|l| {
            let points = l
                .iter()
                .map(|i| mesh.point(*i).map(|p| p.to_vec()))
                .collect::<polygon_core::Result<Vec<_>>>()?;
            Ok(Curve::from_polyline(points)?)
        })
        .collect()
}
pub fn dispatch(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
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
        "sketch_solve" => encode(sketch_core::solve(
            &field(&v, "sketch")?,
            field(&v, "tolerance")?,
        )?),
        "polygon_deform" => encode(polygon_core::solid::edit::deform(
            &field(&v, "mesh")?,
            &field(&v, "deformation")?,
        )?),
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
        "nurbs_patches_tessellate" => encode(reconstruction::tessellate_patches(
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
        "sdf_tessellate" => {
            let mesh = mesh_from_triangles(sdf_core::polygonize(
                &field(&v, "field")?,
                &field(&v, "grid")?,
            )?);
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "sdf_prepare" => sdf_gpu::prepare(&v),
        "sdf_finish" => sdf_gpu::finish(&v),
        "surface_tessellate" => encode(tessellate_nurbs(
            &field(&v, "surface")?,
            &field(&v, "options")?,
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
        // P2 print path: section → toolpaths → optional G-code (crates already existed).
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
        "mesh_toolpaths" => mesh_toolpaths(&v),
        "mesh_gcode" => {
            let mesh: Mesh = field(&v, "mesh")?;
            let z_min: f64 = field(&v, "zMin")?;
            let z_max: f64 = field(&v, "zMax")?;
            let settings = toolpath_settings(&v)?;
            let index = polygon_core::solid::section::MeshSectionIndex::new(&mesh)?;
            let layers = slicer_core::schedule_layers(
                |z| {
                    index
                        .section(z)
                        .map(layer_from_mesh_section)
                        .map_err(|error| slicer_core::Error {
                            code: error.code,
                            message: error.message,
                        })
                },
                z_min,
                z_max,
                &settings,
            )?;
            let gcode = slicer_core::emit_gcode(&layers, &settings)?;
            Ok(json!({
                "gcode": gcode,
                "layerCount": layers.len(),
            }))
        }
        "brep_nurbs_box" => encode(brep_core::cuboid(field(&v, "min")?, field(&v, "max")?)?),
        "brep_nurbs_extrude_polygon" => encode(brep_core::extrude_polygon(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "zMin")?,
            field(&v, "zMax")?,
        )?),
        "brep_nurbs_faceted_cylinder" => encode(brep_core::faceted_cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_sphere" => encode(brep_core::faceted_sphere(
            field(&v, "radius")?,
            field(&v, "radialSegments")?,
            field(&v, "latitudeSegments")?,
        )?),
        "brep_nurbs_boolean" => encode(brep_core::boolean(
            &field(&v, "a")?,
            &field(&v, "b")?,
            v.get("operation")
                .and_then(|value| value.as_str())
                .ok_or_else(|| input("Invalid operation"))?,
        )?),
        "brep_nurbs_chamfer" => encode(brep_core::chamfer(
            &field(&v, "model")?,
            field(&v, "edge")?,
            field(&v, "size")?,
        )?),
        "brep_nurbs_chamfer_edges" => encode(brep_core::chamfer_edges(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "edges")?,
            field(&v, "size")?,
        )?),
        "brep_nurbs_fillet" => encode(brep_core::fillet(
            &field(&v, "model")?,
            field(&v, "edge")?,
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_fillet_edges" => encode(brep_core::fillet_edges(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "edges")?,
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_inspect" => encode(field::<brep_core::Model>(&v, "model")?.validate()?),
        "brep_nurbs_tessellate" => {
            encode(brep::nurbs(&field(&v, "model")?, field(&v, "segments")?)?)
        }
        "brep_nurbs_to_polygon" => {
            let t = brep::nurbs(&field(&v, "model")?, field(&v, "segments")?)?;
            encode(polygon_core::solid::brep::from_mesh(
                &t.built.mesh,
                Some(&t.face_ids),
            )?)
        }
        "brep_polygon_from_mesh" => {
            let ids: Option<Vec<usize>> =
                v.get("faceIds").map(|_| field(&v, "faceIds")).transpose()?;
            encode(polygon_core::solid::brep::from_mesh(
                &field(&v, "mesh")?,
                ids.as_deref(),
            )?)
        }
        "brep_polygon_inspect" => {
            polygon_core::solid::brep::validate(&field(&v, "model")?)?;
            encode(json!({"topologyValid":true,"solidGeometryStatus":"not_certified"}))
        }
        "brep_polygon_tessellate" => encode(brep::polygons(&field(&v, "model")?)?),
        "mesh_inspect" => encode(field::<Mesh>(&v, "mesh")?.inspect()?),
        "mesh_thicken" => encode(field::<Mesh>(&v, "mesh")?.thicken(field(&v, "vector")?)?),
        "mesh_transform" => {
            let mesh = field::<Mesh>(&v, "mesh")?.transform(field(&v, "matrix")?)?;
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "mesh_boundary_loops" => encode(field::<Mesh>(&v, "mesh")?.boundary_loops()?),
        "mesh_boundary_curves" => encode(boundary_curves(&field(&v, "mesh")?)?),
        "mesh_export_stl" => encode(field::<Mesh>(&v, "mesh")?.export_stl()?),
        _ => Ok(nurbs_core::dispatch(v)?),
    }
}
pub fn execute(input_text: &str) -> String {
    if input_text.len() > 32 * 1024 * 1024 {
        return response(Err(input("Geometry request exceeds 32 MiB")));
    }
    response(
        value_codec::from_str(input_text)
            .map_err(|e| input(e.to_string()))
            .and_then(dispatch),
    )
}
#[cfg(test)]
mod tests;

/// Owned binary transport snapshot. Pointers are borrowed until `free()`;
/// clients must reacquire the WASM memory buffer after allocating this object.
pub struct CadMeshBuffer {
    pub(crate) positions: Vec<f64>,
    pub(crate) indices: Vec<u32>,
    pub(crate) face_ids: Vec<u32>,
}
impl value_codec::Serialize for CadMeshBuffer {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "positions".into(),
            value_codec::Serialize::to_value(&self.positions),
        );
        object.insert(
            "indices".into(),
            value_codec::Serialize::to_value(&self.indices),
        );
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        value_codec::Value::Object(object)
    }
}
impl CadMeshBuffer {
    pub fn positions_ptr(&self) -> usize {
        self.positions.as_ptr() as usize
    }
    pub fn positions_len(&self) -> usize {
        self.positions.len()
    }
    pub fn indices_ptr(&self) -> usize {
        self.indices.as_ptr() as usize
    }
    pub fn indices_len(&self) -> usize {
        self.indices.len()
    }
    pub fn face_ids_ptr(&self) -> usize {
        self.face_ids.as_ptr() as usize
    }
    pub fn face_ids_len(&self) -> usize {
        self.face_ids.len()
    }
}
/// Typed native entry point shared with the WASM import adapter.
pub fn import_cad_mesh(stride: usize, vertices: &[f32], indices: &[u32]) -> Result<u32> {
    mesh::import_buffers(stride, vertices, indices)
}

/// Linear-memory ABI core shared by the geometry-wasm shell.
pub mod abi;
