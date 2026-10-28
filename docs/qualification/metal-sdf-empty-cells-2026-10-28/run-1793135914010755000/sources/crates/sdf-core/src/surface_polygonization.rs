//! Surface polygonization: preserves the original arithmetic and proof budgets.
use super::*;

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
