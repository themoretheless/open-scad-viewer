#![doc = include_str!("../README.md")]
//! Negative-inside implicit fields and bounded marching-tetrahedra extraction.
//! CSG fields preserve the zero set but are not generally exact signed distances.
//! Extraction yields a neutral triangle buffer; mesh inspect lives in the bridge.
use geometry_ops::Triangles;
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Acceleration, Error, Result, V3, finite};
#[allow(unused_imports)]
use planar_geometry::rings::{self, Rings};
const INVALID_INPUT: &str = "SDF_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[cfg(feature = "cuda")]
pub mod cuda;
pub mod flat;
#[cfg(feature = "gpu")]
mod gpu;

#[cfg(feature = "gpu")]
pub fn gpu_backend_label() -> Option<&'static str> {
    gpu_backend_report().map(|report| report.label)
}

#[cfg(feature = "gpu")]
pub fn gpu_backend_report() -> Option<gpu_compute::BackendReport> {
    gpu::backend_report()
}


mod differential_and_queries;
mod field_eval;
mod wgsl;

pub use differential_and_queries::polygonize_surface_nets;
pub use wgsl::SDF_WGSL;
#[cfg(test)]
use field_eval::{point, triangles_closed, triangles_ok};
use field_eval::{finish_triangles, positive};

