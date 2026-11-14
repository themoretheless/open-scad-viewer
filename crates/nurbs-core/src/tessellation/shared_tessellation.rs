//! Common indexed seam vertices on full rectangular NURBS charts.
use crate::{
    Result, check,
    curve::Curve,
    curve_surface_agreement::{self, Report as Agreement, Status},
    distance_bounds::{Interval as I, box_distance},
    foundation::guards::require_finite_f64,
    surface::{Axis, Surface},
    surface_distance,
};
#[derive(Clone, Copy, Debug)]
pub enum Side {
    U0,
    U1,
    V0,
    V1,
}
#[derive(Clone, Copy, Debug)]
pub struct Seam {
    pub patches: [usize; 2],
    pub sides: [Side; 2],
    pub reversed: bool,
}
#[derive(Clone, Debug)]
pub struct Triangle {
    pub patch: usize,
    pub vertices: [usize; 3],
    pub error_upper: f64,
}
#[derive(Clone, Debug)]
pub struct Mesh {
    pub points: Vec<[f64; 3]>,
    pub triangles: Vec<Triangle>,
    /// One representative chart `(patch, [u, v])` per point; welded seam
    /// points keep the chart of their lowest grid index.
    pub charts: Vec<(usize, [f64; 2])>,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub mesh: Option<Mesh>,
    pub agreements: Vec<Agreement>,
    pub agreement_cells: usize,
    pub cells: usize,
    pub error_upper: f64,
    pub within_tolerance: bool,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn edge(s: &Surface, side: Side) -> Result<(Curve, Curve)> {
    let d = domain(s);
    let (axis, fixed, a, b) = match side {
        Side::U0 => (
            Axis::U,
            d[0][0],
            vec![d[0][0], d[1][0]],
            vec![d[0][0], d[1][1]],
        ),
        Side::U1 => (
            Axis::U,
            d[0][1],
            vec![d[0][1], d[1][0]],
            vec![d[0][1], d[1][1]],
        ),
        Side::V0 => (
            Axis::V,
            d[1][0],
            vec![d[0][0], d[1][0]],
            vec![d[0][1], d[1][0]],
        ),
        Side::V1 => (
            Axis::V,
            d[1][1],
            vec![d[0][0], d[1][1]],
            vec![d[0][1], d[1][1]],
        ),
    };
    Ok((s.iso(axis, fixed)?, Curve::from_polyline(vec![a, b])?))
}
fn index(patch: usize, side: Side, k: usize, n: usize) -> usize {
    let (u, v) = match side {
        Side::U0 => (0, k),
        Side::U1 => (n, k),
        Side::V0 => (k, 0),
        Side::V1 => (k, n),
    };
    patch * (n + 1) * (n + 1) + u * (n + 1) + v
}
fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}
/// Second-order deviation data for a grid cell lying inside one knot span.
///
/// For a parameter triangle with vertices `p_i` and a point `x` with
/// barycentric weights `λ_i`, Taylor's formula with integral remainder gives
/// `S(x) - Σ λ_i S(p_i) = -Σ λ_i ∫₀¹ (1-t) D²S(x + t(p_i-x))[d_i, d_i] dt`
/// with `d_i = p_i - x` (the first-order terms cancel since `Σ λ_i d_i = 0`).
/// In the cell's unit coordinates `|d_i|` is at most 1 per axis, so
/// `|S(x) - flat(x)| <= (|S_uu| + 2|S_uv| + |S_vv|) / 2` using interval jet
/// norms of the original rational surface over the cell. The welded vertex
/// may differ from `S(p_i)`; that displacement (per cell corner, in grid order
/// `(i,j), (i+1,j), (i+1,j+1), (i,j+1)`) is added by the caller. The bound
/// shrinks with the square of the grid step. `None` when the cell crosses a
/// knot or the jets fail; the box bound then stands alone.
fn taylor_cell(s: &Surface, domain: [[f64; 2]; 2], vertices: [[f64; 3]; 4]) -> Option<(f64, [f64; 4])> {
    use crate::analysis::surface_measure::jets;
    use crate::numerics::interval_vec3::norm;
    let find = |knots: &[f64], degree: usize, count: usize, [a, b]: [f64; 2]| {
        (degree..count).find(|&k| knots[k] < knots[k + 1] && knots[k] <= a && b <= knots[k + 1])
    };
    let span = [
        find(&s.knots_u, s.degree_u, s.control_points.len(), domain[0])?,
        find(&s.knots_v, s.degree_v, s.control_points[0].len(), domain[1])?,
    ];
    let j = jets::calculate(s, span, domain, None).ok()?;
    let m = |v| norm(v).ok().map(|n| n.hi);
    let curvature = ((m(j[2][0])? + 2. * m(j[1][1])? + m(j[0][2])?) / 2.).next_up();
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    let mut displacement = [0.; 4];
    for (k, corner) in [[0, 0], [1, 0], [1, 1], [0, 1]].into_iter().enumerate() {
        let c = jets::calculate(s, span, domain, Some(corner)).ok()?;
        let exact: Vec<I> = (0..3).map(|a| c[0][0][a].add(I::point(origin[a]))).collect::<Result<_>>().ok()?;
        let vertex: Vec<I> = vertices[k].iter().map(|&x| I::point(x)).collect();
        displacement[k] = box_distance(&exact, &vertex).ok()?.1;
    }
    (curvature.is_finite() && displacement.iter().all(|d| d.is_finite())).then_some((curvature, displacement))
}

