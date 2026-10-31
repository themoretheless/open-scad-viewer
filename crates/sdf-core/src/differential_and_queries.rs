//! Differential SDF queries (tetrahedral gradient, mean & principal curvatures, ambient occlusion,
//! closest surface point projection, sphere-tracing raycast, Lipschitz-accelerated volume estimation)
//! and Dual Surface Nets / QEF isosurface extraction (`polygonize_surface_nets`).

use geometry_ops::Triangles;
use math_core::{cross, dot, finite, norm as length, sub};
use crate::{Field, Grid, Point, Result, check_grid_budget, error, field_eval::finish_triangles};



/// Dual Surface Nets / Dual Contouring extraction with gradient-guided QEF vertex projection.
/// Places one dual vertex per active grid cell at the mass point of edge crossings and projects
/// it onto the tangent planes defined by the SDF gradients $\nabla f$, then emits two outward
/// triangles per sign-changing interior grid edge.
pub fn polygonize_surface_nets(field: &Field, grid: &Grid) -> Result<Triangles> {
    check_grid_budget(field, grid)?;
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
    let snap = 32.0
        * f64::EPSILON
        * (0..3)
            .map(|i| grid.max[i] - grid.min[i])
            .fold(0.0, f64::max);
    for z in 0..=nz {
        for y in 0..=ny {
            for x in 0..=nx {
                let p = std::array::from_fn(|i| {
                    grid.min[i]
                        + (grid.max[i] - grid.min[i]) * [x, y, z][i] as f64 / grid.cells[i] as f64
                });
                let v = field.sample(p);
                let v = if v.abs() <= snap { 0.0 } else { v };
                if !v.is_finite() {
                    return Err(error("Field returned a non-finite value"));
                }
                if (x == 0 || y == 0 || z == 0 || x == nx || y == ny || z == nz) && v <= 0.0 {
                    return Err(error("Surface touches grid boundary; enlarge bounds"));
                }
                points.push(p);
                values.push(v);
            }
        }
    }

    let cell_idx = |cx: usize, cy: usize, cz: usize| (cz * ny + cy) * nx + cx;
    let mut cell_vert = vec![usize::MAX; nx * ny * nz];
    let cell_estimate = (nx * ny + ny * nz + nz * nx) * 2;
    let mut positions = Vec::with_capacity(cell_estimate * 3);
    let step = [
        (grid.max[0] - grid.min[0]) / nx as f64,
        (grid.max[1] - grid.min[1]) / ny as f64,
        (grid.max[2] - grid.min[2]) / nz as f64,
    ];
    let grad_eps = step[0].min(step[1]).min(step[2]) * 0.05;

    const CUBE_EDGES: [[usize; 2]; 12] = [
        [0, 1],
        [1, 2],
        [2, 3],
        [3, 0],
        [4, 5],
        [5, 6],
        [6, 7],
        [7, 4],
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7],
    ];

    for cz in 0..nz {
        for cy in 0..ny {
            for cx in 0..nx {
                let c = [
                    index(cx, cy, cz),
                    index(cx + 1, cy, cz),
                    index(cx + 1, cy + 1, cz),
                    index(cx, cy + 1, cz),
                    index(cx, cy, cz + 1),
                    index(cx + 1, cy, cz + 1),
                    index(cx + 1, cy + 1, cz + 1),
                    index(cx, cy + 1, cz + 1),
                ];
                let mut crossings_buf = [[0.0; 3]; 12];
                let mut cross_len = 0usize;
                for &[ea, eb] in &CUBE_EDGES {
                    let va = values[c[ea]];
                    let vb = values[c[eb]];
                    if (va < 0.0) != (vb < 0.0) {
                        let t = (va / (va - vb)).clamp(0.0, 1.0);
                        let pa = points[c[ea]];
                        let pb = points[c[eb]];
                        crossings_buf[cross_len] = [
                            pa[0] + t * (pb[0] - pa[0]),
                            pa[1] + t * (pb[1] - pa[1]),
                            pa[2] + t * (pb[2] - pa[2]),
                        ];
                        cross_len += 1;
                    }
                }
                if cross_len == 0 {
                    continue;
                }
                let crossings = &crossings_buf[..cross_len];
                let inv_len = 1.0 / cross_len as f64;
                let mut v_pos = [0.0; 3];
                for p in crossings {
                    for k in 0..3 {
                        v_pos[k] += p[k] * inv_len;
                    }
                }
                // Gradient-descent / QEF tangent-plane projection clamped strictly inside cell bounds
                let c_min = points[c[0]];
                let c_max = points[c[6]];
                for _ in 0..2 {
                    let mut disp = [0.0; 3];
                    for p in crossings {
                        let n = field.gradient(*p, grad_eps);
                        let d = dot(sub(*p, v_pos), n);
                        for k in 0..3 {
                            disp[k] += d * n[k] * inv_len;
                        }
                    }
                    for k in 0..3 {
                        let margin = step[k] * 0.05;
                        v_pos[k] = (v_pos[k] + 0.5 * disp[k])
                            .clamp(c_min[k] + margin, c_max[k] - margin);
                    }
                }
                let vid = positions.len() / 3;
                positions.extend_from_slice(&v_pos);
                cell_vert[cell_idx(cx, cy, cz)] = vid;
            }
        }
    }

    let mut indices = Vec::with_capacity(positions.len() * 2);
    let mut emit_quad = |v0: usize, v1: usize, v2: usize, v3: usize, flip: bool| {
        if flip {
            indices.extend_from_slice(&[v0, v2, v1, v0, v3, v2]);
        } else {
            indices.extend_from_slice(&[v0, v1, v2, v0, v2, v3]);
        }
    };

    // X-parallel edges: shared by cells (x, y-1, z-1), (x, y, z-1), (x, y, z), (x, y-1, z)
    for z in 1..nz {
        for y in 1..ny {
            for x in 0..nx {
                let va = values[index(x, y, z)];
                let vb = values[index(x + 1, y, z)];
                if (va < 0.0) != (vb < 0.0) {
                    let c0 = cell_vert[cell_idx(x, y - 1, z - 1)];
                    let c1 = cell_vert[cell_idx(x, y, z - 1)];
                    let c2 = cell_vert[cell_idx(x, y, z)];
                    let c3 = cell_vert[cell_idx(x, y - 1, z)];
                    emit_quad(c0, c1, c2, c3, va >= 0.0);
                }
            }
        }
    }
    // Y-parallel edges: shared by cells (x-1, y, z-1), (x-1, y, z), (x, y, z), (x, y, z-1)
    for z in 1..nz {
        for y in 0..ny {
            for x in 1..nx {
                let va = values[index(x, y, z)];
                let vb = values[index(x, y + 1, z)];
                if (va < 0.0) != (vb < 0.0) {
                    let c0 = cell_vert[cell_idx(x - 1, y, z - 1)];
                    let c1 = cell_vert[cell_idx(x - 1, y, z)];
                    let c2 = cell_vert[cell_idx(x, y, z)];
                    let c3 = cell_vert[cell_idx(x, y, z - 1)];
                    emit_quad(c0, c1, c2, c3, va >= 0.0);
                }
            }
        }
    }
    // Z-parallel edges: shared by cells (x-1, y-1, z), (x, y-1, z), (x, y, z), (x-1, y, z)
    for z in 0..nz {
        for y in 1..ny {
            for x in 1..nx {
                let va = values[index(x, y, z)];
                let vb = values[index(x, y, z + 1)];
                if (va < 0.0) != (vb < 0.0) {
                    let c0 = cell_vert[cell_idx(x - 1, y - 1, z)];
                    let c1 = cell_vert[cell_idx(x, y - 1, z)];
                    let c2 = cell_vert[cell_idx(x, y, z)];
                    let c3 = cell_vert[cell_idx(x - 1, y, z)];
                    emit_quad(c0, c1, c2, c3, va >= 0.0);
                }
            }
        }
    }

    finish_triangles(Triangles { positions, indices })
}