pub type Point = V3;
use math_core::{cross, dot, norm as length, sub};
#[derive(Clone, Debug)]
pub enum Field {
    Extrude {
        profile: Rings,
        half_height: f64,
    },
    Revolve {
        profile: Rings,
    },
    Deform {
        input: Box<Field>,
        deformation: geometry_ops::Deformation,
    },
    MeshDistance {
        mesh: Triangles,
        signed: bool,
    },
    Sphere {
        center: Point,
        radius: f64,
    },
    Box {
        center: Point,
        half_size: Point,
    },
    Torus {
        center: Point,
        major_radius: f64,
        minor_radius: f64,
    },
    Cylinder {
        center: Point,
        radius: f64,
        half_height: f64,
    },
    Gyroid {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    SchwarzP {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Diamond {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Neovius {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Lidinoid {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    SchoenIWP {
        origin: Point,
        period: f64,
        thickness: f64,
    },
    Capsule {
        a: Point,
        b: Point,
        radius: f64,
    },
    Octahedron {
        center: Point,
        size: f64,
    },
    HexPrism {
        center: Point,
        radius: f64,
        half_height: f64,
    },
    TriangularPrism {
        center: Point,
        radius: f64,
        half_height: f64,
    },
    Union {
        a: Box<Field>,
        b: Box<Field>,
    },
    Intersection {
        a: Box<Field>,
        b: Box<Field>,
    },
    Difference {
        a: Box<Field>,
        b: Box<Field>,
    },
    SmoothUnion {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    SmoothIntersection {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    SmoothDifference {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    ChamferUnion {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    ChamferIntersection {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    ChamferDifference {
        a: Box<Field>,
        b: Box<Field>,
        radius: f64,
    },
    Offset {
        input: Box<Field>,
        distance: f64,
    },
    Onion {
        input: Box<Field>,
        thickness: f64,
    },
    Elongate {
        input: Box<Field>,
        half_extents: Point,
    },
    Mirror {
        input: Box<Field>,
        axis: [bool; 3],
    },
    Taper {
        input: Box<Field>,
        height: f64,
        factor: f64,
    },
    Translate {
        input: Box<Field>,
        vector: Point,
    },
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub min: Point,
    pub max: Point,
    pub cells: [usize; 3],
}

/// Also accepts user-defined scalar fields. Sampling is bounded; sub-cell features
/// can be missed. A negative boundary sample is rejected rather than capped.
pub fn polygonize_with(field: impl Fn(Point) -> f64, grid: &Grid) -> Result<Triangles> {
    polygonize_tile(field, grid, true)
}
/// Extract an open tile for a caller that welds and validates the complete surface.
pub fn polygonize_tile(
    field: impl Fn(Point) -> f64,
    grid: &Grid,
    require_closed_bounds: bool,
) -> Result<Triangles> {
    if !finite(grid.min)
        || !finite(grid.max)
        || grid.cells.iter().any(|&n| n == 0 || n > 64)
        || (0..3).any(|i| grid.max[i] <= grid.min[i])
    {
        return Err(error(
            "Invalid grid: each axis requires 1..64 cells and ordered finite bounds",
        ));
    }
    let [nx, ny, nz] = grid.cells;
    let index = |x: usize, y: usize, z: usize| (z * (ny + 1) + y) * (nx + 1) + x;
    let sample_count = (nx + 1) * (ny + 1) * (nz + 1);
    let mut points = Vec::with_capacity(sample_count);
    let mut values = Vec::with_capacity(sample_count);
    for z in 0..=nz {
        for y in 0..=ny {
            for x in 0..=nx {
                let p = std::array::from_fn(|i| {
                    grid.min[i]
                        + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64 / grid.cells[i] as f64
                });
                let v = field(p);
                let v = if v.abs()
                    <= 32.
                        * f64::EPSILON
                        * (0..3).map(|i| grid.max[i] - grid.min[i]).fold(0., f64::max)
                {
                    0.
                } else {
                    v
                };
                if !v.is_finite() {
                    return Err(error("Field returned a non-finite value"));
                }
                if require_closed_bounds
                    && (x == 0 || y == 0 || z == 0 || x == nx || y == ny || z == nz)
                    && v <= 0.
                {
                    return Err(error("Surface touches grid boundary; enlarge bounds"));
                }
                points.push(p);
                values.push(v);
            }
        }
    }
    let cell_estimate = (nx * ny + ny * nz + nz * nx) * 2;
    let mut mesh = Triangles {
        positions: Vec::with_capacity(cell_estimate * 3),
        indices: Vec::with_capacity(cell_estimate * 6),
    };
    // Edge cache is only ever probed by key (entry API); iteration order never
    // reaches the mesh, so a hash map keeps the output bit-identical while
    // avoiding per-entry tree comparisons.
    let mut cache = rustc_hash::FxHashMap::with_capacity_and_hasher(
        cell_estimate,
        Default::default(),
    );
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let cube = [
                    index(x, y, z),
                    index(x + 1, y, z),
                    index(x + 1, y + 1, z),
                    index(x, y + 1, z),
                    index(x, y, z + 1),
                    index(x + 1, y, z + 1),
                    index(x + 1, y + 1, z + 1),
                    index(x, y + 1, z + 1),
                ];
                for tet in [
                    [0, 1, 2, 6],
                    [0, 2, 3, 6],
                    [0, 3, 7, 6],
                    [0, 7, 4, 6],
                    [0, 4, 5, 6],
                    [0, 5, 1, 6],
                ] {
                    let t = tet.map(|i| cube[i]);
                    let mut polygon = [0usize; 4];
                    let mut poly_len = 0usize;
                    let mut outward = [0.; 3];
                    let mut ni = 0.;
                    let mut no = 0.;
                    let mut inside = [0.; 3];
                    let mut outside = [0.; 3];
                    for &i in &t {
                        if values[i] < 0. {
                            ni += 1.;
                            for (k, v) in inside.iter_mut().enumerate() {
                                *v += points[i][k];
                            }
                        } else {
                            no += 1.;
                            for (k, v) in outside.iter_mut().enumerate() {
                                *v += points[i][k];
                            }
                        }
                    }
                    if ni == 0. || no == 0. {
                        continue;
                    }
                    for k in 0..3 {
                        outward[k] = outside[k] / no - inside[k] / ni;
                    }
                    for [a, b] in [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]] {
                        let a = t[a];
                        let b = t[b];
                        if (values[a] < 0.) == (values[b] < 0.) {
                            continue;
                        }
                        let key = if values[a] == 0. {
                            (a, a)
                        } else if values[b] == 0. {
                            (b, b)
                        } else {
                            (a.min(b), a.max(b))
                        };
                        let id = *cache.entry(key).or_insert_with(|| {
                            let ratio = values[a] / (values[a] - values[b]);
                            let id = mesh.positions.len() / 3;
                            mesh.positions.extend(
                                points[a]
                                    .iter()
                                    .zip(points[b])
                                    .map(|(&a, b)| a + ratio * (b - a)),
                            );
                            id
                        });
                        if !polygon[..poly_len].contains(&id) {
                            polygon[poly_len] = id;
                            poly_len += 1;
                        }
                    }
                    if poly_len < 3 {
                        continue;
                    }
                    let polygon = &mut polygon[..poly_len];
                    let point = |i: usize| {
                        [
                            mesh.positions[i * 3],
                            mesh.positions[i * 3 + 1],
                            mesh.positions[i * 3 + 2],
                        ]
                    };
                    let mut center = [0.; 3];
                    for &i in &*polygon {
                        for (k, v) in center.iter_mut().enumerate() {
                            *v += point(i)[k] / polygon.len() as f64;
                        }
                    }
                    let u = sub(point(polygon[0]), center);
                    let v = cross(outward, u);
                    polygon.sort_by(|&a, &b| {
                        let a = sub(point(a), center);
                        let b = sub(point(b), center);
                        dot(a, v)
                            .atan2(dot(a, u))
                            .total_cmp(&dot(b, v).atan2(dot(b, u)))
                    });
                    for i in 1..polygon.len() - 1 {
                        let a = polygon[0];
                        let mut b = polygon[i];
                        let mut c = polygon[i + 1];
                        let normal = cross(sub(point(b), point(a)), sub(point(c), point(a)));
                        if length(normal) == 0. {
                            continue;
                        }
                        if dot(normal, outward) < 0. {
                            std::mem::swap(&mut b, &mut c);
                        }
                        mesh.indices.extend([a, b, c]);
                    }
                    if mesh.indices.len() / 3 > 100_000 {
                        return Err(error("Implicit mesh exceeds 100000 triangles"));
                    }
                }
            }
        }
    }
    finish_triangles(mesh)
}
/// Budget and field validation shared by every extraction path.
pub fn check_grid_budget(field: &Field, grid: &Grid) -> Result<()> {
    field.validate()?;
    if grid.cells.iter().any(|&n| n > 64)
        || grid
            .cells
            .iter()
            .map(|n| n + 1)
            .product::<usize>()
            .saturating_mul(field.sample_work())
            > 8_000_000
    {
        return Err(error("SDF extraction exceeds 8000000 sample work units"));
    }
    Ok(())
}
pub fn polygonize(field: &Field, grid: &Grid) -> Result<Triangles> {
    check_grid_budget(field, grid)?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        if finite(grid.min)
            && finite(grid.max)
            && grid.cells.iter().all(|&n| (1..=64).contains(&n))
            && (0..3).all(|i| grid.max[i] > grid.min[i])
        {
            let [nx, ny, nz] = grid.cells;
            let sample_count = (nx + 1) * (ny + 1) * (nz + 1);
            let total_work = sample_count.saturating_mul(field.sample_work());
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            if threads > 1 && nz >= 2 && total_work >= 4_096 {
                let plane_size = (nx + 1) * (ny + 1);
                let z_count = nz + 1;
                let chunk_z = z_count.div_ceil(threads).max(1);
                let mut precomputed = Vec::with_capacity(sample_count);
                std::thread::scope(|s| {
                    let mut handles = Vec::with_capacity(z_count.div_ceil(chunk_z));
                    let mut z_start = 0usize;
                    while z_start < z_count {
                        let z_end = (z_start + chunk_z).min(z_count);
                        handles.push(s.spawn(move || {
                            let mut chunk_vals = Vec::with_capacity((z_end - z_start) * plane_size);
                            for z in z_start..z_end {
                                for y in 0..=ny {
                                    for x in 0..=nx {
                                        let p = std::array::from_fn(|i| {
                                            grid.min[i]
                                                + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                                    / grid.cells[i] as f64
                                        });
                                        chunk_vals.push(field.sample(p));
                                    }
                                }
                            }
                            chunk_vals
                        }));
                        z_start = z_end;
                    }
                    for handle in handles {
                        precomputed.extend(handle.join().expect("sdf worker panicked"));
                    }
                });
                let cursor = std::cell::Cell::new(0usize);
                return polygonize_with(
                    |_| {
                        let i = cursor.get();
                        cursor.set(i + 1);
                        precomputed[i]
                    },
                    grid,
                );
            }
        }
    }
    polygonize_with(|p| field.sample(p), grid)
}
/// Rebuilds the mesh from externally computed grid samples (GPU shader or the
/// browser WebGPU host path). The snap-to-zero, boundary validation and
/// marching-tetrahedra extraction run here on the CPU, exactly as in
/// `polygonize`; only the raw field values arrive precomputed (f32).
pub fn polygonize_with_values(field: &Field, grid: &Grid, values: &[f32]) -> Result<Triangles> {
    check_grid_budget(field, grid)?;
    let expected = grid.cells.iter().map(|n| n + 1).product::<usize>();
    if values.len() != expected {
        return Err(error("Grid sample count does not match the grid"));
    }
    if values.iter().any(|v| !v.is_finite()) {
        return Err(error("Field returned a non-finite value"));
    }
    let cursor = std::cell::Cell::new(0usize);
    polygonize_with(
        |_| {
            let i = cursor.get();
            cursor.set(i + 1);
            values[i] as f64
        },
        grid,
    )
}

fn grid_sample_count(grid: &Grid) -> usize {
    grid.cells.iter().map(|n| n + 1).product()
}

/// Suggests a placement for [`polygonize_accelerated`] from grid size and SDF
/// field cost. Primitive CSG fields are cheap per point, while mesh-distance
/// fields do a brute-force triangle scan at every grid sample; this combines
/// both into `grid samples * field.sample_work()` units.
///
/// Measured with `examples/bench_gpu.rs` on an NVIDIA RTX 5090 (CUDA 13.4):
/// primitive CSG at 64^3 (~550K work units) is only slightly faster on
/// CUDA/wgpu (67ms CPU vs 62ms device), while a 1088-triangle mesh-distance
/// field at 16^3 (~5.3M work units) is ~200x faster (324ms CPU vs 1.3-1.6ms
/// device). `Auto` therefore stays on CPU below 500K work and prefers CUDA
/// (or portable wgpu, including Metal on macOS) once the device launch cost is
/// likely amortized.
pub fn recommended_for_polygonize(field: &Field, grid: &Grid) -> Acceleration {
    const SDF_GPU_WORK_THRESHOLD: usize = 500_000;
    let work = grid_sample_count(grid).saturating_mul(field.sample_work());
    if work < SDF_GPU_WORK_THRESHOLD {
        Acceleration::Cpu
    } else if cfg!(feature = "cuda") {
        Acceleration::Cuda
    } else {
        Acceleration::Gpu
    }
}

fn resolve_polygonize_acceleration(
    acceleration: Acceleration,
    field: &Field,
    grid: &Grid,
) -> Acceleration {
    match acceleration {
        Acceleration::Auto => recommended_for_polygonize(field, grid),
        explicit => explicit,
    }
}

/// `polygonize` with an optional GPU grid sampler. Eligible fields (primitive
/// and CSG trees) sample the grid on the GPU in f32; the snap-to-zero, boundary
/// validation and marching-tetrahedra extraction stay on the CPU. Everything
/// else — and any failure — falls back to the CPU reference.
///
/// `Acceleration::Auto` uses [`recommended_for_polygonize`].
/// `Acceleration::Cuda` runs the PTX port through the CUDA driver (feature
/// `cuda`), then the wgpu shader (feature `gpu`, Metal on macOS), then the CPU
/// reference.
pub fn polygonize_accelerated(
    field: &Field,
    grid: &Grid,
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Result<Triangles> {
    #[allow(unused_variables)]
    let acceleration = resolve_polygonize_acceleration(acceleration, field, grid);
    #[cfg(feature = "gpu")]
    if acceleration.is_gpu() {
        check_grid_budget(field, grid)?;
        if let Some(flat) = field.to_flat() {
            #[cfg(feature = "cuda")]
            if acceleration == Acceleration::Cuda
                && let Some(values) = cuda::sample_grid_cuda(&flat, grid)
            {
                return polygonize_with_values(field, grid, &values);
            }
            if let Some(values) = gpu::sample_grid_gpu(&flat, grid) {
                return polygonize_with_values(field, grid, &values);
            }
        }
    }
    polygonize(field, grid)
}

impl Field {
    pub fn deform(&self, deformation: geometry_ops::Deformation) -> Result<Self> {
        self.validate()?;
        let out = Self::Deform {
            input: Box::new(self.clone()),
            deformation,
        };
        out.validate()?;
        Ok(out)
    }
    /// Applies one sculpt stroke: a primitive tool shape combined with the field
    /// by hard or smooth-blended add/remove.
    pub fn sculpt(&self, stroke: &SdfStroke) -> Result<Self> {
        self.validate()?;
        stroke.validate()?;
        let tool = stroke.tool.field();
        tool.validate()?;
        let a = Box::new(self.clone());
        let b = Box::new(tool);
        let out = match (stroke.remove, stroke.blend) {
            (false, None) => Self::Union { a, b },
            (true, None) => Self::Difference { a, b },
            (false, Some(radius)) => Self::SmoothUnion { a, b, radius },
            (true, Some(radius)) => Self::SmoothDifference { a, b, radius },
        };
        out.validate()?;
        Ok(out)
    }
    /// Hard sphere add/remove; the historical entry point, now a `sculpt` shorthand.
    pub fn sculpt_sphere(&self, center: Point, radius: f64, remove: bool) -> Result<Self> {
        self.sculpt(&SdfStroke {
            tool: SdfTool::Sphere { center, radius },
            remove,
            blend: None,
        })
    }
}

/// Tool shape of an SDF sculpt stroke.
#[derive(Clone, Debug, PartialEq)]
pub enum SdfTool {
    Sphere { center: Point, radius: f64 },
    Box { center: Point, half_size: Point },
    Capsule { a: Point, b: Point, radius: f64 },
}
impl SdfTool {
    pub fn field(&self) -> Field {
        match self {
            Self::Sphere { center, radius } => Field::Sphere {
                center: *center,
                radius: *radius,
            },
            Self::Box { center, half_size } => Field::Box {
                center: *center,
                half_size: *half_size,
            },
            Self::Capsule { a, b, radius } => Field::Capsule {
                a: *a,
                b: *b,
                radius: *radius,
            },
        }
    }
}

/// One SDF sculpt stroke. `blend` is the smooth-blend radius; `None` keeps hard CSG.
#[derive(Clone, Debug, PartialEq)]
pub struct SdfStroke {
    pub tool: SdfTool,
    pub remove: bool,
    pub blend: Option<f64>,
}
impl SdfStroke {
    pub fn validate(&self) -> Result<()> {
        self.tool.field().validate()?;
        match self.blend {
            Some(k) if !positive(k) => Err(error("Invalid sculpt blend radius")),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygonize_auto_recommendation_tracks_work_size() {
        let sphere = Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        };
        let tiny = Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [8, 8, 8],
        };
        assert_eq!(
            recommended_for_polygonize(&sphere, &tiny),
            Acceleration::Cpu
        );
        let large = Grid {
            cells: [64, 64, 64],
            ..tiny.clone()
        };
        let smooth = Field::SmoothUnion {
            a: Box::new(sphere.clone()),
            b: Box::new(Field::Box {
                center: [0.5; 3],
                half_size: [0.75; 3],
            }),
            radius: 0.25,
        };
        assert!(recommended_for_polygonize(&smooth, &large).is_gpu());
        let mut mesh = Triangles {
            positions: vec![
                0., 0., 0., //
                1., 0., 0., 0., 1., 0.,
            ],
            indices: Vec::new(),
        };
        for _ in 0..1024 {
            mesh.indices.extend([0, 1, 2]);
        }
        let mesh_field = Field::from_triangles(mesh, false).unwrap();
        assert!(recommended_for_polygonize(&mesh_field, &tiny).is_gpu());
    }

    #[test]
    fn polygonize_auto_matches_cpu_on_small_grid() {
        let field = Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        };
        let grid = Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [8, 8, 8],
        };
        let cpu = polygonize(&field, &grid).unwrap();
        let automatic = polygonize_accelerated(&field, &grid, Acceleration::Auto).unwrap();
        assert_eq!(cpu.indices, automatic.indices);
        assert_eq!(cpu.positions, automatic.positions);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_sampling_matches_cpu_field_within_tolerance() {
        let field = Field::Translate {
            input: Box::new(Field::SmoothUnion {
                a: Box::new(Field::Sphere {
                    center: [0., 0., 0.],
                    radius: 10.,
                }),
                b: Box::new(Field::Box {
                    center: [8., 0., 0.],
                    half_size: [6., 6., 6.],
                }),
                radius: 3.,
            }),
            vector: [1., -2., 0.5],
        };
        let Some(flat) = field.to_flat() else {
            panic!("CSG tree should flatten")
        };
        let grid = Grid {
            min: [-12., -12., -12.],
            max: [16., 12., 12.],
            cells: [24, 16, 16],
        };
        let Some(values) = gpu::sample_grid_gpu(&flat, &grid) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        let mut max_diff = 0f64;
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let gpu = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((gpu - field.sample(p)).abs());
                }
            }
        }
        assert!(max_diff < 0.01, "GPU field diverges: {max_diff}");
        // Full extraction: same topology class and volume within f32 tolerance.
        let cpu_mesh = polygonize(&field, &grid).unwrap();
        let gpu_mesh = polygonize_accelerated(&field, &grid, Acceleration::Gpu).unwrap();
        let cpu_tris = cpu_mesh.indices.len() / 3;
        let gpu_tris = gpu_mesh.indices.len() / 3;
        assert!(
            (cpu_tris as f64 - gpu_tris as f64).abs() <= 0.02 * cpu_tris as f64,
            "triangle counts diverge: {cpu_tris} vs {gpu_tris}"
        );
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_mesh_distance_matches_cpu_signed_and_unsigned() {
        // Closed outward tetrahedron around the origin-ish region.
        let mesh = Triangles {
            positions: vec![
                10., 10., 10., //
                30., 10., 10., 10., 30., 10., 10., 10., 30.,
            ],
            indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
        };
        let field = Field::from_triangles(mesh, true).unwrap();
        let Some(flat) = field.to_flat() else {
            panic!("mesh field should flatten")
        };
        assert_eq!(flat.triangles.len(), 4 * 9);
        let grid = Grid {
            min: [0., 0., 0.],
            max: [40., 40., 40.],
            cells: [8, 8, 8],
        };
        let Some(values) = gpu::sample_grid_gpu(&flat, &grid) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        let mut max_diff = 0f64;
        let mut inside_cpu = 0;
        let mut inside_gpu = 0;
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let cpu = field.sample(p);
                    let gpu = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((gpu - cpu).abs());
                    if cpu < 0. {
                        inside_cpu += 1;
                    }
                    if gpu < 0. {
                        inside_gpu += 1;
                    }
                }
            }
        }
        assert!(inside_cpu > 0, "tetrahedron should contain grid points");
        assert_eq!(
            inside_cpu, inside_gpu,
            "inside/outside classification diverges"
        );
        assert!(max_diff < 0.01, "signed distance diverges: {max_diff}");
    }