/// Uniform common normalized strips guarantee no T-junction on declared seams.
/// Seams use explicit affine/reversed chart correspondence. Full original
/// edge/lift agreement precedes any welding. Every output triangle gets a
/// continuous outward deviation bound against its original source cell,
/// including the displacement from canonical shared vertices. Bounds cover
/// corresponding positions and symmetric Hausdorff distance; not injectivity,
/// triangle orientation or trimmed faces. Source surfaces remain unchanged.
pub fn tessellate(
    surfaces: &[Surface],
    seams: &[Seam],
    segments: usize,
    tolerance: f64,
    max_cells: usize,
    max_agreement_cells: usize,
) -> Result<Report> {
    require_finite_f64(tolerance, "shared tessellation tolerance")?;
    check(
        (1..=16).contains(&surfaces.len())
            && seams.len() <= 64
            && (1..=128).contains(&segments)
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells)
            && (1..=1000000).contains(&max_agreement_cells),
        "Shared tessellation requires bounded patch/grid/work counts and positive tolerance",
    )?;
    for s in surfaces {
        s.validate()?;
    }
    check(
        surfaces.len() * segments * segments <= max_cells,
        "Shared grid exceeds cell budget",
    )?;
    let mut report = Report {
        mesh: None,
        agreements: Vec::with_capacity(seams.len() * 2),
        agreement_cells: 0,
        cells: 0,
        error_upper: 0.,
        within_tolerance: false,
    };
    for seam in seams {
        check(
            seam.patches[0] != seam.patches[1] && seam.patches.iter().all(|p| *p < surfaces.len()),
            "Seam patch addresses must be distinct and valid",
        )?;
        let (c, source_pcurve) = edge(&surfaces[seam.patches[0]], seam.sides[0])?;
        let (_, target_pcurve) = edge(&surfaces[seam.patches[1]], seam.sides[1])?;
        for (p, surface, reversed) in [
            (&source_pcurve, &surfaces[seam.patches[0]], false),
            (&target_pcurve, &surfaces[seam.patches[1]], seam.reversed),
        ] {
            if report.agreement_cells == max_agreement_cells {
                return Ok(report);
            }
            let proof = curve_surface_agreement::verify(
                &c,
                p,
                surface,
                reversed,
                tolerance,
                (max_agreement_cells - report.agreement_cells).min(100000),
            )?;
            report.agreement_cells += proof.cells;
            let admitted = proof.status == Status::WithinTolerance;
            report.agreements.push(proof);
            if !admitted {
                return Ok(report);
            }
        }
    }
    let n = segments;
    let mut points: Vec<[f64; 3]> = Vec::new();
    let mut grids = Vec::new();
    for s in surfaces {
        let d = domain(s);
        let grid: [Vec<f64>; 2] = std::array::from_fn(|axis| {
            (0..=n)
                .map(|i| {
                    if i == 0 {
                        d[axis][0]
                    } else if i == n {
                        d[axis][1]
                    } else {
                        d[axis][0] + (d[axis][1] - d[axis][0]) * i as f64 / n as f64
                    }
                })
                .collect()
        });
        check(
            grid.iter().all(|g| g.windows(2).all(|p| p[0] < p[1])),
            "Shared grid reached precision limit",
        )?;
        for &u in &grid[0] {
            for &v in &grid[1] {
                points.push(s.evaluate(u, v)?.point.try_into().unwrap());
            }
        }
        grids.push(grid);
    }
    let mut parent: Vec<_> = (0..points.len()).collect();
    for seam in seams {
        for k in 0..=n {
            let a = root(&parent, index(seam.patches[0], seam.sides[0], k, n));
            let b = root(
                &parent,
                index(
                    seam.patches[1],
                    seam.sides[1],
                    if seam.reversed { n - k } else { k },
                    n,
                ),
            );
            parent[a.max(b)] = a.min(b);
        }
    }
    let map: Vec<_> = (0..points.len()).map(|i| root(&parent, i)).collect();
    let mut remap = vec![0; points.len()];
    let mut compact = Vec::with_capacity(points.len());
    let mut charts = Vec::with_capacity(points.len());
    let per_patch = (n + 1) * (n + 1);
    for i in 0..points.len() {
        if map[i] == i {
            remap[i] = compact.len();
            compact.push(points[i]);
            let (patch, local) = (i / per_patch, i % per_patch);
            let g = &grids[patch];
            charts.push((patch, [g[0][local / (n + 1)], g[1][local % (n + 1)]]));
        }
    }
    let mut triangles = Vec::with_capacity(surfaces.len() * n * n * 2);
    for (patch, s) in surfaces.iter().enumerate() {
        for i in 0..n {
            for j in 0..n {
                let base = patch * (n + 1) * (n + 1) + i * (n + 1) + j;
                let ids = [base, base + n + 1, base + n + 2, base + 1].map(|v| remap[map[v]]);
                let bounds = surface_distance::rectangle_bounds(
                    s,
                    [
                        [grids[patch][0][i], grids[patch][0][i + 1]],
                        [grids[patch][1][j], grids[patch][1][j + 1]],
                    ],
                )?;
                let image = bounds
                    .iter()
                    .map(|b| I::new(b[0], b[1]))
                    .collect::<Result<Vec<_>>>()?;
                let taylor = taylor_cell(
                    s,
                    [
                        [grids[patch][0][i], grids[patch][0][i + 1]],
                        [grids[patch][1][j], grids[patch][1][j + 1]],
                    ],
                    ids.map(|v| compact[v]),
                );
                for (corners, vertices) in [([0, 1, 2], [ids[0], ids[1], ids[2]]), ([0, 2, 3], [ids[0], ids[2], ids[3]])] {
                    let represented = (0..3)
                        .map(|axis| {
                            I::new(
                                vertices
                                    .iter()
                                    .map(|&v| compact[v][axis])
                                    .fold(f64::INFINITY, f64::min),
                                vertices
                                    .iter()
                                    .map(|&v| compact[v][axis])
                                    .fold(f64::NEG_INFINITY, f64::max),
                            )
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let mut error_upper = box_distance(&image, &represented)?.1;
                    if let Some((curvature, displacement)) = taylor {
                        let shift = corners.iter().map(|&c| displacement[c]).fold(0., f64::max);
                        error_upper = error_upper.min((curvature + shift).next_up());
                    }
                    report.error_upper = report.error_upper.max(error_upper);
                    triangles.push(Triangle {
                        patch,
                        vertices,
                        error_upper,
                    });
                }
                report.cells += 1;
            }
        }
    }
    report.within_tolerance = report.error_upper <= tolerance;
    report.mesh = Some(Mesh {
        points: compact,
        triangles,
        charts,
    });
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rational biquadratic dome; `split` adds an interior knot in u at 0.3.
    fn dome(split: bool) -> Surface {
        let mut control_points: Vec<Vec<Vec<f64>>> = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 / 2., j as f64 / 2., if i == 1 && j == 1 { 0.8 } else { 0.1 * i as f64 }])
                    .collect()
            })
            .collect();
        let mut weights = vec![vec![1., 0.8, 1.], vec![0.9, 1.3, 0.9], vec![1., 0.8, 1.]];
        let mut knots_u = vec![0., 0., 0., 1., 1., 1.];
        if split {
            control_points.insert(2, (0..3).map(|j| vec![0.75, j as f64 / 2., 0.3]).collect());
            weights.insert(2, vec![1.; 3]);
            knots_u = vec![0., 0., 0., 0.3, 1., 1., 1.];
        }
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u,
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points,
            weights,
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Largest sampled distance between surface points and the flat triangle
    /// at the same barycentric coordinates, per triangle.
    fn sampled(s: &Surface, mesh: &Mesh) -> Vec<f64> {
        mesh.triangles
            .iter()
            .map(|t| {
                let uv = t.vertices.map(|v| mesh.charts[v].1);
                let xyz = t.vertices.map(|v| mesh.points[v]);
                let mut worst: f64 = 0.;
                for a in 0..=8 {
                    for b in 0..=8 - a {
                        let l = [a as f64 / 8., b as f64 / 8., (8 - a - b) as f64 / 8.];
                        let p: [f64; 2] = std::array::from_fn(|k| (0..3).map(|i| l[i] * uv[i][k]).sum());
                        let flat: [f64; 3] = std::array::from_fn(|k| (0..3).map(|i| l[i] * xyz[i][k]).sum());
                        let e = s.evaluate(p[0], p[1]).unwrap().point;
                        worst = worst.max((0..3).map(|k| (e[k] - flat[k]).powi(2)).sum::<f64>().sqrt());
                    }
                }
                worst
            })
            .collect()
    }

    #[test]
    fn deviation_bounds_converge_quadratically_and_stay_conservative() {
        let s = dome(false);
        let mut previous: Option<f64> = None;
        for n in [8, 16, 32, 64] {
            let r = tessellate(std::slice::from_ref(&s), &[], n, 1., 100_000, 1).unwrap();
            let mesh = r.mesh.as_ref().unwrap();
            for (t, d) in mesh.triangles.iter().zip(sampled(&s, mesh)) {
                assert!(d <= t.error_upper, "n={n}: sampled {d} > bound {}", t.error_upper);
            }
            if let Some(p) = previous {
                // Box enclosures only halve per refinement; the curvature bound quarters.
                let ratio = p / r.error_upper;
                assert!(ratio > 3.5, "n={n}: ratio {ratio}");
            }
            previous = Some(r.error_upper);
        }
    }

    #[test]
    fn cells_across_knots_keep_a_conservative_bound() {
        let s = dome(true);
        for n in [2, 3, 10] {
            let r = tessellate(std::slice::from_ref(&s), &[], n, 1., 100_000, 1).unwrap();
            let mesh = r.mesh.as_ref().unwrap();
            for (t, d) in mesh.triangles.iter().zip(sampled(&s, mesh)) {
                assert!(d <= t.error_upper, "n={n}: sampled {d} > bound {}", t.error_upper);
            }
        }
    }
}