/// Evaluates a Triply Periodic Minimal Surface (TPMS) Gyroid sheet implicit field at point `p`
/// with spatial period `period_mm` ($L$) and wall thickness `thickness_mm` ($t$):
/// $$f(\mathbf{p}) = \frac{|\sin(kx)\cos(ky) + \sin(ky)\cos(kz) + \sin(kz)\cos(kx)|}{k} - \frac{t}{2}, \quad k = \frac{2\pi}{L}$$
#[inline]
pub fn evaluate_tpms_gyroid(p: Point, period_mm: f64, thickness_mm: f64) -> f64 {
    let period = period_mm.abs().max(1e-6);
    let k = std::f64::consts::TAU / period;
    let (sx, cx) = (p[0] * k).sin_cos();
    let (sy, cy) = (p[1] * k).sin_cos();
    let (sz, cz) = (p[2] * k).sin_cos();
    let iso = sx * cy + sy * cz + sz * cx;
    iso.abs() / k - 0.5 * thickness_mm.max(0.0)
}

/// Evaluates a Triply Periodic Minimal Surface (TPMS) Schwarz-P sheet implicit field at point `p`
/// with spatial period `period_mm` ($L$) and wall thickness `thickness_mm` ($t$):
/// $$f(\mathbf{p}) = \frac{|\cos(kx) + \cos(ky) + \cos(kz)|}{k} - \frac{t}{2}, \quad k = \frac{2\pi}{L}$$
#[inline]
pub fn evaluate_tpms_schwarz_p(p: Point, period_mm: f64, thickness_mm: f64) -> f64 {
    let period = period_mm.abs().max(1e-6);
    let k = std::f64::consts::TAU / period;
    let iso = (p[0] * k).cos() + (p[1] * k).cos() + (p[2] * k).cos();
    iso.abs() / k - 0.5 * thickness_mm.max(0.0)
}


#[path="differential_and_queries/methods_Field_gradient_tetrahedral.rs"]
mod methods_field_gradient_tetrahedral;
