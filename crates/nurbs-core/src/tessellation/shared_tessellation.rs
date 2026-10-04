//! Common indexed seam vertices on full rectangular NURBS charts.
use crate::{
    Result, check,
    curve::Curve,
    curve_surface_agreement::{self, Report as Agreement, Status},
    distance_bounds::{Interval as I, box_distance},
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
        agreements: vec![],
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
    let mut compact = Vec::new();
    for i in 0..points.len() {
        if map[i] == i {
            remap[i] = compact.len();
            compact.push(points[i]);
        }
    }
    let mut triangles = Vec::new();
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
                for vertices in [[ids[0], ids[1], ids[2]], [ids[0], ids[2], ids[3]]] {
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
                    let error_upper = box_distance(&image, &represented)?.1;
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
    });
    Ok(report)
}
