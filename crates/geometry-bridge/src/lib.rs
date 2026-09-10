//! Application adapter between two independent Rust geometry libraries.
//! Owns the NURBS-to-polygon sampler and the WASM/JSON transport, not a third
//! geometry representation. Native clients can use the same typed adapters.
pub mod brep;
mod cad;
pub mod reconstruction;
pub mod mesh_shell;
mod sdf_gpu;
#[cfg(feature = "gpu")]
pub mod lattice_gpu;
use nurbs_kernel::{
    curve::Curve,
    surface::{Surface, SurfaceSampler},
};
use polygon_kernel::{
    tessellation::{self, Boundary, Options, ParametricSurface},
    BuiltMesh, Mesh, Seams,
};
use value_codec::{json, Value};
use value_codec::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl value_codec::Serialize for Error {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("code".into(), value_codec::Serialize::to_value(&self.code));
        object.insert(
            "message".into(),
            value_codec::Serialize::to_value(&self.message),
        );
        value_codec::Value::Object(object)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
impl From<nurbs_kernel::Error> for Error {
    fn from(e: nurbs_kernel::Error) -> Self {
        Self {
            code: e.code,
            message: e.message,
        }
    }
}
impl From<polygon_kernel::Error> for Error {
    fn from(e: polygon_kernel::Error) -> Self {
        Self {
            code: e.code,
            message: e.message,
        }
    }
}
fn input(message: impl Into<String>) -> Error {
    Error {
        code: "GEOMETRY_INVALID_INPUT",
        message: message.into(),
    }
}
fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
fn encode(v: impl Serialize) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| input(e.to_string()))
}
fn response(result: Result<Value>) -> String {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error}),
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
    fn point(&self, u: f64, v: f64) -> polygon_kernel::Result<[f64; 3]> {
        self.sampler
            .evaluate(u, v)
            .map(|e| e.point)
            .map_err(|e| polygon_kernel::Error {
                code: e.code,
                message: e.message,
            })
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
                .collect::<polygon_kernel::Result<Vec<_>>>()?;
            Ok(Curve::from_polyline(points)?)
        })
        .collect()
}
pub fn dispatch(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "cad" => cad::dispatch(v),
        "subdivision_extrude" => encode(subdivision_kernel::Cage::extrude(
            &field::<Vec<[f64; 3]>>(&v, "profile")?,
            field(&v, "vector")?,
        )?),
        "subdivision_loft" => encode(subdivision_kernel::Cage::loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
            field(&v, "caps")?,
        )?),
        "subdivision_sweep" => encode(subdivision_kernel::Cage::sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
            field(&v, "caps")?,
        )?),
        "subdivision_revolve" => encode(subdivision_kernel::Cage::revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "segments")?,
        )?),
        "sketch_solve" => encode(
            sketch_kernel::solve(&field(&v, "sketch")?, field(&v, "tolerance")?).map_err(input)?,
        ),
        "polygon_deform" => encode(polygon_kernel::edit::deform(
            &field(&v, "mesh")?,
            &field(&v, "deformation")?,
        )?),
        "polygon_brush" => encode(polygon_kernel::edit::brush(
            &field(&v, "mesh")?,
            &field(&v, "brush")?,
        )?),
        "polygon_extrude_faces" => encode(polygon_kernel::edit::extrude_faces(
            &field(&v, "mesh")?,
            &field::<Vec<usize>>(&v, "triangles")?,
            field(&v, "vector")?,
        )?),
        "subdivision_deform" => encode(
            field::<subdivision_kernel::Cage>(&v, "cage")?.deform(&field(&v, "deformation")?)?,
        ),
        "subdivision_brush" => {
            encode(field::<subdivision_kernel::Cage>(&v, "cage")?.brush(&field(&v, "brush")?)?)
        }
        "sdf_deform" => {
            encode(field::<sdf_kernel::Field>(&v, "field")?.deform(field(&v, "deformation")?)?)
        }
        "sdf_sculpt_sphere" => encode(field::<sdf_kernel::Field>(&v, "field")?.sculpt_sphere(
            field(&v, "center")?,
            field(&v, "radius")?,
            field(&v, "remove")?,
        )?),
        "polygon_extrude" => encode(polygon_kernel::modeling::extrude(
            &field(&v, "profile")?,
            field(&v, "vector")?,
        )?),
        "polygon_revolve" => encode(polygon_kernel::modeling::revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "angle")?,
            field(&v, "segments")?,
            field(&v, "caps")?,
        )?),
        "polygon_loft" => encode(polygon_kernel::modeling::loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
            field(&v, "caps")?,
        )?),
        "polygon_sweep" => encode(polygon_kernel::modeling::sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
            field(&v, "caps")?,
        )?),
        "mesh_to_nurbs_brep" => encode(reconstruction::nurbs_brep_from_mesh(&field(&v, "mesh")?)?),
        "mesh_to_sdf" => encode(sdf_kernel::Field::from_mesh(
            &field(&v, "mesh")?,
            field(&v, "signed")?,
        )?),
        "mesh_to_subdivision" => encode(subdivision_kernel::reconstruct(
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
            encode(field::<subdivision_kernel::Cage>(&v, "cage")?.subdivide(field(&v, "levels")?)?)
        }
        "subdivision_tessellate" => {
            let refined =
                field::<subdivision_kernel::Cage>(&v, "cage")?.subdivide(field(&v, "levels")?)?;
            let (mesh, face_ids) = refined.triangulate()?;
            let report = mesh.inspect()?;
            let mut value = encode(BuiltMesh { mesh, report })?;
            value["faceIds"] = json!(face_ids);
            Ok(value)
        }
        "sdf_evaluate" => {
            encode(field::<sdf_kernel::Field>(&v, "field")?.evaluate(field(&v, "point")?)?)
        }
        "mesh_spatial_lattice" => encode(mesh_shell::lattice(&field(&v,"mesh")?,field(&v,"nodes")?,field(&v,"edges")?,field(&v,"radius")?,field(&v,"skin")?,field(&v,"step")?,field(&v,"organic")?,field(&v,"openTop")?,v.get("wallDepth").and_then(|x|x.as_f64()).unwrap_or(0.),v.get("keepCore").and_then(|x|x.as_bool()).unwrap_or(false))?),
        "mesh_shell_adaptive" => encode(mesh_shell::shell_options(&field(&v,"mesh")?, &field::<Vec<usize>>(&v,"openings")?, field(&v,"thickness")?, field(&v,"step")?, true)?),
        "mesh_shell_sampled" => encode(mesh_shell::shell(&field(&v,"mesh")?, &field::<Vec<usize>>(&v,"openings")?, field(&v,"thickness")?, field(&v,"step")?)?),
        "sdf_tessellate" => {
            let mesh = sdf_kernel::polygonize(&field(&v, "field")?, &field(&v, "grid")?)?;
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
        "sdf_prepare" => sdf_gpu::prepare(&v),
        "sdf_finish" => sdf_gpu::finish(&v),
        "surface_tessellate" => encode(tessellate_nurbs(
            &field(&v, "surface")?,
            &field(&v, "options")?,
        )?),
        "mesh_boolean" => encode(polygon_kernel::boolean::boolean(
            &field::<Mesh>(&v, "a")?,
            &field::<Mesh>(&v, "b")?,
            field(&v, "operation")?,
            &match v.get("options") {
                Some(options) => {
                    value_codec::from_value(options.clone()).map_err(|e| input(e.to_string()))?
                }
                None => polygon_kernel::boolean::Options::default(),
            },
        )?),
        "brep_nurbs_box" => encode(nurbs_kernel::brep::cuboid(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_inspect" => {
            encode(field::<nurbs_kernel::brep::Model>(&v, "model")?.validate()?)
        }
        "brep_nurbs_tessellate" => {
            encode(brep::nurbs(&field(&v, "model")?, field(&v, "segments")?)?)
        }
        "brep_nurbs_to_polygon" => {
            let t = brep::nurbs(&field(&v, "model")?, field(&v, "segments")?)?;
            encode(polygon_kernel::brep::from_mesh(
                &t.built.mesh,
                Some(&t.face_ids),
            )?)
        }
        "brep_polygon_from_mesh" => {
            let ids: Option<Vec<usize>> =
                v.get("faceIds").map(|_| field(&v, "faceIds")).transpose()?;
            encode(polygon_kernel::brep::from_mesh(
                &field(&v, "mesh")?,
                ids.as_deref(),
            )?)
        }
        "brep_polygon_inspect" => {
            polygon_kernel::brep::validate(&field(&v, "model")?)?;
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
        _ => Ok(nurbs_kernel::dispatch(v)?),
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

/// Source-to-graph frontend shared by browser workers and native callers.
pub fn compile_modelgraph_text(source: &str) -> String {
    match modelgraph_text::compile(source) {
        Ok(value) => json!({"ok":true,"value":value}).to_string(),
        Err(message) => json!({"ok":false,"message":message}).to_string(),
    }
}

/// Canonical graph preparation; errors retain the public ModelGraph code/path/details.
pub fn compile_modelgraph(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(modelgraph_runtime::compile),
        None,
    )
}
pub fn compile_modelgraph_nurbs(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(modelgraph_runtime::nurbs::compile),
        None,
    )
}
fn parse_graph_input(input: &str) -> modelgraph_runtime::Result<Value> {
    if input.len() > 2 * 1024 * 1024 {
        return Err(modelgraph_runtime::Error::new(
            "input_limit",
            "/",
            "Document exceeds transport limit.",
        ));
    }
    value_codec::from_str(input)
        .map_err(|e| modelgraph_runtime::Error::new("invalid_document", "/", e.to_string()))
}
fn runtime_response(
    result: modelgraph_runtime::Result<Value>,
    customizer: Option<Value>,
) -> String {
    runtime_value(result, customizer).to_string()
}
fn runtime_value(result: modelgraph_runtime::Result<Value>, customizer: Option<Value>) -> Value {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error,"customizer":customizer}),
    }
}
/// Fused source -> authoring graph -> evaluated graph, with no intermediate JS graph.
pub fn execute_modelgraph_text(source: &str) -> String {
    execute_text_value(source).to_string()
}
fn execute_text_value(source: &str) -> Value {
    let mut graph = match modelgraph_text::compile(source) {
        Ok(graph) => graph,
        Err(message) => {
            return runtime_value(
                Err(modelgraph_runtime::Error::new("text_error", "", message)),
                None,
            )
        }
    };
    let controls = graph["customizer"].take();
    let result = (|| {
        let nodes = graph["nodes"].take();
        let own = nodes.as_array().unwrap().iter().any(|n| {
            [
                "polygon_profile",
                "polygon_loft",
                "triangle_mesh",
                "subdivision",
                "sdf_sphere",
                "sdf_box",
                "sdf_torus",
                "brep_box",
                "nurbs_surface",
                "nurbs_curve",
                "mesh_boolean",
            ]
            .contains(&n["op"].as_str().unwrap_or(""))
        });
        let mut compiled = if own {
            if !graph["constraints"].as_array().unwrap().is_empty()
                || !graph["checks"].as_array().unwrap().is_empty()
            {
                return Err(modelgraph_runtime::Error::new(
                    "text_error",
                    "",
                    "NURBS text checks are not supported; use the build topology report",
                ));
            }
            if graph.get("segments").is_some() {
                return Err(modelgraph_runtime::Error::new("text_error", "", "segments applies only to legacy geometry; use explicit tessellation arguments for own geometry"));
            }
            let Value::Array(nodes) = nodes else {
                unreachable!()
            };
            let mut compiled = modelgraph_runtime::nurbs::compile_text(
                nodes,
                graph["parameters"].as_array().unwrap(),
                graph["root"].as_str().unwrap().into(),
            )?;
            compiled["source"] = json!("");
            for key in [
                "source_map",
                "geometry_assertions",
                "constraint_report",
                "sketch_solutions",
                "assembly_components",
                "mechanical_reports",
                "mechanical_parts",
            ] {
                compiled[key] = json!([]);
            }
            compiled
        } else {
            let mut document = value_codec::Map::new();
            document.insert("language".into(), json!("modelgraph/1"));
            document.insert("units".into(), json!("mm"));
            document.insert("nodes".into(), nodes);
            for key in ["parameters", "root"] {
                document.insert(key.into(), graph[key].take());
            }
            if let Some(segments) = graph.get_mut("segments") {
                document.insert("segments".into(), segments.take());
            }
            for (source, target) in [
                ("constraints", "constraints"),
                ("checks", "geometry_assertions"),
            ] {
                if !graph[source].as_array().unwrap().is_empty() {
                    document.insert(target.into(), graph[source].take());
                }
            }
            modelgraph_runtime::compile(Value::Object(document))?
        };
        compiled["customizer"] = controls.clone();
        Ok(compiled)
    })();
    runtime_value(result, Some(controls))
}

pub fn compile_modelgraph_text_nurbs(input: &str) -> String {
    let result = parse_graph_input(input).and_then(|v| {
        let nodes = v["nodes"].as_array().ok_or_else(|| {
            modelgraph_runtime::Error::new("invalid_document", "/nodes", "Expected nodes.")
        })?;
        let parameters = v["parameters"].as_array().ok_or_else(|| {
            modelgraph_runtime::Error::new(
                "invalid_document",
                "/parameters",
                "Expected parameters.",
            )
        })?;
        let root = v["root"].as_str().ok_or_else(|| {
            modelgraph_runtime::Error::new("invalid_document", "/root", "Expected root.")
        })?;
        modelgraph_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())
    });
    runtime_response(result, None)
}

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
    cad::import_buffers(stride, vertices, indices)
}

/// Linear-memory ABI core shared by the geometry-wasm / geometry-native shells.
pub mod abi;