    /// CSG tree plus a signed mesh-distance leaf: every node kind the flat
    /// interpreter implements, so the CUDA port is exercised end to end.
    #[cfg(feature = "cuda")]
    fn cuda_fixture() -> (Field, Grid) {
        let tetra = Triangles {
            positions: vec![
                -6., -6., -6., //
                6., -6., -6., -6., 6., -6., -6., -6., 6.,
            ],
            indices: vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
        };
        let field = Field::Translate {
            input: Box::new(Field::Difference {
                a: Box::new(Field::SmoothUnion {
                    a: Box::new(Field::Sphere {
                        center: [0., 0., 0.],
                        radius: 8.,
                    }),
                    b: Box::new(Field::Offset {
                        input: Box::new(Field::Torus {
                            center: [4., 0., 0.],
                            major_radius: 4.,
                            minor_radius: 1.5,
                        }),
                        distance: 0.5,
                    }),
                    radius: 3.,
                }),
                b: Box::new(Field::Intersection {
                    a: Box::new(Field::Box {
                        center: [0., 0., 0.],
                        half_size: [5., 5., 5.],
                    }),
                    b: Box::new(Field::from_triangles(tetra, true).unwrap()),
                }),
            }),
            vector: [1., -2., 0.5],
        };
        let grid = Grid {
            min: [-12., -12., -12.],
            max: [16., 12., 12.],
            cells: [28, 24, 24],
        };
        (field, grid)
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_sampling_matches_cpu_field_within_tolerance() {
        let (field, grid) = cuda_fixture();
        let flat = field.to_flat().expect("fixture should flatten");
        let Some(values) = cuda::sample_grid_cuda(&flat, &grid) else {
            eprintln!("no CUDA device; skipping");
            return;
        };
        let [nx, ny, nz] = grid.cells;
        assert_eq!(values.len(), (nx + 1) * (ny + 1) * (nz + 1));
        let mut max_diff = 0f64;
        let (mut inside_cpu, mut inside_cuda) = (0, 0);
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p = std::array::from_fn(|i| {
                        grid.min[i]
                            + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64
                                / grid.cells[i] as f64
                    });
                    let cpu = field.sample(p);
                    let cuda = values[(z * (ny + 1) + y) * (nx + 1) + x] as f64;
                    max_diff = max_diff.max((cuda - cpu).abs());
                    inside_cpu += (cpu < 0.) as usize;
                    inside_cuda += (cuda < 0.) as usize;
                }
            }
        }
        assert!(inside_cpu > 0);
        assert_eq!(
            inside_cpu, inside_cuda,
            "inside/outside classification diverges"
        );
        assert!(max_diff < 0.01, "CUDA field diverges: {max_diff}");
        let cpu_mesh = polygonize(&field, &grid).unwrap();
        let cuda_mesh = polygonize_accelerated(&field, &grid, Acceleration::Cuda).unwrap();
        let cpu_tris = cpu_mesh.indices.len() / 3;
        let cuda_tris = cuda_mesh.indices.len() / 3;
        assert!(
            (cpu_tris as f64 - cuda_tris as f64).abs() <= 0.02 * cpu_tris as f64,
            "triangle counts diverge: {cpu_tris} vs {cuda_tris}"
        );
    }

    /// The PTX and WGSL kernels are ports of the same text; on the same device
    /// class they should agree to f32 rounding, far tighter than the CPU bound.
    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_and_wgpu_samplers_agree() {
        let (field, grid) = cuda_fixture();
        let flat = field.to_flat().expect("fixture should flatten");
        let (Some(cuda), Some(wgpu)) = (
            cuda::sample_grid_cuda(&flat, &grid),
            gpu::sample_grid_gpu(&flat, &grid),
        ) else {
            eprintln!("CUDA or wgpu unavailable; skipping");
            return;
        };
        assert_eq!(cuda.len(), wgpu.len());
        let max_diff = cuda
            .iter()
            .zip(&wgpu)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(
            max_diff < 1e-3,
            "CUDA and wgpu samplers diverge: {max_diff}"
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_request_without_device_falls_back() {
        // Whatever the host has, `Cuda` must never fail where the CPU succeeds.
        let (field, grid) = cuda_fixture();
        let cpu = polygonize(&field, &grid).unwrap();
        let cuda = polygonize_accelerated(&field, &grid, Acceleration::Cuda).unwrap();
        assert!(!cuda.indices.is_empty());
        let flat = field.to_flat().unwrap();
        if !cuda::available() && gpu::sample_grid_gpu(&flat, &grid).is_none() {
            assert_eq!(
                cpu.indices, cuda.indices,
                "CPU fallback must be bit-identical"
            );
        }
    }

    fn sphere() -> Field {
        Field::Sphere {
            center: [0.; 3],
            radius: 1.,
        }
    }
    fn grid(n: usize) -> Grid {
        Grid {
            min: [-1.5; 3],
            max: [1.5; 3],
            cells: [n; 3],
        }
    }
    #[test]
    fn primitives_and_csg() {
        assert_eq!(sphere().evaluate([0.; 3]).unwrap(), -1.);
        let f = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [0.; 3],
                radius: 0.5,
            }),
        };
        assert_eq!(f.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(f.evaluate([0.75, 0., 0.]).unwrap() < 0.);
    }
    fn signed_volume(mesh: &Triangles) -> f64 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let a = point(mesh, t[0]);
                let b = point(mesh, t[1]);
                let c = point(mesh, t[2]);
                dot(a, cross(b, c)) / 6.
            })
            .sum()
    }
    #[test]
    fn sculpt_sphere_adds_and_removes_material() {
        let base = sphere();
        let added = base.sculpt_sphere([1.5, 0., 0.], 0.5, false).unwrap();
        assert!(matches!(added, Field::Union { .. }));
        assert_eq!(base.evaluate([1.5, 0., 0.]).unwrap(), 0.5);
        assert_eq!(added.evaluate([1.5, 0., 0.]).unwrap(), -0.5);
        assert_eq!(added.evaluate([0.; 3]).unwrap(), -1.);
        let removed = base.sculpt_sphere([0.; 3], 0.5, true).unwrap();
        assert!(matches!(removed, Field::Difference { .. }));
        assert_eq!(removed.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(removed.evaluate([0.75, 0., 0.]).unwrap() < 0.);
        // Chained strokes compose and the result still polygonizes to a closed shell.
        let strokes = removed
            .sculpt_sphere([0., 1., 0.], 0.3, false)
            .unwrap()
            .sculpt_sphere([0., -1., 0.], 0.3, true)
            .unwrap();
        assert!(strokes.evaluate([0., 1., 0.]).unwrap() < 0.);
        assert!(strokes.evaluate([0., -1., 0.]).unwrap() > 0.);
        let mesh = polygonize(&strokes, &grid(16)).unwrap();
        assert!(triangles_closed(&mesh));
        assert!(triangles_ok(&mesh));
    }
    #[test]
    fn capsule_and_smooth_difference_sample_correctly() {
        let capsule = Field::Capsule {
            a: [0.; 3],
            b: [2., 0., 0.],
            radius: 0.5,
        };
        assert_eq!(capsule.evaluate([1., 0., 0.]).unwrap(), -0.5);
        assert_eq!(capsule.evaluate([3., 0., 0.]).unwrap(), 0.5);
        assert_eq!(capsule.evaluate([1., 1., 0.]).unwrap(), 0.5);
        let degenerate = Field::Capsule {
            a: [0.; 3],
            b: [0.; 3],
            radius: 1.,
        };
        assert_eq!(
            degenerate.evaluate([0., 2., 0.]).unwrap(),
            1.,
            "zero-length capsule is a sphere"
        );
        let hard = Field::Difference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [1., 0., 0.],
                radius: 0.5,
            }),
        };
        let smooth = Field::SmoothDifference {
            a: Box::new(sphere()),
            b: Box::new(Field::Sphere {
                center: [1., 0., 0.],
                radius: 0.5,
            }),
            radius: 0.3,
        };
        // Far from the seam the smooth result equals the hard result; near it, material is removed more.
        assert!(
            (smooth.evaluate([-0.9, 0., 0.]).unwrap() - hard.evaluate([-0.9, 0., 0.]).unwrap())
                .abs()
                < 1e-12
        );
        assert!(
            (smooth.evaluate([1., 0., 0.]).unwrap() - hard.evaluate([1., 0., 0.]).unwrap()).abs()
                < 1e-12
        );
        // On the seam circle both distances vanish; the blend lifts the value by k/4.
        let seam = [0.875, (1. - 0.875f64 * 0.875).sqrt(), 0.];
        assert!(hard.evaluate(seam).unwrap().abs() < 1e-9);
        assert!((smooth.evaluate(seam).unwrap() - 0.3 / 4.).abs() < 1e-9);
        assert!(triangles_closed(&polygonize(&smooth, &grid(16)).unwrap()));
    }
    #[test]
    fn sculpt_strokes_compose_tools_and_blend_modes() {
        let base = sphere();
        let add = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Box {
                    center: [1.2, 0., 0.],
                    half_size: [0.3; 3],
                },
                remove: false,
                blend: None,
            })
            .unwrap();
        assert!(matches!(add, Field::Union { .. }));
        assert!(add.evaluate([1.2, 0., 0.]).unwrap() < 0.);
        let smooth_add = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Capsule {
                    a: [0., 0., 0.8],
                    b: [0., 0., 1.6],
                    radius: 0.2,
                },
                remove: false,
                blend: Some(0.2),
            })
            .unwrap();
        assert!(matches!(smooth_add, Field::SmoothUnion { .. }));
        assert!(smooth_add.evaluate([0., 0., 1.6]).unwrap() < 0.);
        let hard_remove = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 0.5,
                },
                remove: true,
                blend: None,
            })
            .unwrap();
        assert_eq!(hard_remove.evaluate([0.; 3]).unwrap(), 0.5);
        assert!(matches!(hard_remove, Field::Difference { .. }));
        let smooth_remove = base
            .sculpt(&SdfStroke {
                tool: SdfTool::Sphere {
                    center: [1., 0., 0.],
                    radius: 0.5,
                },
                remove: true,
                blend: Some(0.25),
            })
            .unwrap();
        assert!(matches!(smooth_remove, Field::SmoothDifference { .. }));
        assert!(smooth_remove.evaluate([0.6, 0., 0.]).unwrap() > 0.);
        let mesh = polygonize(&smooth_remove, &grid(16)).unwrap();
        assert!(triangles_closed(&mesh) && triangles_ok(&mesh));
        #[cfg(feature = "codec")]
        {
            let round: SdfStroke = value_codec::from_value(
                value_codec::to_value(SdfStroke {
                    tool: SdfTool::Capsule {
                        a: [0.; 3],
                        b: [1., 0., 0.],
                        radius: 0.2,
                    },
                    remove: true,
                    blend: Some(0.1),
                })
                .unwrap(),
            )
            .unwrap();
            assert_eq!(round.blend, Some(0.1));
            assert!(round.remove);
        }
        for bad in [
            SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 0.,
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Box {
                    center: [0.; 3],
                    half_size: [1., -1., 1.],
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Capsule {
                    a: [f64::NAN; 3],
                    b: [0.; 3],
                    radius: 1.,
                },
                remove: false,
                blend: None,
            },
            SdfStroke {
                tool: SdfTool::Sphere {
                    center: [0.; 3],
                    radius: 1.,
                },
                remove: false,
                blend: Some(0.),
            },
        ] {
            assert!(base.sculpt(&bad).is_err());
        }
    }
    #[test]
    fn sculpt_sphere_rejects_invalid_strokes() {
        let base = sphere();
        for radius in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                base.sculpt_sphere([0.; 3], radius, false).is_err(),
                "{radius}"
            );
        }
        assert!(base.sculpt_sphere([f64::NAN, 0., 0.], 1., true).is_err());
        let invalid = Field::Sphere {
            center: [0.; 3],
            radius: -1.,
        };
        assert!(invalid.sculpt_sphere([0.; 3], 1., false).is_err());
    }
    #[test]
    fn closed_outward_sphere_and_convergence() {
        let coarse = polygonize(&sphere(), &grid(8)).unwrap();
        let fine = polygonize(&sphere(), &grid(16)).unwrap();
        assert!(triangles_closed(&fine));
        assert!(triangles_ok(&fine));
        let exact = 4. * std::f64::consts::PI / 3.;
        assert!((signed_volume(&fine) - exact).abs() < (signed_volume(&coarse) - exact).abs());
        assert!((signed_volume(&fine) - exact).abs() < 0.1);
    }
    #[test]
    fn exact_grid_vertices_and_empty() {
        let mut g = grid(12);
        g.min = [-2.; 3];
        g.max = [2.; 3];
        assert!(triangles_closed(&polygonize(&sphere(), &g).unwrap()));
        assert!(polygonize_with(|_| 1., &g).unwrap().indices.is_empty());
        assert!(polygonize_with(|_| f64::NAN, &g).is_err());
        assert!(polygonize_with(|_| -1., &g).is_err());
    }
    #[test]
    fn tpms_chamfer_onion_elongate_and_surface_nets() {
        let neovius = Field::Neovius {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        let lidinoid = Field::Lidinoid {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        let iwp = Field::SchoenIWP {
            origin: [0.; 3],
            period: 4.0,
            thickness: 0.5,
        };
        for tpms in [&neovius, &lidinoid, &iwp] {
            assert!(tpms.validate().is_ok());
            assert!(tpms.evaluate([0.5, 0.7, 1.1]).unwrap().is_finite());
        }

        let onion_sphere = Field::Onion {
            input: Box::new(sphere()),
            thickness: 0.2,
        };
        // On the original sphere surface (r=1), onion field is -0.1 (inside shell); at origin it's +0.9 (outside shell).
        assert!((onion_sphere.evaluate([1., 0., 0.]).unwrap() + 0.1).abs() < 1e-12);
        assert!((onion_sphere.evaluate([0., 0., 0.]).unwrap() - 0.9).abs() < 1e-12);

        let elongated = Field::Elongate {
            input: Box::new(sphere()),
            half_extents: [1.0, 0.0, 0.0],
        };
        // Elongating a unit sphere by 1.0 along +-X produces a capsule of total length 4 along X.
        assert!((elongated.evaluate([2.0, 0., 0.]).unwrap()).abs() < 1e-12);
        assert!(elongated.evaluate([1.5, 0., 0.]).unwrap() < 0.);

        let chamfer = Field::ChamferUnion {
            a: Box::new(sphere()),
            b: Box::new(Field::Box {
                center: [0.8, 0., 0.],
                half_size: [0.6; 3],
            }),
            radius: 0.2,
        };
        assert!(chamfer.validate().is_ok());

        let sn_mesh = polygonize_surface_nets(&chamfer, &grid(16)).unwrap();
        assert!(triangles_closed(&sn_mesh));
        assert!(triangles_ok(&sn_mesh));
    }

    #[test]
    fn octahedron_prisms_mirror_taper_raycast_and_volume() {
        let oct = Field::Octahedron {
            center: [0.; 3],
            size: 1.0,
        };
        assert!(oct.validate().is_ok());
        assert!(oct.evaluate([0.; 3]).unwrap() < 0.0);
        assert!((oct.evaluate([1.0, 0.0, 0.0]).unwrap()).abs() < 1e-12);

        let hex = Field::HexPrism {
            center: [0.; 3],
            radius: 1.0,
            half_height: 0.8,
        };
        assert!(hex.validate().is_ok());
        assert!(hex.evaluate([0.; 3]).unwrap() < 0.0);

        let tri_prism = Field::TriangularPrism {
            center: [0.; 3],
            radius: 1.0,
            half_height: 0.8,
        };
        assert!(tri_prism.validate().is_ok());
        assert!(tri_prism.evaluate([0.; 3]).unwrap() < 0.0);

        let s_inter = Field::SmoothIntersection {
            a: Box::new(sphere()),
            b: Box::new(Field::Box {
                center: [0.; 3],
                half_size: [0.8; 3],
            }),
            radius: 0.2,
        };
        assert!(s_inter.validate().is_ok());
        assert!(s_inter.evaluate([0.; 3]).unwrap() < 0.0);

        let mirrored = Field::Mirror {
            input: Box::new(Field::Sphere {
                center: [1.5, 0.0, 0.0],
                radius: 0.5,
            }),
            axis: [true, false, false],
        };
        assert!((mirrored.evaluate([-1.5, 0.0, 0.0]).unwrap() + 0.5).abs() < 1e-12);

        let tapered = Field::Taper {
            input: Box::new(Field::Cylinder {
                center: [0.; 3],
                radius: 1.0,
                half_height: 1.0,
            }),
            height: 2.0,
            factor: 0.5,
        };
        assert!(tapered.validate().is_ok());
        assert!(tapered.evaluate([0.0, 0.0, 0.8]).unwrap() < 0.0);
        assert!(tapered.evaluate([0.8, 0.0, 0.8]).unwrap() > 0.0);

        // Raycast from (-3, 0, 0) along +X against unit sphere at origin
        let (hit_p, hit_n, dist) = sphere()
            .raycast([-3.0, 0.0, 0.0], [1.0, 0.0, 0.0], 10.0, 1e-5)
            .unwrap();
        assert!((dist - 2.0).abs() < 1e-4);
        assert!((hit_p[0] + 1.0).abs() < 1e-4);
        assert!((hit_n[0] + 1.0).abs() < 1e-3);

        // Volume estimation of unit sphere (~4/3 * pi = 4.18879)
        let vol = sphere().estimate_volume(32);
        let exact = 4.0 * std::f64::consts::PI / 3.0;
        assert!((vol - exact).abs() < 0.15, "estimated {vol} vs exact {exact}");

        // Mean curvature on a sphere of radius R=2.0 is H = 1/R = 0.5
        let r2_sphere = Field::Sphere {
            center: [0.; 3],
            radius: 2.0,
        };
        let h_curv = r2_sphere.mean_curvature([2.0, 0.0, 0.0], 1e-3);
        assert!((h_curv - 0.5).abs() < 1e-3, "mean curvature got {h_curv}");

        // Ambient occlusion: unoccluded sphere surface -> 1.0; occluded by an overhanging box -> < 0.8
        let ao_open = r2_sphere.ambient_occlusion([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 5, 0.2);
        assert!((ao_open - 1.0).abs() < 1e-6, "expected 1.0, got {ao_open}");
        let occluded_scene = Field::Union {
            a: Box::new(r2_sphere),
            b: Box::new(Field::Box {
                center: [2.8, 0.0, 0.0],
                half_size: [0.5, 2.0, 2.0],
            }),
        };
        let ao_occluded = occluded_scene.ambient_occlusion([2.0, 0.0, 0.0], [1.0, 0.0, 0.0], 5, 0.2);
        assert!(ao_occluded < 0.6, "expected occluded AO < 0.6, got {ao_occluded}");

        // Principal curvatures on a sphere of radius R = 2.0: k1 = k2 = H = 0.5, K = 0.25
        let sph2 = Field::Sphere {
            center: [0.; 3],
            radius: 2.0,
        };
        let (k1, k2, gauss_k, mean_h) = sph2.principal_curvatures([2.0, 0.0, 0.0], 1e-3);
        assert!((k1 - 0.5).abs() < 1e-3, "expected k1=0.5, got {k1}");
        assert!((k2 - 0.5).abs() < 1e-3, "expected k2=0.5, got {k2}");
        assert!((gauss_k - 0.25).abs() < 1e-3, "expected K=0.25, got {gauss_k}");
        assert!((mean_h - 0.5).abs() < 1e-3, "expected H=0.5, got {mean_h}");

        // Principal curvatures on a cylinder of radius R = 2.0: k1 = 0.5, k2 = 0.0, K = 0.0, H = 0.25
        let cyl2 = Field::Cylinder {
            center: [0.; 3],
            radius: 2.0,
            half_height: 5.0,
        };
        let (ck1, ck2, c_gauss_k, c_mean_h) = cyl2.principal_curvatures([2.0, 0.0, 0.0], 1e-3);
        assert!((ck1 - 0.5).abs() < 1e-3, "expected ck1=0.5, got {ck1}");
        assert!(ck2.abs() < 1e-3, "expected ck2=0.0, got {ck2}");
        assert!(c_gauss_k.abs() < 1e-3, "expected cK=0.0, got {c_gauss_k}");
        assert!((c_mean_h - 0.25).abs() < 1e-3, "expected cH=0.25, got {c_mean_h}");

        // Closest surface point projection onto a sphere of radius 2.0 from query [5.0, 0.0, 0.0]
        let proj = sph2.closest_surface_point([5.0, 0.0, 0.0], 16, 1e-6);
        assert!((proj[0] - 2.0).abs() < 1e-5, "expected x=2.0, got {:?}", proj);
        assert!(proj[1].abs() < 1e-5 && proj[2].abs() < 1e-5);
    }
}

