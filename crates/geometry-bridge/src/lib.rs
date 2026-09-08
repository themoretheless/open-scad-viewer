//! Application adapter between two independent Rust geometry libraries.
//! Owns the NURBS-to-polygon sampler and the WASM/JSON transport, not a third
//! geometry representation. Native clients can use the same typed adapters.
pub mod brep;
pub mod reconstruction;
use nurbs_kernel::{
    curve::Curve,
    surface::{Surface, SurfaceSampler},
};
use polygon_kernel::{
    tessellation::{self, Boundary, Options, ParametricSurface},
    BuiltMesh, Mesh, Seams,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[derive(Debug, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
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
    serde_json::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
fn encode(v: impl Serialize) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| input(e.to_string()))
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
        "sdf_tessellate" => {
            let mesh = sdf_kernel::polygonize(&field(&v, "field")?, &field(&v, "grid")?)?;
            let report = mesh.inspect()?;
            encode(BuiltMesh { mesh, report })
        }
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
                    serde_json::from_value(options.clone()).map_err(|e| input(e.to_string()))?
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
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn execute(input_text: &str) -> String {
    if input_text.len() > 32 * 1024 * 1024 {
        return response(Err(input("Geometry request exceeds 32 MiB")));
    }
    response(
        serde_json::from_str(input_text)
            .map_err(|e| input(e.to_string()))
            .and_then(dispatch),
    )
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct SurfaceEvaluator {
    sampler: SurfaceSampler,
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl SurfaceEvaluator {
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str) -> std::result::Result<SurfaceEvaluator, JsValue> {
        let result = (|| {
            if text.len() > 2 * 1024 * 1024 {
                return Err(input("Surface request exceeds 2 MiB"));
            }
            let surface: Surface = serde_json::from_str(text).map_err(|e| input(e.to_string()))?;
            Ok(Self {
                sampler: SurfaceSampler::new(&surface)?,
            })
        })();
        result.map_err(|e: Error| JsValue::from_str(&serde_json::to_string(&e).unwrap()))
    }
    pub fn evaluate(&self, u: f64, v: f64) -> String {
        response(
            self.sampler
                .evaluate(u, v)
                .map_err(Error::from)
                .and_then(encode),
        )
    }
}

#[cfg(test)]
mod tests;
